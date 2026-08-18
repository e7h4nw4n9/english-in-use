use super::{Database, DatabaseFuture, SqlStatement, SqlValue};
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
    /// 打开 SQLite 数据库并准备连接池。
    ///
    /// # 参数
    /// - `path`：目标文件或目录路径。
    pub async fn new(path: &str) -> Result<Self> {
        info!("正在连接 SQLite 数据库: {}", path);
        // 建立连接前确保数据库父目录存在。
        if let Some(parent) = std::path::Path::new(path).parent()
            && !parent.exists()
        {
            debug!("创建数据库目录: {:?}", parent);
            std::fs::create_dir_all(parent)?;
        }

        let pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}?mode=rwc", path))
            .await
            .context("Failed to connect to SQLite")?;
        info!("SQLite 数据库连接成功");
        Ok(Self { pool })
    }

    /// 检查 SQLite 路径是否可以建立数据库连接。
    ///
    /// # 参数
    /// - `path`：目标文件或目录路径。
    pub async fn check_status(path: &str) -> ServiceStatus {
        debug!("执行 SQLite 状态检查: {}", path);
        let path_obj = std::path::Path::new(path);
        if let Some(parent) = path_obj.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            error!("创建 SQLite 目录失败: {}", e);
            return ServiceStatus::Disconnected(format!("Failed to create directory: {}", e));
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

    /// 按统一 SqlValue 类型顺序绑定 SQLite 查询参数。
    ///
    /// # 参数
    /// - `query`：待绑定参数的 SQLite 查询。
    /// - `params`：顺序绑定参数。
    fn bind_query<'q>(
        mut query: sqlx::query::Query<'q, Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
        params: &[SqlValue],
    ) -> sqlx::query::Query<'q, Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
        for value in params {
            query = match value {
                SqlValue::Null => query.bind(Option::<String>::None),
                SqlValue::Integer(value) => query.bind(*value),
                SqlValue::Real(value) => query.bind(*value),
                SqlValue::Text(value) => query.bind(value.clone()),
                SqlValue::Boolean(value) => query.bind(*value),
            };
        }
        query
    }

    /// 根据 SQLite 运行时列类型将查询结果转换为 JSON 行。
    ///
    /// # 参数
    /// - `rows`：SQLite 查询结果行流。
    fn rows_to_json(rows: Vec<SqliteRow>) -> Vec<Value> {
        let mut results = Vec::new();
        for row in rows {
            let mut map = serde_json::Map::new();
            for col in row.columns() {
                let name = col.name();
                let raw_value = row.try_get_raw(name);
                let value = if raw_value.as_ref().map_or(true, |value| value.is_null()) {
                    Value::Null
                } else {
                    // 查询表达式未必有可靠的声明类型，应以当前值的运行时类型解码。
                    let type_name = raw_value
                        .as_ref()
                        .map(|value| value.type_info().name().to_string())
                        .unwrap_or_default();
                    match SqliteAffinity::from_type_name(&type_name) {
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
        results
    }
}

impl Database for SqliteDatabase {
    fn execute(&self, sql: String) -> DatabaseFuture<'_, ()> {
        Box::pin(async move {
            debug!("执行 SQL (SQLite): {}", sql);
            sqlx::query(&sql).execute(&self.pool).await.map_err(|e| {
                error!("SQL 执行失败 (SQLite): {}", e);
                e
            })?;
            Ok(())
        })
    }

    fn query(&self, sql: String) -> DatabaseFuture<'_, Vec<Value>> {
        Box::pin(async move {
            debug!("执行查询 (SQLite): {}", sql);
            let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
            Ok(Self::rows_to_json(rows))
        })
    }

    fn execute_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, ()> {
        Box::pin(async move {
            debug!("执行参数化 SQL (SQLite): {}", statement.sql);
            Self::bind_query(sqlx::query(&statement.sql), &statement.params)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn query_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, Vec<Value>> {
        Box::pin(async move {
            debug!("执行参数化查询 (SQLite): {}", statement.sql);
            let rows = Self::bind_query(sqlx::query(&statement.sql), &statement.params)
                .fetch_all(&self.pool)
                .await?;
            Ok(Self::rows_to_json(rows))
        })
    }

    fn execute_batch(&self, statements: Vec<SqlStatement>) -> DatabaseFuture<'_, ()> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await?;
            for statement in statements {
                Self::bind_query(sqlx::query(&statement.sql), &statement.params)
                    .execute(&mut *transaction)
                    .await?;
            }
            transaction.commit().await?;
            Ok(())
        })
    }

    fn query_batch(&self, statements: Vec<SqlStatement>) -> DatabaseFuture<'_, Vec<Vec<Value>>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await?;
            let mut results = Vec::with_capacity(statements.len());
            for statement in statements {
                let rows = Self::bind_query(sqlx::query(&statement.sql), &statement.params)
                    .fetch_all(&mut *transaction)
                    .await?;
                results.push(Self::rows_to_json(rows));
            }
            transaction.commit().await?;
            Ok(results)
        })
    }

    fn query_write_batch(
        &self,
        statements: Vec<SqlStatement>,
    ) -> DatabaseFuture<'_, Vec<Vec<Value>>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await?;
            let mut results = Vec::with_capacity(statements.len());
            for statement in statements {
                let rows = Self::bind_query(sqlx::query(&statement.sql), &statement.params)
                    .fetch_all(&mut *transaction)
                    .await?;
                results.push(Self::rows_to_json(rows));
            }
            transaction.commit().await?;
            Ok(results)
        })
    }

    fn get_version(&self) -> DatabaseFuture<'_, String> {
        Box::pin(async move {
            let table_count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='_app_meta'",
            )
            .fetch_one(&self.pool)
            .await?;

            if table_count == 0 {
                debug!("表 _app_meta 不存在，初始版本为 0.0.0");
                return Ok("0.0.0".to_string());
            }

            let version = sqlx::query_scalar::<_, String>("SELECT version FROM _app_meta LIMIT 1")
                .fetch_optional(&self.pool)
                .await?
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("_app_meta.version 缺失或格式无效"))?;

            debug!("当前数据库版本 (SQLite): {}", version);
            Ok(version)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        crate::database::migrate_up_from_version(&db, "0.0.0", Some("0.1.0"))
            .await
            .expect("Migration failed");

        let v = db.get_version().await.expect("Failed to get version");
        assert_eq!(v, "0.1.0");

        // Verify table exists
        db.execute("SELECT * FROM _app_meta".to_string())
            .await
            .expect("Table should exist");
    }

    #[tokio::test]
    async fn test_query_decodes_expression_runtime_types() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();

        let rows = db
            .query(
                "SELECT 42 AS integer_value, 1.5 AS real_value, 'text' AS text_value".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(rows[0]["integer_value"], 42);
        assert_eq!(rows[0]["real_value"], 1.5);
        assert_eq!(rows[0]["text_value"], "text");
    }

    #[tokio::test]
    async fn test_existing_meta_table_requires_version() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();

        db.execute("CREATE TABLE _app_meta (version TEXT NOT NULL)".to_string())
            .await
            .unwrap();

        let error = db.get_version().await.unwrap_err();
        assert!(error.to_string().contains("version"));
    }

    #[tokio::test]
    async fn test_migration_rejects_invalid_current_version() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();

        db.execute("CREATE TABLE _app_meta (version TEXT NOT NULL)".to_string())
            .await
            .unwrap();
        db.execute("INSERT INTO _app_meta (version) VALUES ('invalid')".to_string())
            .await
            .unwrap();

        let version = db.get_version().await.unwrap();
        let error = crate::database::migrate_up_from_version(&db, &version, None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("数据库版本无效"));
    }
}
