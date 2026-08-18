use crate::database::{self, AppInitProgress, Database, DbState};
use crate::models::DatabaseConnection;
use anyhow::Result;
use log::{debug, info};
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

/// 获取数据库初始化标志文件路径
fn get_init_flag_path() -> Result<PathBuf, String> {
    let data_dir = crate::utils::local::get_app_data_dir()?;
    if !data_dir.exists() {
        fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    }
    Ok(data_dir.join(".db_initialized"))
}

/// 获取本地数据库版本文件路径
fn get_local_db_version_file_path() -> Result<PathBuf, String> {
    let data_dir = crate::utils::local::get_app_data_dir()?;
    if !data_dir.exists() {
        fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    }
    Ok(data_dir.join(".db_version"))
}

/// 将简写版本补齐为可解析的三段语义版本。
///
/// # 参数
/// - `v`：原始版本文本。
fn normalize_version(v: &str) -> String {
    let trimmed = v.trim();
    let parts: Vec<&str> = trimmed.split('.').collect();
    match parts.len() {
        0 => "0.0.0".to_string(),
        1 => format!("{}.0.0", trimmed),
        2 => format!("{}.0", trimmed),
        _ => trimmed.to_string(),
    }
}

fn parse_version(v: &str) -> Result<Version, String> {
    Version::parse(&normalize_version(v)).map_err(|error| format!("数据库版本无效 ({v}): {error}"))
}

fn is_same_version(a: &str, b: &str) -> Result<bool, String> {
    Ok(parse_version(a)? == parse_version(b)?)
}

/// 比较数据库版本与最新迁移版本，判断是否需要升级。
///
/// # 参数
/// - `current_db_version`：当前数据库版本。
/// - `latest_migration_version`：最新内置迁移版本。
fn should_run_migration(
    current_db_version: &str,
    latest_migration_version: &str,
) -> Result<bool, String> {
    Ok(parse_version(current_db_version)? < parse_version(latest_migration_version)?)
}

fn latest_migration_version() -> Option<&'static str> {
    crate::database::migrations::MIGRATIONS
        .last()
        .map(|migration| migration.version)
}

/// 读取本地数据库版本记录；文件不存在或为空时返回空值。
///
/// # 参数
/// - `path`：目标文件或目录路径。
fn read_local_db_version(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let version = content.trim().to_string();
    if version.is_empty() {
        return Ok(None);
    }

    Ok(Some(version))
}

/// 原子流程完成后写入本地数据库版本记录。
///
/// # 参数
/// - `path`：目标文件或目录路径。
/// - `version`：迁移完成后的数据库版本。
fn write_local_db_version(path: &Path, version: &str) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(path, format!("{}\n", version.trim())).map_err(|e| e.to_string())
}

fn emit_progress(app: &AppHandle, message: &str, progress: f32) {
    let payload = AppInitProgress {
        message: message.to_string(),
        progress,
    };
    let _ = app.emit("init-progress", payload);
}

/// 将数据库标记为已初始化
pub fn mark_as_initialized() -> Result<(), String> {
    let path = get_init_flag_path()?;
    fs::write(&path, b"initialized").map_err(|e| e.to_string())?;
    debug!("已创建数据库初始化标志文件: {:?}", path);
    Ok(())
}

pub trait DatabaseInitHandler: Send + Sync {
    /// 根据连接配置创建数据库实现。
    fn init_db(
        &self,
        config: &DatabaseConnection,
    ) -> impl std::future::Future<Output = anyhow::Result<Box<dyn Database>>> + Send;
    /// 将数据库升级到当前应用支持的最新版本。
    fn migrate_up(
        &self,
        db: &dyn Database,
        current_version: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    /// 写入本地数据库初始化完成标记。
    fn mark_initialized(&self) -> Result<(), String>;
}

struct DefaultInitHandler {
    app: AppHandle,
    gateway: Option<crate::models::CloudflareGatewayConfig>,
}

impl DatabaseInitHandler for DefaultInitHandler {
    fn init_db(
        &self,
        config: &DatabaseConnection,
    ) -> impl std::future::Future<Output = anyhow::Result<Box<dyn Database>>> + Send {
        let app = self.app.clone();
        let config = config.clone();
        let gateway = self.gateway.clone();
        async move { database::init(&app, &config, gateway.as_ref()).await }
    }
    fn migrate_up(
        &self,
        db: &dyn Database,
        current_version: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        database::migrate_up_from_version(db, current_version, None)
    }
    fn mark_initialized(&self) -> Result<(), String> {
        mark_as_initialized()
    }
}

/// 初始化数据库并根据标志执行迁移。返回布尔值表示是否执行了新的迁移。
pub async fn init_database(app: &AppHandle) -> Result<bool, String> {
    let db_state = app.state::<DbState>();
    let _init_guard = db_state.init_lock.lock().await;
    let config = {
        use crate::services::config::ConfigState;
        let state = app.state::<ConfigState>();
        let config = state.0.read().unwrap();
        config.clone()
    };
    let config_version = config.version;
    let gateway = config.cloudflare_gateway.clone();
    let db_config = match config.database {
        Some(db_config) => db_config,
        None => {
            debug!("未配置数据库，跳过初始化");
            emit_progress(app, "db.init.done", 1.0);
            return Ok(false);
        }
    };
    if matches!(db_config, DatabaseConnection::CloudflareGateway { .. }) && gateway.is_none() {
        debug!("Cloudflare 网关尚未配置，跳过数据库初始化");
        emit_progress(app, "db.init.done", 1.0);
        return Ok(false);
    }

    if db_state.connection.read().await.as_ref() == Some(&db_config)
        && *db_state.config_version.read().await == Some(config_version)
        && db_state.db.read().await.is_some()
    {
        debug!("数据库已使用相同配置初始化，跳过重复初始化");
        emit_progress(app, "db.init.done", 1.0);
        return Ok(false);
    }

    let init_flag_path = get_init_flag_path()?;
    let local_db_version_file_path = get_local_db_version_file_path()?;
    let latest_version = latest_migration_version();
    let handler = DefaultInitHandler {
        app: app.clone(),
        gateway,
    };

    let (migrated, db) = init_database_internal(
        &db_config,
        &init_flag_path,
        &local_db_version_file_path,
        latest_version,
        &handler,
        |message, progress| emit_progress(app, message, progress),
    )
    .await?;

    // 将数据库句柄存入 DbState 以便全局使用
    let mut db_guard = db_state.db.write().await;
    *db_guard = Some(std::sync::Arc::from(db));
    drop(db_guard);
    *db_state.connection.write().await = Some(db_config);
    *db_state.config_version.write().await = Some(config_version);
    app.state::<crate::commands::books::BookCacheState>()
        .cache
        .invalidate_all();

    Ok(migrated)
}

/// 执行可注入的数据库初始化、版本比较和迁移流程。
///
/// # 参数
/// - `db_config`：待初始化的数据库配置。
/// - `init_flag_path`：数据库初始化完成标记路径。
/// - `local_db_version_file_path`：本地数据库版本记录路径。
/// - `latest_migration_version`：当前应用内置的最新迁移版本。
/// - `handler`：可注入的数据库初始化处理器。
/// - `report_progress`：数据库初始化进度回调。
pub async fn init_database_internal<H: DatabaseInitHandler + ?Sized>(
    db_config: &DatabaseConnection,
    init_flag_path: &Path,
    local_db_version_file_path: &Path,
    latest_migration_version: Option<&str>,
    handler: &H,
    mut report_progress: impl FnMut(&str, f32),
) -> Result<(bool, Box<dyn Database>), String> {
    // 初始化数据库连接
    let db = handler
        .init_db(db_config)
        .await
        .map_err(|e| e.to_string())?;

    let Some(latest_version) = latest_migration_version
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        info!("未找到可用迁移版本，跳过数据库迁移");
        report_progress("db.init.done", 1.0);
        return Ok((false, db));
    };

    report_progress("db.init.checkLatestVersion", 0.05);
    report_progress("db.init.readLocalVersion", 0.15);
    let local_version = read_local_db_version(local_db_version_file_path)?;

    if let Some(local_version) = local_version.as_deref()
        && is_same_version(local_version, latest_version)?
    {
        debug!(
            "本地版本文件已是最新版本 {}，继续核对数据库实际版本",
            latest_version
        );
    }

    report_progress("db.init.readDbVersion", 0.3);
    let current_db_version = db.get_version().await.map_err(|e| e.to_string())?;

    if !should_run_migration(&current_db_version, latest_version)? {
        if !is_same_version(&current_db_version, latest_version)? {
            info!(
                "数据库版本 {} 高于程序最新迁移版本 {}，跳过迁移并同步本地版本文件",
                current_db_version, latest_version
            );
        } else {
            debug!("数据库版本已是最新版本 {}", latest_version);
        }

        report_progress("db.init.writeLocalVersion", 0.8);
        write_local_db_version(local_db_version_file_path, latest_version)?;
        if !init_flag_path.exists() {
            handler.mark_initialized()?;
        }
        report_progress("db.init.done", 1.0);
        return Ok((false, db));
    }

    info!(
        "检测到数据库版本需要升级 (db: {}, latest: {})",
        current_db_version, latest_version
    );
    report_progress("db.init.migrating", 0.55);
    handler
        .migrate_up(db.as_ref(), &current_db_version)
        .await
        .map_err(|e| e.to_string())?;

    report_progress("db.init.writeLocalVersion", 0.8);
    write_local_db_version(local_db_version_file_path, latest_version)?;
    handler.mark_initialized()?;
    info!(
        "数据库迁移完成，当前版本已同步至本地文件 {}",
        latest_version
    );

    report_progress("db.init.done", 1.0);
    Ok((true, db))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::Value;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockDb {
        version: String,
        get_version_calls: Arc<AtomicUsize>,
    }

    impl Database for MockDb {
        fn execute(
            &self,
            _sql: String,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async { Ok(()) })
        }
        fn query(
            &self,
            _sql: String,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Value>>> + Send + '_>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn query_write_batch(
            &self,
            _statements: Vec<crate::database::SqlStatement>,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Vec<Value>>>> + Send + '_>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn get_version(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + '_>> {
            let calls = Arc::clone(&self.get_version_calls);
            let version = self.version.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(version)
            })
        }
    }

    struct MockHandler {
        flag_path: std::path::PathBuf,
        db_version: String,
        migrate_calls: Arc<AtomicUsize>,
        get_version_calls: Arc<AtomicUsize>,
        migrate_should_fail: bool,
    }

    impl DatabaseInitHandler for MockHandler {
        fn init_db(
            &self,
            _config: &DatabaseConnection,
        ) -> impl std::future::Future<Output = anyhow::Result<Box<dyn Database>>> + Send {
            let db_version = self.db_version.clone();
            let get_version_calls = Arc::clone(&self.get_version_calls);
            async move {
                Ok(Box::new(MockDb {
                    version: db_version,
                    get_version_calls,
                }) as Box<dyn Database>)
            }
        }
        fn migrate_up(
            &self,
            _db: &dyn Database,
            _current_version: &str,
        ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
            let migrate_calls = Arc::clone(&self.migrate_calls);
            let migrate_should_fail = self.migrate_should_fail;
            async move {
                migrate_calls.fetch_add(1, Ordering::SeqCst);
                if migrate_should_fail {
                    anyhow::bail!("migration failed");
                }
                Ok(())
            }
        }
        fn mark_initialized(&self) -> Result<(), String> {
            fs::write(&self.flag_path, b"init").unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_init_database_internal_runs_migration_when_db_is_behind() {
        let temp = tempfile::tempdir().unwrap();
        let flag_path = temp.path().join(".db_initialized");
        let local_version_path = temp.path().join(".db_version");
        fs::write(&local_version_path, b"0.1.0\n").unwrap();
        let db_config = DatabaseConnection::SQLite {
            path: "test.db".to_string(),
        };

        let migrate_calls = Arc::new(AtomicUsize::new(0));
        let get_version_calls = Arc::new(AtomicUsize::new(0));
        let handler = MockHandler {
            flag_path: flag_path.clone(),
            db_version: "0.1.0".to_string(),
            migrate_calls: Arc::clone(&migrate_calls),
            get_version_calls: Arc::clone(&get_version_calls),
            migrate_should_fail: false,
        };

        let progress_events = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let progress_events_clone = Arc::clone(&progress_events);
        let (migrated, _) = init_database_internal(
            &db_config,
            &flag_path,
            &local_version_path,
            Some("0.3.0"),
            &handler,
            |message, _| {
                progress_events_clone
                    .lock()
                    .unwrap()
                    .push(message.to_string());
            },
        )
        .await
        .unwrap();

        assert!(migrated);
        assert!(flag_path.exists());
        assert_eq!(migrate_calls.load(Ordering::SeqCst), 1);
        assert_eq!(get_version_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            fs::read_to_string(local_version_path).unwrap().trim(),
            "0.3.0"
        );
        let events = progress_events.lock().unwrap();
        assert!(events.contains(&"db.init.migrating".to_string()));
        assert_eq!(events.last().map(String::as_str), Some("db.init.done"));
    }

    #[tokio::test]
    async fn test_init_database_internal_runs_migration_when_local_is_latest_if_db_is_behind() {
        let temp = tempfile::tempdir().unwrap();
        let flag_path = temp.path().join(".db_initialized");
        let local_version_path = temp.path().join(".db_version");
        fs::write(&local_version_path, b"0.3.0\n").unwrap();
        let db_config = DatabaseConnection::SQLite {
            path: "test.db".to_string(),
        };

        let migrate_calls = Arc::new(AtomicUsize::new(0));
        let get_version_calls = Arc::new(AtomicUsize::new(0));
        let handler = MockHandler {
            flag_path: flag_path.clone(),
            db_version: "0.1.0".to_string(),
            migrate_calls: Arc::clone(&migrate_calls),
            get_version_calls: Arc::clone(&get_version_calls),
            migrate_should_fail: false,
        };

        let (migrated, _) = init_database_internal(
            &db_config,
            &flag_path,
            &local_version_path,
            Some("0.3.0"),
            &handler,
            |_message, _| {},
        )
        .await
        .unwrap();

        assert!(migrated);
        assert!(flag_path.exists());
        assert_eq!(migrate_calls.load(Ordering::SeqCst), 1);
        assert_eq!(get_version_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            fs::read_to_string(local_version_path).unwrap().trim(),
            "0.3.0"
        );
    }

    #[tokio::test]
    async fn test_init_database_internal_syncs_local_version_when_db_is_latest() {
        let temp = tempfile::tempdir().unwrap();
        let flag_path = temp.path().join(".db_initialized");
        let local_version_path = temp.path().join(".db_version");
        fs::write(&local_version_path, b"0.2.0\n").unwrap();
        let db_config = DatabaseConnection::SQLite {
            path: "test.db".to_string(),
        };

        let migrate_calls = Arc::new(AtomicUsize::new(0));
        let get_version_calls = Arc::new(AtomicUsize::new(0));
        let handler = MockHandler {
            flag_path: flag_path.clone(),
            db_version: "0.3.0".to_string(),
            migrate_calls: Arc::clone(&migrate_calls),
            get_version_calls: Arc::clone(&get_version_calls),
            migrate_should_fail: false,
        };

        let (migrated, _) = init_database_internal(
            &db_config,
            &flag_path,
            &local_version_path,
            Some("0.3.0"),
            &handler,
            |_message, _| {},
        )
        .await
        .unwrap();

        assert!(!migrated);
        assert!(flag_path.exists());
        assert_eq!(migrate_calls.load(Ordering::SeqCst), 0);
        assert_eq!(get_version_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            fs::read_to_string(local_version_path).unwrap().trim(),
            "0.3.0"
        );
    }

    #[tokio::test]
    async fn test_init_database_internal_does_not_write_local_version_on_migration_error() {
        let temp = tempfile::tempdir().unwrap();
        let flag_path = temp.path().join(".db_initialized");
        let local_version_path = temp.path().join(".db_version");
        let db_config = DatabaseConnection::SQLite {
            path: "test.db".to_string(),
        };

        let migrate_calls = Arc::new(AtomicUsize::new(0));
        let get_version_calls = Arc::new(AtomicUsize::new(0));
        let handler = MockHandler {
            flag_path: flag_path.clone(),
            db_version: "0.1.0".to_string(),
            migrate_calls: Arc::clone(&migrate_calls),
            get_version_calls: Arc::clone(&get_version_calls),
            migrate_should_fail: true,
        };

        let result = init_database_internal(
            &db_config,
            &flag_path,
            &local_version_path,
            Some("0.3.0"),
            &handler,
            |_message, _| {},
        )
        .await;

        assert!(result.is_err());
        assert!(!flag_path.exists());
        assert!(!local_version_path.exists());
        assert_eq!(migrate_calls.load(Ordering::SeqCst), 1);
        assert_eq!(get_version_calls.load(Ordering::SeqCst), 1);
    }
}
