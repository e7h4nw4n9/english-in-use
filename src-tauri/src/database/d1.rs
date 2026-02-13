use super::Database;
use crate::models::ServiceStatus;
use anyhow::Result;
use log::{debug, error, info};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";

#[derive(Debug, Deserialize)]
pub struct D1Response {
    pub result: Option<Vec<D1Result>>,
    pub success: bool,
    pub errors: Option<Vec<D1Error>>,
}

#[derive(Debug, Deserialize)]
pub struct D1Result {
    pub results: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct D1Error {
    pub message: String,
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

    pub async fn check_status(
        account_id: &str,
        database_id: &str,
        api_token: &str,
    ) -> ServiceStatus {
        debug!("执行 D1 状态检查...");
        let url = format!(
            "{}/accounts/{}/d1/database/{}",
            CLOUDFLARE_API_BASE, account_id, database_id
        );

        let client = reqwest::Client::new();
        match client.get(&url).bearer_auth(api_token).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    ServiceStatus::Connected
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    error!("D1 状态检查失败 ({}): {}", status, text);
                    ServiceStatus::Disconnected(format!(
                        "D1 connection failed ({}): {}",
                        status, text
                    ))
                }
            }
            Err(e) => {
                error!("D1 状态检查请求失败: {}", e);
                ServiceStatus::Disconnected(format!("Request failed: {}", e))
            }
        }
    }

    async fn raw_query(&self, sql: &str) -> Result<D1Response> {
        debug!("向 D1 发送查询请求: {}", sql);
        let url = format!(
            "{}/accounts/{}/d1/database/{}/query",
            CLOUDFLARE_API_BASE, self.account_id, self.database_id
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

    fn query(
        &self,
        sql: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Value>>> + Send + '_>> {
        Box::pin(async move {
            let res = self.raw_query(&sql).await?;
            let results = res
                .result
                .map(|r| r.into_iter().flat_map(|dr| dr.results).collect())
                .unwrap_or_default();
            Ok(results)
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
