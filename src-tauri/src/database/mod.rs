use crate::models::{DatabaseConnection, ServiceStatus};
use anyhow::{Context, Result};
use log::info;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod d1;
pub mod migrations;
mod sqlite;

pub use d1::D1Database;
pub use sqlite::SqliteDatabase;

pub trait Database: Send + Sync {
    fn execute(
        &self,
        sql: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>>;
    fn query(
        &self,
        sql: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Value>>> + Send + '_>>;
    fn get_version(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>>;
    fn set_version(
        &self,
        version: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>>;
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

pub async fn init<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
    config: &DatabaseConnection,
) -> Result<Box<dyn Database>> {
    use self::migrations::MIGRATIONS;
    use tauri::Emitter;

    let emit_progress = |msg: &str, p: f32| {
        info!("初始化进度: {} ({}%)", msg, (p * 100.0) as u32);
        let _ = handle.emit(
            "init-progress",
            AppInitProgress {
                message: msg.to_string(),
                progress: p,
            },
        );
    };

    emit_progress("正在连接数据库...", 0.1);

    let db: Box<dyn Database> = match config {
        DatabaseConnection::SQLite { path } => Box::new(SqliteDatabase::new(path).await?),
        DatabaseConnection::CloudflareD1 {
            account_id,
            database_id,
            api_token,
        } => Box::new(D1Database::new(
            account_id.clone(),
            database_id.clone(),
            api_token.clone(),
        )),
    };

    emit_progress("正在检查数据库版本...", 0.3);
    let current_db_version_str = db.get_version().await?;
    let normalized_db_version = normalize_version(&current_db_version_str);
    let current_db_version =
        Version::parse(&normalized_db_version).unwrap_or_else(|_| Version::parse("0.0.0").unwrap());

    info!(
        "当前数据库版本: {} (原始: {})",
        current_db_version, current_db_version_str
    );

    let total_migrations = MIGRATIONS.len() as f32;
    for (i, migration) in MIGRATIONS.iter().enumerate() {
        let migration_version = Version::parse(migration.version)
            .with_context(|| format!("Failed to parse migration version: {}", migration.version))?;

        if migration_version > current_db_version {
            let p = 0.3 + (i as f32 / total_migrations) * 0.6;
            emit_progress(&format!("正在应用迁移至版本 {}...", migration.version), p);
            db.execute(migration.sql.to_string()).await?;
            db.set_version(migration.version).await?;
            info!("已成功应用迁移至版本 {}", migration.version);
        }
    }

    emit_progress("数据库初始化完成", 1.0);
    Ok(db)
}

pub struct DbState {
    pub db: tokio::sync::RwLock<Option<Box<dyn Database>>>,
}

impl Default for DbState {
    fn default() -> Self {
        Self {
            db: tokio::sync::RwLock::new(None),
        }
    }
}

pub async fn check_status(connection: &DatabaseConnection) -> ServiceStatus {
    match connection {
        DatabaseConnection::SQLite { path } => SqliteDatabase::check_status(path).await,
        DatabaseConnection::CloudflareD1 {
            account_id,
            database_id,
            api_token,
        } => D1Database::check_status(account_id, database_id, api_token).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let status = check_status(&conn).await;
        assert_eq!(status, ServiceStatus::Connected);
    }
}
