use crate::models::{DatabaseConnection, ServiceStatus};
use anyhow::Result;
use log::info;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::Manager;

mod d1;
pub mod migrations;
mod sqlite;

pub use d1::D1Database;
pub use sqlite::SqliteDatabase;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize)]
pub struct SqlStatement {
    pub sql: String,
    pub params: Vec<SqlValue>,
}

/// 数据库异步操作统一返回类型。
pub type DatabaseFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

impl SqlStatement {
    /// 创建一条带绑定参数的 SQL 语句。
    ///
    /// # 参数
    /// - `sql`：SQL 文本。
    /// - `params`：SQL 绑定参数。
    pub fn new(sql: impl Into<String>, params: Vec<SqlValue>) -> Self {
        Self {
            sql: sql.into(),
            params,
        }
    }
}

pub trait Database: Send + Sync {
    /// 执行不返回结果集的 SQL 文本。
    fn execute(&self, sql: String) -> DatabaseFuture<'_, ()>;
    /// 执行 SQL 查询并返回 JSON 行。
    fn query(&self, sql: String) -> DatabaseFuture<'_, Vec<Value>>;
    /// 执行一条参数化 SQL 语句；默认实现仅接受空参数。
    fn execute_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, ()> {
        Box::pin(async move {
            if !statement.params.is_empty() {
                anyhow::bail!("当前数据库实现不支持参数化执行");
            }
            self.execute(statement.sql).await
        })
    }
    /// 执行一条参数化查询；默认实现仅接受空参数。
    fn query_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, Vec<Value>> {
        Box::pin(async move {
            if !statement.params.is_empty() {
                anyhow::bail!("当前数据库实现不支持参数化查询");
            }
            self.query(statement.sql).await
        })
    }
    /// 执行会修改数据并返回结果集的参数化语句。
    fn query_write_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, Vec<Value>> {
        self.query_statement(statement)
    }
    /// 顺序执行一组 SQL 语句；数据库实现可以覆盖为原子批处理。
    fn execute_batch(&self, statements: Vec<SqlStatement>) -> DatabaseFuture<'_, ()> {
        Box::pin(async move {
            for statement in statements {
                self.execute_statement(statement).await?;
            }
            Ok(())
        })
    }
    /// 有序执行一组查询并分别返回结果集；云数据库可覆盖为单次网络请求。
    fn query_batch(&self, statements: Vec<SqlStatement>) -> DatabaseFuture<'_, Vec<Vec<Value>>> {
        Box::pin(async move {
            let mut results = Vec::with_capacity(statements.len());
            for statement in statements {
                results.push(self.query_statement(statement).await?);
            }
            Ok(results)
        })
    }
    /// 读取数据库中记录的应用迁移版本。
    fn get_version(&self) -> DatabaseFuture<'_, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInitProgress {
    pub message: String,
    pub progress: f32,
}

fn normalize_version(v: &str) -> String {
    let parts: Vec<&str> = v.split('.').collect();
    match parts.len() {
        1 => format!("{}.0.0", v),
        2 => format!("{}.0", v),
        _ => v.to_string(),
    }
}

/// 按受控迁移标记拆分 SQL，避免依赖不安全的通用 SQL 分号解析。
fn split_migration_statements(sql: &str) -> impl Iterator<Item = &str> {
    sql.split("-- statement-breakpoint")
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
}

/// 根据数据库配置创建统一的数据库实现。
///
/// # 参数
/// - `_handle`：为统一初始化接口保留的应用句柄。
/// - `config`：数据库或应用配置。
pub async fn init<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
    config: &DatabaseConnection,
    gateway: Option<&crate::models::CloudflareGatewayConfig>,
) -> Result<Box<dyn Database>> {
    let db: Box<dyn Database> = match config {
        DatabaseConnection::SQLite { path } => Box::new(SqliteDatabase::new(path).await?),
        DatabaseConnection::CloudflareGateway {} => {
            let gateway = gateway.ok_or_else(|| anyhow::anyhow!("需要配置 Cloudflare 网关"))?;
            let state = handle.state::<crate::utils::gateway::GatewayClientState>();
            let config_state = handle.state::<crate::services::config::ConfigState>();
            let version = config_state
                .0
                .read()
                .map_err(|error| anyhow::anyhow!(error.to_string()))?
                .version;
            Box::new(D1Database::from_client(
                state
                    .get(version, gateway)
                    .await
                    .map_err(anyhow::Error::msg)?,
            ))
        }
    };

    Ok(db)
}

/// 使用内置迁移将数据库升级到指定版本或最新版本。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `target_version`：目标迁移版本；为空时使用默认目标。
pub async fn migrate_up(db: &dyn Database, target_version: Option<&str>) -> Result<()> {
    use self::migrations::MIGRATIONS;
    migrate_up_with_list(db, target_version, MIGRATIONS).await
}

/// 从已读取的数据库版本继续升级，避免初始化流程重复访问远端版本表。
pub async fn migrate_up_from_version(
    db: &dyn Database,
    current_version: &str,
    target_version: Option<&str>,
) -> Result<()> {
    use self::migrations::MIGRATIONS;
    migrate_up_from_version_with_list(db, current_version, target_version, MIGRATIONS).await
}

/// 使用给定迁移列表按顺序升级数据库。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `target_version`：目标迁移版本；为空时使用默认目标。
/// - `migrations`：按版本升序排列的迁移列表。
pub async fn migrate_up_with_list(
    db: &dyn Database,
    target_version: Option<&str>,
    migrations: &[self::migrations::Migration],
) -> Result<()> {
    let current_db_version_str = db.get_version().await?;
    migrate_up_from_version_with_list(db, &current_db_version_str, target_version, migrations).await
}

async fn migrate_up_from_version_with_list(
    db: &dyn Database,
    current_db_version_str: &str,
    target_version: Option<&str>,
    migrations: &[self::migrations::Migration],
) -> Result<()> {
    let normalized_db_version = normalize_version(current_db_version_str);
    let current_db_version = Version::parse(&normalized_db_version)
        .map_err(|error| anyhow::anyhow!("数据库版本无效 ({current_db_version_str}): {error}"))?;

    let target_v = if let Some(v) = target_version {
        Some(Version::parse(&normalize_version(v))?)
    } else {
        None
    };

    let mut statements = Vec::new();
    for migration in migrations {
        let migration_version = Version::parse(&normalize_version(migration.version))?;

        if migration_version > current_db_version {
            if let Some(ref tv) = target_v
                && migration_version > *tv
            {
                break;
            }

            info!("正在应用升级迁移至版本 {}...", migration.version);
            statements.extend(
                split_migration_statements(migration.up).map(|sql| SqlStatement::new(sql, vec![])),
            );
            statements.push(SqlStatement::new(
                "UPDATE _app_meta SET version = ?",
                vec![SqlValue::Text(migration.version.to_string())],
            ));
        }
    }
    if !statements.is_empty() {
        db.execute_batch(statements).await?;
    }
    Ok(())
}

/// 使用内置迁移将数据库降级到指定版本或前一版本。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `target_version`：目标迁移版本；为空时使用默认目标。
pub async fn migrate_down(db: &dyn Database, target_version: Option<&str>) -> Result<()> {
    use self::migrations::MIGRATIONS;
    migrate_down_with_list(db, target_version, MIGRATIONS).await
}

/// 使用给定迁移列表按逆序降级数据库。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `target_version`：目标迁移版本；为空时使用默认目标。
/// - `migrations`：按版本升序排列的迁移列表。
pub async fn migrate_down_with_list(
    db: &dyn Database,
    target_version: Option<&str>,
    migrations: &[self::migrations::Migration],
) -> Result<()> {
    let current_db_version_str = db.get_version().await?;
    let normalized_db_version = normalize_version(&current_db_version_str);
    let current_db_version = Version::parse(&normalized_db_version)
        .map_err(|error| anyhow::anyhow!("数据库版本无效 ({current_db_version_str}): {error}"))?;

    let target_v = if let Some(v) = target_version {
        Version::parse(&normalize_version(v))?
    } else {
        // 未指定目标时默认降级一个版本。
        let mut prev_version = Version::parse("0.0.0").unwrap();
        for migration in migrations {
            let mv = Version::parse(&normalize_version(migration.version))?;
            if mv < current_db_version && mv > prev_version {
                prev_version = mv;
            }
        }
        prev_version
    };

    // 迁移按版本升序排列，降级时必须逆序执行。
    let mut statements = Vec::new();
    for migration in migrations.iter().rev() {
        let migration_version = Version::parse(&normalize_version(migration.version))?;

        if migration_version <= current_db_version && migration_version > target_v {
            info!("正在应用降级迁移至版本 {}...", migration.version);
            // 将版本号更新为当前降级迁移之前的版本。
            let mut prev_v = "0.0.0".to_string();
            for m in migrations {
                let mv = Version::parse(&normalize_version(m.version))?;
                if mv < migration_version {
                    prev_v = m.version.to_string();
                } else {
                    break;
                }
            }
            if !migration.down.is_empty() {
                statements.extend(
                    split_migration_statements(migration.down)
                        .map(|sql| SqlStatement::new(sql, vec![])),
                );
            }
            statements.push(SqlStatement::new(
                "UPDATE _app_meta SET version = ?",
                vec![SqlValue::Text(prev_v)],
            ));
        }
    }
    if !statements.is_empty() {
        db.execute_batch(statements).await?;
    }
    Ok(())
}

pub struct DbState {
    pub db: tokio::sync::RwLock<Option<Arc<dyn Database>>>,
    pub connection: tokio::sync::RwLock<Option<DatabaseConnection>>,
    pub config_version: tokio::sync::RwLock<Option<uuid::Uuid>>,
    pub init_lock: tokio::sync::Mutex<()>,
}

impl Default for DbState {
    fn default() -> Self {
        Self {
            db: tokio::sync::RwLock::new(None),
            connection: tokio::sync::RwLock::new(None),
            config_version: tokio::sync::RwLock::new(None),
            init_lock: tokio::sync::Mutex::new(()),
        }
    }
}

impl DbState {
    /// 克隆数据库句柄，避免在网络或磁盘等待期间占用状态读锁。
    pub async fn get(&self) -> Result<Arc<dyn Database>, String> {
        self.db
            .read()
            .await
            .clone()
            .ok_or_else(|| "Database not initialized".to_string())
    }
}

/// 根据连接类型检查当前数据库服务状态。
///
/// # 参数
/// - `connection`：数据库连接配置。
pub async fn check_status(
    connection: &DatabaseConnection,
    gateway: Option<&crate::models::CloudflareGatewayConfig>,
) -> ServiceStatus {
    match connection {
        DatabaseConnection::SQLite { path } => SqliteDatabase::check_status(path).await,
        DatabaseConnection::CloudflareGateway {} => match gateway {
            Some(gateway) => D1Database::check_status(gateway).await,
            None => ServiceStatus::NotConfigured,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::Migration;

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("1"), "1.0.0");
        assert_eq!(normalize_version("1.1"), "1.1.0");
        assert_eq!(normalize_version("1.1.1"), "1.1.1");
    }

    #[tokio::test]
    async fn test_check_status_sqlite() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let conn = DatabaseConnection::SQLite { path };
        let status = check_status(&conn, None).await;
        assert_eq!(status, ServiceStatus::Connected);
    }

    #[tokio::test]
    async fn test_migration_logic() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();

        static TEST_MIGRATIONS: &[Migration] = &[
            Migration {
                version: "0.1.0",
                up: "CREATE TABLE _app_meta (version TEXT); INSERT INTO _app_meta (version) VALUES ('0.0.0'); CREATE TABLE t1 (id INTEGER);",
                down: "DROP TABLE t1;",
            },
            Migration {
                version: "0.2.0",
                up: "CREATE TABLE t2 (id INTEGER);",
                down: "DROP TABLE t2;",
            },
        ];

        // Initial state
        assert_eq!(db.get_version().await.unwrap(), "0.0.0");

        // Migrate up to 0.1.0
        migrate_up_with_list(&db, Some("0.1.0"), TEST_MIGRATIONS)
            .await
            .unwrap();
        assert_eq!(db.get_version().await.unwrap(), "0.1.0");
        db.execute("SELECT * FROM t1".to_string()).await.unwrap();
        db.execute("SELECT * FROM t2".to_string())
            .await
            .unwrap_err();

        // Migrate up to latest (0.2.0)
        migrate_up_with_list(&db, None, TEST_MIGRATIONS)
            .await
            .unwrap();
        assert_eq!(db.get_version().await.unwrap(), "0.2.0");
        db.execute("SELECT * FROM t2".to_string()).await.unwrap();

        // Migrate down to 0.1.0
        migrate_down_with_list(&db, Some("0.1.0"), TEST_MIGRATIONS)
            .await
            .unwrap();
        assert_eq!(db.get_version().await.unwrap(), "0.1.0");
        db.execute("SELECT * FROM t2".to_string())
            .await
            .unwrap_err();
        db.execute("SELECT * FROM t1".to_string()).await.unwrap();

        // Migrate down to 0.0.0
        migrate_down_with_list(&db, Some("0.0.0"), TEST_MIGRATIONS)
            .await
            .unwrap();
        assert_eq!(db.get_version().await.unwrap(), "0.0.0");
        db.execute("SELECT * FROM t1".to_string())
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn test_real_migrations_integration() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();

        // 1. Migrate Up to latest
        migrate_up(&db, None)
            .await
            .expect("Real migration UP failed");

        let version = db.get_version().await.unwrap();
        assert_ne!(version, "0.0.0");
        info!("Migrated to real version: {}", version);

        // 2. Verify some tables exist (e.g., _app_meta, books)
        db.query("SELECT * FROM _app_meta".to_string())
            .await
            .expect("Table _app_meta should exist");
        db.query("SELECT * FROM books".to_string())
            .await
            .expect("Table books should exist");
        let query_plan = db
            .query(
                "EXPLAIN QUERY PLAN SELECT * FROM study_sessions WHERE book_id = 1 AND local_date = '2026-08-03'"
                    .to_string(),
            )
            .await
            .unwrap();
        assert!(query_plan.iter().any(|row| {
            row.get("detail")
                .and_then(Value::as_str)
                .is_some_and(|detail| detail.contains("idx_study_sessions_book_local_date"))
        }));

        // 3. Migrate Down to 0.0.0
        migrate_down(&db, Some("0.0.0"))
            .await
            .expect("Real migration DOWN failed");
        assert_eq!(db.get_version().await.unwrap(), "0.0.0");

        // 4. Verify tables are gone
        db.query("SELECT * FROM books".to_string())
            .await
            .expect_err("Table books should be dropped");
    }

    #[test]
    fn test_fresh_migrations_fit_gateway_batch_limit() {
        let statement_count: usize = migrations::MIGRATIONS
            .iter()
            .map(|migration| split_migration_statements(migration.up).count() + 1)
            .sum();

        assert!(
            statement_count <= 32,
            "完整迁移包含 {statement_count} 条语句，超过网关单批 32 条限制"
        );
    }

    #[tokio::test]
    async fn test_reading_progress_migration_removes_autoincrement_sequence() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();
        migrate_up(&db, Some("0.5.0")).await.unwrap();
        db.execute(
            "INSERT INTO reading_progress (book_id, page_label) VALUES (1, '1')".to_string(),
        )
        .await
        .unwrap();
        for page in 2..=10 {
            db.execute(format!(
                "INSERT INTO reading_progress (book_id, page_label) VALUES (1, '{page}') \
                 ON CONFLICT(book_id) DO UPDATE SET page_label=excluded.page_label"
            ))
            .await
            .unwrap();
        }

        migrate_up(&db, None).await.unwrap();

        let progress = db
            .query("SELECT book_id, page_label FROM reading_progress".to_string())
            .await
            .unwrap();
        assert_eq!(progress[0]["book_id"], 1);
        assert_eq!(progress[0]["page_label"], "10");
        let columns = db
            .query("PRAGMA table_info(reading_progress)".to_string())
            .await
            .unwrap();
        assert!(!columns.iter().any(|column| column["name"] == "id"));
        let sequence = db
            .query("SELECT seq FROM sqlite_sequence WHERE name = 'reading_progress'".to_string())
            .await
            .unwrap();
        assert!(sequence.is_empty());
    }
}
