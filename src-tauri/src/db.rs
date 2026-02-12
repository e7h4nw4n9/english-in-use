use crate::config::DatabaseConnection;
use anyhow::{Context, Result};
use log::{debug, error, info};
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

#[derive(Debug, Deserialize)]
struct D1Response {
    result: Option<Vec<D1Result>>,
    success: bool,
    errors: Option<Vec<D1Error>>,
}

#[derive(Debug, Deserialize)]
struct D1Result {
    results: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct D1Error {
    message: String,
}

pub trait Database: Send + Sync {
    fn execute(
        &self,
        sql: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>>;
    fn get_version(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>>;
    fn set_version(
        &self,
        version: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>>;
}

pub struct SqliteDatabase {
    pool: Pool<Sqlite>,
}

impl SqliteDatabase {
    pub async fn new(path: &str) -> Result<Self> {
        info!("正在连接 SQLite 数据库: {}", path);
        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                debug!("创建数据库目录: {:?}", parent);
                std::fs::create_dir_all(parent)?;
            }
        }

        let pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}?mode=rwc", path))
            .await
            .context("Failed to connect to SQLite")?;
        info!("SQLite 数据库连接成功");
        Ok(Self { pool })
    }
}

impl Database for SqliteDatabase {
    fn execute(
        &self,
        sql: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            debug!("执行 SQL (SQLite): {}", sql);
            sqlx::query(&sql).execute(&self.pool).await.map_err(|e| {
                error!("SQL 执行失败 (SQLite): {}", e);
                e
            })?;
            Ok(())
        })
    }

    fn get_version(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>> {
        use sqlx::Row;
        Box::pin(async move {
            let exists: bool = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='_app_meta'",
            )
            .fetch_one(&self.pool)
            .await
            .unwrap_or(false);

            if !exists {
                debug!("表 _app_meta 不存在，初始版本为 0.0.0");
                return Ok("0.0.0".to_string());
            }

            let row = sqlx::query("SELECT version FROM _app_meta LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

            let version = match row {
                Some(r) => {
                    if let Ok(s) = r.try_get::<String, _>(0) {
                        s
                    } else if let Ok(i) = r.try_get::<i64, _>(0) {
                        i.to_string()
                    } else {
                        "0.0.0".to_string()
                    }
                }
                None => "0.0.0".to_string(),
            };

            debug!("当前数据库版本 (SQLite): {}", version);
            Ok(version)
        })
    }

    fn set_version(
        &self,
        version: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let version = version.to_string();
        Box::pin(async move {
            debug!("设置数据库版本 (SQLite): {}", version);
            sqlx::query("UPDATE _app_meta SET version = ?")
                .bind(version)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }
}

pub struct D1Database {
    client: Client,
    account_id: String,
    database_id: String,
    api_token: String,
}

impl D1Database {
    pub fn new(account_id: String, database_id: String, api_token: String) -> Self {
        info!("初始化 Cloudflare D1 数据库客户端: {}", database_id);
        Self {
            client: Client::new(),
            account_id,
            database_id,
            api_token,
        }
    }

    async fn raw_query(&self, sql: &str) -> Result<D1Response> {
        debug!("向 D1 发送查询请求: {}", sql);
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/d1/database/{}/query",
            self.account_id, self.database_id
        );

        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&serde_json::json!({ "sql": sql }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            error!("D1 API 错误 ({}): {}", status, text);
            return Err(anyhow::anyhow!("D1 API Error ({}): {}", status, text));
        }

        let d1_res: D1Response = res.json().await?;
        if !d1_res.success {
            let msg = d1_res
                .errors
                .as_ref()
                .and_then(|e| e.first())
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown D1 error".to_string());
            error!("D1 查询失败: {}", msg);
            return Err(anyhow::anyhow!("D1 Query Failed: {}", msg));
        }

        Ok(d1_res)
    }
}

impl Database for D1Database {
    fn execute(
        &self,
        sql: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            self.raw_query(&sql).await?;
            Ok(())
        })
    }

    fn get_version(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>> {
        Box::pin(async move {
            // Check if table exists
            let sql = "SELECT count(*) as count FROM sqlite_master WHERE type='table' AND name='_app_meta'";
            let res = self.raw_query(sql).await?;

            let count = res
                .result
                .as_ref()
                .and_then(|r| r.first())
                .and_then(|r| r.results.first())
                .and_then(|v| v.get("count"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if count == 0 {
                debug!("表 _app_meta 不存在 (D1)，初始版本为 0.0.0");
                return Ok("0.0.0".to_string());
            }

            let res = self
                .raw_query("SELECT version FROM _app_meta LIMIT 1")
                .await?;
            let version_val = res
                .result
                .as_ref()
                .and_then(|r| r.first())
                .and_then(|r| r.results.first())
                .and_then(|v| v.get("version"));

            let version = match version_val {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                _ => "0.0.0".to_string(),
            };

            debug!("当前数据库版本 (D1): {}", version);
            Ok(version)
        })
    }

    fn set_version(
        &self,
        version: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let version = version.to_string();
        Box::pin(async move {
            debug!("设置数据库版本 (D1): {}", version);
            let sql = format!("UPDATE _app_meta SET version = '{}'", version);
            self.raw_query(&sql).await?;
            Ok(())
        })
    }
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
    use crate::migrations::MIGRATIONS;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::MIGRATIONS;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_sqlite_init() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let db = SqliteDatabase::new(&path)
            .await
            .expect("Failed to create db");

        // Manual migration 1
        let v = db.get_version().await.expect("Failed to get version");
        assert_eq!(v, "0.0.0");

        db.execute(MIGRATIONS[0].sql.to_string())
            .await
            .expect("Migration failed");
        db.set_version("0.1.0").await.expect("Set version failed");

        let v = db.get_version().await.expect("Failed to get version");
        assert_eq!(v, "0.1.0");

        // Verify table exists
        db.execute("SELECT * FROM _app_meta".to_string())
            .await
            .expect("Table should exist");
    }

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("1"), "1.0.0");
        assert_eq!(normalize_version("1.1"), "1.1.0");
        assert_eq!(normalize_version("1.1.1"), "1.1.1");
    }
}
