use super::Database;
use crate::models::ServiceStatus;
use anyhow::{Context, Result};
use log::{debug, error, info};
use serde_json::Value;
use sqlx::{Column, Decode, Row, Type, TypeInfo, sqlite::SqliteRow};
use sqlx::{Pool, Sqlite, ValueRef, sqlite::SqlitePoolOptions};

// --- 类型转换 Trait ---
trait ToJson {
    fn to_json(self) -> Value;
}

impl ToJson for i64 {
    fn to_json(self) -> Value {
        self.into()
    }
}
impl ToJson for bool {
    fn to_json(self) -> Value {
        self.into()
    }
}
impl ToJson for String {
    fn to_json(self) -> Value {
        self.into()
    }
}
impl ToJson for f64 {
    fn to_json(self) -> Value {
        serde_json::Number::from_f64(self)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

// --- SQLite 类型关联枚举 ---
enum SqliteAffinity {
    Integer,
    Real,
    Text,
    Blob,
    Boolean,
}

impl SqliteAffinity {
    fn from_type_name(name: &str) -> Self {
        let name = name.to_uppercase();
        if name.contains("INT") {
            Self::Integer
        } else if name.contains("CHAR") || name.contains("TEXT") || name.contains("CLOB") {
            Self::Text
        } else if name.contains("REAL") || name.contains("FLOA") || name.contains("DOUB") {
            Self::Real
        } else if name.contains("BOOL") {
            Self::Boolean
        } else {
            Self::Blob
        }
    }
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

    pub async fn check_status(path: &str) -> ServiceStatus {
        debug!("执行 SQLite 状态检查: {}", path);
        let path_obj = std::path::Path::new(path);
        if let Some(parent) = path_obj.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("创建 SQLite 目录失败: {}", e);
                return ServiceStatus::Disconnected(format!("Failed to create directory: {}", e));
            }
        }

        match SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}?mode=rwc", path))
            .await
        {
            Ok(_) => ServiceStatus::Connected,
            Err(e) => {
                error!("SQLite 连接失败: {}", e);
                ServiceStatus::Disconnected(format!("SQLite connection failed: {}", e))
            }
        }
    }

    // --- 辅助解码函数 ---
    fn decode<'r, T>(row: &'r SqliteRow, col: &str) -> Value
    where
        T: Decode<'r, Sqlite> + Type<Sqlite> + ToJson,
    {
        row.try_get::<T, _>(col)
            .map(T::to_json)
            .unwrap_or(Value::Null)
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

    fn query(
        &self,
        sql: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Value>>> + Send + '_>> {
        Box::pin(async move {
            debug!("执行查询 (SQLite): {}", sql);
            let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
            let mut results = Vec::new();
            for row in rows {
                let mut map = serde_json::Map::new();
                for col in row.columns() {
                    let name = col.name();
                    let value = if row.try_get_raw(name).map_or(true, |v| v.is_null()) {
                        Value::Null
                    } else {
                        match SqliteAffinity::from_type_name(col.type_info().name()) {
                            SqliteAffinity::Integer => Self::decode::<i64>(&row, name),
                            SqliteAffinity::Real => Self::decode::<f64>(&row, name),
                            SqliteAffinity::Text => Self::decode::<String>(&row, name),
                            SqliteAffinity::Boolean => Self::decode::<bool>(&row, name),
                            SqliteAffinity::Blob => {
                                let v: Vec<u8> = row.try_get(name).unwrap_or_default();
                                match String::from_utf8(v) {
                                    Ok(s) => Value::String(s),
                                    Err(_) => Value::String("<BINARY>".to_string()),
                                }
                            }
                        }
                    };
                    map.insert(name.to_string(), value);
                }
                results.push(Value::Object(map));
            }
            Ok(results)
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
                Some(r) => match r.try_get::<String, _>(0) {
                    Ok(s) => s,
                    _ => match r.try_get::<i64, _>(0) {
                        Ok(i) => i.to_string(),
                        _ => "0.0.0".to_string(),
                    },
                },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::MIGRATIONS;
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

        db.execute(MIGRATIONS[0].up.to_string())
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
}
