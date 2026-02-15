use crate::database::{self, Database, DbState};
use crate::models::{DatabaseConnection, config};
use anyhow::Result;
use log::{debug, info};
use std::fs;
use tauri::{AppHandle, Manager};

/// 获取数据库初始化标志文件路径
fn get_init_flag_path() -> Result<std::path::PathBuf, String> {
    let data_dir = crate::utils::local::get_app_data_dir()?;
    if !data_dir.exists() {
        fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    }
    Ok(data_dir.join(".db_initialized"))
}

/// 将数据库标记为已初始化
pub fn mark_as_initialized() -> Result<(), String> {
    let path = get_init_flag_path()?;
    fs::write(&path, b"initialized").map_err(|e| e.to_string())?;
    debug!("已创建数据库初始化标志文件: {:?}", path);
    Ok(())
}

pub trait DatabaseInitHandler: Send + Sync {
    async fn init_db(&self, config: &DatabaseConnection) -> anyhow::Result<Box<dyn Database>>;
    async fn migrate_up(&self, db: &dyn Database) -> anyhow::Result<()>;
    fn mark_initialized(&self) -> Result<(), String>;
}

struct DefaultInitHandler {
    app: AppHandle,
}

impl DatabaseInitHandler for DefaultInitHandler {
    async fn init_db(&self, config: &DatabaseConnection) -> anyhow::Result<Box<dyn Database>> {
        database::init(&self.app, config).await
    }
    async fn migrate_up(&self, db: &dyn Database) -> anyhow::Result<()> {
        database::migrate_up(db, None).await
    }
    fn mark_initialized(&self) -> Result<(), String> {
        mark_as_initialized()
    }
}

/// 初始化数据库并根据标志执行迁移。返回布尔值表示是否执行了新的迁移。
pub async fn init_database(app: &AppHandle) -> Result<bool, String> {
    let config = {
        use crate::services::config::ConfigState;
        let state = app.state::<ConfigState>();
        let config = state.0.read().unwrap();
        config.clone()
    };
    let db_config = match config.database {
        Some(db_config) => db_config,
        None => {
            debug!("未配置数据库，跳过初始化");
            return Ok(false);
        }
    };

    let init_flag_path = get_init_flag_path()?;
    let handler = DefaultInitHandler { app: app.clone() };

    let (migrated, db) = init_database_internal(&db_config, &init_flag_path, &handler).await?;

    // 将数据库句柄存入 DbState 以便全局使用
    let db_state = app.state::<DbState>();
    let mut db_guard = db_state.db.write().await;
    *db_guard = Some(db);

    Ok(migrated)
}

pub async fn init_database_internal<H: DatabaseInitHandler + ?Sized>(
    db_config: &DatabaseConnection,
    init_flag_path: &std::path::Path,
    handler: &H,
) -> Result<(bool, Box<dyn Database>), String> {
    let mut migrated = false;

    // 初始化数据库连接
    let db = handler
        .init_db(db_config)
        .await
        .map_err(|e| e.to_string())?;

    // 如果标志文件不存在，说明是第一次运行或需要强制初始化/迁移
    if !init_flag_path.exists() {
        info!("检测到数据库未初始化（标志文件不存在），正在执行迁移...");
        handler
            .migrate_up(db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        // 创建标志文件
        handler.mark_initialized()?;
        info!("数据库初始化及迁移完成");
        migrated = true;
    } else {
        debug!("数据库已初始化（标志文件已存在）");
    }

    Ok((migrated, db))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::Value;
    use std::future::Future;
    use std::pin::Pin;

    struct MockDb;
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
        fn get_version(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + '_>> {
            Box::pin(async { Ok("0.1.0".to_string()) })
        }
        fn set_version(
            &self,
            _version: &str,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async { Ok(()) })
        }
    }

    struct MockHandler {
        flag_path: std::path::PathBuf,
    }

    impl DatabaseInitHandler for MockHandler {
        async fn init_db(&self, _config: &DatabaseConnection) -> anyhow::Result<Box<dyn Database>> {
            Ok(Box::new(MockDb))
        }
        async fn migrate_up(&self, _db: &dyn Database) -> anyhow::Result<()> {
            Ok(())
        }
        fn mark_initialized(&self) -> Result<(), String> {
            fs::write(&self.flag_path, b"init").unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_init_database_internal_first_time() {
        let temp = tempfile::tempdir().unwrap();
        let flag_path = temp.path().join(".db_initialized");
        let db_config = DatabaseConnection::SQLite {
            path: "test.db".to_string(),
        };
        let handler = MockHandler {
            flag_path: flag_path.clone(),
        };

        let (migrated, _) = init_database_internal(&db_config, &flag_path, &handler)
            .await
            .unwrap();

        assert!(migrated);
        assert!(flag_path.exists());
    }

    #[tokio::test]
    async fn test_init_database_internal_second_time() {
        let temp = tempfile::tempdir().unwrap();
        let flag_path = temp.path().join(".db_initialized");
        fs::write(&flag_path, b"init").unwrap();
        let db_config = DatabaseConnection::SQLite {
            path: "test.db".to_string(),
        };
        let handler = MockHandler {
            flag_path: flag_path.clone(),
        };

        let (migrated, _) = init_database_internal(&db_config, &flag_path, &handler)
            .await
            .unwrap();

        assert!(!migrated);
    }
}
