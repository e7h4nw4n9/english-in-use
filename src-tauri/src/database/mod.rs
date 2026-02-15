use crate::models::{DatabaseConnection, ServiceStatus};
use anyhow::Result;
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
    _handle: &tauri::AppHandle<R>,
    config: &DatabaseConnection,
) -> Result<Box<dyn Database>> {
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

    Ok(db)
}

pub async fn migrate_up(db: &dyn Database, target_version: Option<&str>) -> Result<()> {
    use self::migrations::MIGRATIONS;
    migrate_up_with_list(db, target_version, MIGRATIONS).await
}

pub async fn migrate_up_with_list(
    db: &dyn Database,
    target_version: Option<&str>,
    migrations: &[self::migrations::Migration],
) -> Result<()> {
    let current_db_version_str = db.get_version().await?;
    let normalized_db_version = normalize_version(&current_db_version_str);
    let current_db_version =
        Version::parse(&normalized_db_version).unwrap_or_else(|_| Version::parse("0.0.0").unwrap());

    let target_v = if let Some(v) = target_version {
        Some(Version::parse(&normalize_version(v))?)
    } else {
        None
    };

    for migration in migrations {
        let migration_version = Version::parse(&normalize_version(migration.version))?;

        if migration_version > current_db_version {
            if let Some(ref tv) = target_v {
                if migration_version > *tv {
                    break;
                }
            }

            info!("正在应用升级迁移至版本 {}...", migration.version);
            db.execute(migration.up.to_string()).await?;
            db.set_version(migration.version).await?;
        }
    }
    Ok(())
}

pub async fn migrate_down(db: &dyn Database, target_version: Option<&str>) -> Result<()> {
    use self::migrations::MIGRATIONS;
    migrate_down_with_list(db, target_version, MIGRATIONS).await
}

pub async fn migrate_down_with_list(
    db: &dyn Database,
    target_version: Option<&str>,
    migrations: &[self::migrations::Migration],
) -> Result<()> {
    let current_db_version_str = db.get_version().await?;
    let normalized_db_version = normalize_version(&current_db_version_str);
    let current_db_version =
        Version::parse(&normalized_db_version).unwrap_or_else(|_| Version::parse("0.0.0").unwrap());

    let target_v = if let Some(v) = target_version {
        Version::parse(&normalize_version(v))?
    } else {
        // Default to one version down
        let mut prev_version = Version::parse("0.0.0").unwrap();
        for migration in migrations {
            let mv = Version::parse(&normalize_version(migration.version))?;
            if mv < current_db_version && mv > prev_version {
                prev_version = mv;
            }
        }
        prev_version
    };

    // Migrations are sorted ascending, so we need to iterate in reverse for downgrade
    for migration in migrations.iter().rev() {
        let migration_version = Version::parse(&normalize_version(migration.version))?;

        if migration_version <= current_db_version && migration_version > target_v {
            info!("正在应用降级迁移至版本 {}...", migration.version);
            if !migration.down.is_empty() {
                db.execute(migration.down.to_string()).await?;
            }

            // Set version to the one BEFORE this migration
            let mut prev_v = "0.0.0".to_string();
            for m in migrations {
                let mv = Version::parse(&normalize_version(m.version))?;
                if mv < migration_version {
                    prev_v = m.version.to_string();
                } else {
                    break;
                }
            }
            db.set_version(&prev_v).await?;
        }
    }
    Ok(())
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
        let status = check_status(&conn).await;
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
}
