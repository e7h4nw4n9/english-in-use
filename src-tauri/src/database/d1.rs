use super::{Database, DatabaseFuture, SqlStatement};
use crate::models::{CloudflareGatewayConfig, ServiceStatus};
use crate::utils::gateway::{BatchMode, GatewayClient};
use anyhow::Result;
use log::debug;
use serde_json::Value;

pub struct D1Database {
    client: GatewayClient,
}

impl D1Database {
    /// 使用已缓存的网关客户端创建 D1 实现。
    pub fn from_client(client: GatewayClient) -> Self {
        Self { client }
    }

    /// 通过统一健康接口检查 D1 状态。
    pub async fn check_status(config: &CloudflareGatewayConfig) -> ServiceStatus {
        let client = match GatewayClient::new(config) {
            Ok(client) => client,
            Err(error) => return ServiceStatus::Disconnected(error),
        };
        match client.health().await {
            Ok(health) if health.database == "connected" => ServiceStatus::Connected,
            Ok(_) => ServiceStatus::Disconnected("D1 网关连接不可用".to_string()),
            Err(error) => ServiceStatus::Disconnected(error),
        }
    }

    /// 执行单条语句并返回首个结果集。
    async fn query_one(&self, mode: BatchMode, statement: SqlStatement) -> Result<Vec<Value>> {
        let mut results = self
            .client
            .batch(mode, &[statement])
            .await
            .map_err(anyhow::Error::msg)?;
        Ok(results
            .pop()
            .map(|result| result.results)
            .unwrap_or_default())
    }
}

impl Database for D1Database {
    fn execute(&self, sql: String) -> DatabaseFuture<'_, ()> {
        self.execute_statement(SqlStatement::new(sql, vec![]))
    }

    fn query(&self, sql: String) -> DatabaseFuture<'_, Vec<Value>> {
        self.query_statement(SqlStatement::new(sql, vec![]))
    }

    fn execute_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, ()> {
        Box::pin(async move {
            self.client
                .batch(BatchMode::Write, &[statement])
                .await
                .map_err(anyhow::Error::msg)?;
            Ok(())
        })
    }

    fn query_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, Vec<Value>> {
        Box::pin(async move { self.query_one(BatchMode::Read, statement).await })
    }

    fn query_write_statement(&self, statement: SqlStatement) -> DatabaseFuture<'_, Vec<Value>> {
        Box::pin(async move { self.query_one(BatchMode::Write, statement).await })
    }

    fn execute_batch(&self, statements: Vec<SqlStatement>) -> DatabaseFuture<'_, ()> {
        Box::pin(async move {
            self.client
                .batch(BatchMode::Write, &statements)
                .await
                .map_err(anyhow::Error::msg)?;
            Ok(())
        })
    }

    fn query_batch(&self, statements: Vec<SqlStatement>) -> DatabaseFuture<'_, Vec<Vec<Value>>> {
        Box::pin(async move {
            Ok(self
                .client
                .batch(BatchMode::Read, &statements)
                .await
                .map_err(anyhow::Error::msg)?
                .into_iter()
                .map(|result| result.results)
                .collect())
        })
    }

    fn get_version(&self) -> DatabaseFuture<'_, String> {
        Box::pin(async move {
            let table_rows = self
                .query_statement(SqlStatement::new(
                    "SELECT count(*) AS count FROM sqlite_master WHERE type='table' AND name='_app_meta'",
                    vec![],
                ))
                .await?;
            let table_exists = table_rows
                .first()
                .and_then(|row| row.get("count"))
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("无法解析 _app_meta 表检查结果"))?
                > 0;
            if !table_exists {
                return Ok("0.0.0".to_string());
            }
            let version_rows = self
                .query_statement(SqlStatement::new(
                    "SELECT version FROM _app_meta LIMIT 1",
                    vec![],
                ))
                .await?;
            let version = version_rows
                .first()
                .and_then(|row| row.get("version"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("_app_meta.version 缺失或格式无效"))?
                .to_string();
            debug!("当前数据库版本 (D1): {version}");
            Ok(version)
        })
    }
}
