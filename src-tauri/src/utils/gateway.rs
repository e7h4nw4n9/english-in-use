use crate::database::SqlStatement;
use crate::models::CloudflareGatewayConfig;
use crate::utils::keyed_lock::KeyedAsyncLock;
use log::{debug, info};
use moka::future::Cache;
use reqwest::{Client, Response, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchMode {
    Read,
    Write,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayD1Result {
    #[serde(default)]
    pub results: Vec<Value>,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub meta: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayBatchResponse {
    results: Vec<GatewayD1Result>,
    request_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealth {
    pub database: String,
    pub r2: String,
    pub schema_version: Option<String>,
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
struct GatewayListResponse {
    keys: Vec<String>,
    cursor: Option<String>,
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct GatewayErrorEnvelope {
    error: GatewayError,
}

#[derive(Debug, Deserialize)]
struct GatewayError {
    code: String,
    message: String,
    #[serde(rename = "requestId")]
    request_id: String,
}

#[derive(Clone)]
pub struct GatewayClient {
    client: Client,
    base_url: Url,
    access_token: Arc<str>,
    bookmark: Arc<RwLock<Option<String>>>,
    downloads: Arc<KeyedAsyncLock<String>>,
    optional_not_found: Cache<String, ()>,
}

impl GatewayClient {
    /// 使用应用网关配置创建可复用 HTTP 客户端。
    pub fn new(config: &CloudflareGatewayConfig) -> Result<Self, String> {
        let normalized = config.normalized()?;
        let base_url = Url::parse(&normalized.base_url)
            .map_err(|e| format!("Cloudflare 网关地址无效: {e}"))?;
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建网关 HTTP 客户端失败: {e}"))?;
        Ok(Self {
            client,
            base_url,
            access_token: Arc::from(normalized.access_token),
            bookmark: Arc::new(RwLock::new(None)),
            downloads: Arc::new(KeyedAsyncLock::default()),
            optional_not_found: Cache::builder()
                .max_capacity(1024)
                .time_to_live(Duration::from_secs(5 * 60))
                .build(),
        })
    }

    /// 构造网关 API 地址。
    fn endpoint(&self, path: &str) -> Result<Url, String> {
        self.base_url
            .join(path.trim_start_matches('/'))
            .map_err(|e| format!("构造网关地址失败: {e}"))
    }

    /// 解析网关错误，同时避免把令牌或响应正文写入日志。
    async fn response_error(response: Response) -> String {
        let status = response.status();
        match response.json::<GatewayErrorEnvelope>().await {
            Ok(body) => format!(
                "网关请求失败 ({status}, {}, request_id={}): {}",
                body.error.code, body.error.request_id, body.error.message
            ),
            Err(_) => format!("网关请求失败 ({status})"),
        }
    }

    /// 仅保留当前已知的最大 Bookmark，防止旧并发响应覆盖新值。
    async fn update_bookmark(&self, bookmark: Option<&str>) {
        let Some(bookmark) = bookmark.filter(|value| !value.is_empty()) else {
            return;
        };
        let mut current = self.bookmark.write().await;
        if current.as_deref().is_none_or(|value| bookmark > value) {
            *current = Some(bookmark.to_string());
        }
    }

    /// 调用 Worker 执行有序 D1 批处理。
    pub async fn batch(
        &self,
        mode: BatchMode,
        statements: &[SqlStatement],
    ) -> Result<Vec<GatewayD1Result>, String> {
        let started_at = Instant::now();
        let url = self.endpoint("v1/d1/batch")?;
        let bookmark = self.bookmark.read().await.clone();
        let mut request = self
            .client
            .post(url)
            .bearer_auth(self.access_token.as_ref())
            .json(&serde_json::json!({ "mode": mode, "statements": statements }));
        if let Some(bookmark) = bookmark {
            request = request.header("x-d1-bookmark", bookmark);
        }
        let response = request
            .send()
            .await
            .map_err(|e| format!("网关请求失败: {e}"))?;
        if !response.status().is_success() {
            return Err(Self::response_error(response).await);
        }
        let next_bookmark = response
            .headers()
            .get("x-d1-bookmark")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let header_request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = response
            .json::<GatewayBatchResponse>()
            .await
            .map_err(|e| format!("解析网关批处理响应失败: {e}"))?;
        let request_id = header_request_id
            .or_else(|| body.request_id.clone())
            .unwrap_or_else(|| "unknown".to_string());
        if body
            .results
            .iter()
            .any(|result| result.success == Some(false))
        {
            return Err("D1 网关批处理中存在失败语句".to_string());
        }
        self.update_bookmark(next_bookmark.as_deref()).await;
        for (index, result) in body.results.iter().enumerate() {
            if let Some(meta) = &result.meta {
                debug!(
                    "D1 语句元数据: index={}, duration={:?}, rows_read={:?}, rows_written={:?}, region={:?}, primary={:?}",
                    index,
                    meta.get("duration"),
                    meta.get("rows_read"),
                    meta.get("rows_written"),
                    meta.get("served_by_region"),
                    meta.get("served_by_primary")
                );
            }
        }
        debug!(
            "D1 网关批处理完成: statements={}, elapsed_ms={}, request_id={}",
            statements.len(),
            started_at.elapsed().as_millis(),
            request_id
        );
        Ok(body.results)
    }

    /// 检查网关绑定的 D1 与 R2 状态。
    pub async fn health(&self) -> Result<GatewayHealth, String> {
        let started_at = Instant::now();
        let response = self
            .client
            .get(self.endpoint("v1/health")?)
            .bearer_auth(self.access_token.as_ref())
            .send()
            .await
            .map_err(|e| format!("网关健康检查失败: {e}"))?;
        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::SERVICE_UNAVAILABLE
        {
            return Err(Self::response_error(response).await);
        }
        let health: GatewayHealth = response
            .json()
            .await
            .map_err(|e| format!("解析网关健康检查失败: {e}"))?;
        debug!(
            "网关健康检查完成: elapsed_ms={}, schema_version={:?}, request_id={}",
            started_at.elapsed().as_millis(),
            health.schema_version,
            health.request_id
        );
        Ok(health)
    }

    /// 获取对象响应；对象不存在时返回空值。
    pub async fn get_object_response(&self, key: &str) -> Result<Option<Response>, String> {
        let started_at = Instant::now();
        let encoded_key = key
            .trim_start_matches('/')
            .split('/')
            .map(|part| urlencoding::encode(part).into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let response = self
            .client
            .get(self.endpoint(&format!("v1/r2/objects/{encoded_key}"))?)
            .bearer_auth(self.access_token.as_ref())
            .send()
            .await
            .map_err(|e| format!("读取网关对象失败: {e}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(Self::response_error(response).await);
        }
        debug!(
            "R2 网关读取完成: key={}, elapsed_ms={}, cache={}, request_id={}",
            key,
            started_at.elapsed().as_millis(),
            response
                .headers()
                .get("x-gateway-cache")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("UNKNOWN"),
            response
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("unknown")
        );
        Ok(Some(response))
    }

    /// 分页列出指定前缀下的全部对象键。
    pub async fn list_objects(&self, prefix: Option<&str>) -> Result<Vec<String>, String> {
        let mut keys = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut request = self
                .client
                .get(self.endpoint("v1/r2/list")?)
                .bearer_auth(self.access_token.as_ref())
                .query(&[("limit", "1000")]);
            if let Some(prefix) = prefix {
                request = request.query(&[("prefix", prefix)]);
            }
            if let Some(cursor) = cursor.as_deref() {
                request = request.query(&[("cursor", cursor)]);
            }
            let response = request
                .send()
                .await
                .map_err(|e| format!("列出网关对象失败: {e}"))?;
            if !response.status().is_success() {
                return Err(Self::response_error(response).await);
            }
            let page: GatewayListResponse = response
                .json()
                .await
                .map_err(|e| format!("解析网关对象列表失败: {e}"))?;
            keys.extend(page.keys);
            if !page.truncated {
                break;
            }
            cursor = page.cursor;
            if cursor.is_none() {
                return Err("网关返回截断列表但未提供 cursor".to_string());
            }
        }
        Ok(keys)
    }

    /// 获取目标文件对应的 singleflight 锁。
    pub fn download_lock(
        &self,
        key: &str,
        target: &std::path::Path,
    ) -> Result<Arc<Mutex<()>>, String> {
        self.downloads.get(Self::download_lock_key(key, target))
    }

    /// 判断可选对象是否命中过期自动淘汰的 404 缓存。
    pub async fn optional_not_found(&self, key: &str) -> bool {
        self.optional_not_found.get(key).await.is_some()
    }

    /// 记录可选对象的明确 404。
    pub async fn remember_optional_not_found(&self, key: &str) {
        self.optional_not_found.insert(key.to_string(), ()).await;
    }

    fn download_lock_key(key: &str, target: &std::path::Path) -> String {
        format!("{}:{}", target.display(), key)
    }
}

pub struct GatewayClientState {
    client: RwLock<Option<(Uuid, GatewayClient)>>,
}

impl Default for GatewayClientState {
    fn default() -> Self {
        Self {
            client: RwLock::new(None),
        }
    }
}

impl GatewayClientState {
    /// 按配置版本复用网关客户端。
    pub async fn get(
        &self,
        version: Uuid,
        config: &CloudflareGatewayConfig,
    ) -> Result<GatewayClient, String> {
        if let Some((cached_version, client)) = &*self.client.read().await
            && *cached_version == version
        {
            return Ok(client.clone());
        }
        let mut cache = self.client.write().await;
        if let Some((cached_version, client)) = &*cache
            && *cached_version == version
        {
            return Ok(client.clone());
        }
        let client = GatewayClient::new(config)?;
        *cache = Some((version, client.clone()));
        info!("已刷新 Cloudflare 网关客户端");
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::SqlValue;
    use mockito::{Matcher, Server};

    fn config(server: &Server) -> CloudflareGatewayConfig {
        CloudflareGatewayConfig {
            base_url: format!("{}/", server.url()),
            access_token: "token".to_string(),
        }
    }

    #[test]
    fn normalizes_gateway_base_path_and_rejects_insecure_remote_url() {
        let client = GatewayClient::new(&CloudflareGatewayConfig {
            base_url: "https://gateway.example.com/private".to_string(),
            access_token: " token ".to_string(),
        })
        .unwrap();
        assert_eq!(
            client.endpoint("v1/health").unwrap().as_str(),
            "https://gateway.example.com/private/v1/health"
        );

        let error = GatewayClient::new(&CloudflareGatewayConfig {
            base_url: "http://gateway.example.com".to_string(),
            access_token: "token".to_string(),
        })
        .err()
        .unwrap();
        assert!(error.contains("HTTPS"));
    }

    #[tokio::test]
    async fn batch_sends_auth_and_keeps_greatest_bookmark() {
        let mut server = Server::new_async().await;
        let first = server
            .mock("POST", "/v1/d1/batch")
            .match_header("authorization", "Bearer token")
            .match_header("x-d1-bookmark", Matcher::Missing)
            .match_body(Matcher::Json(serde_json::json!({
                "mode": "write",
                "statements": [{ "sql": "SELECT ?", "params": [1] }]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("x-d1-bookmark", "0002-bookmark")
            .with_body(r#"{"results":[{"results":[{"value":1}],"success":true}]}"#)
            .create_async()
            .await;
        let second = server
            .mock("POST", "/v1/d1/batch")
            .match_header("x-d1-bookmark", "0002-bookmark")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("x-d1-bookmark", "0001-bookmark")
            .with_body(r#"{"results":[{"results":[],"success":true}]}"#)
            .create_async()
            .await;
        let client = GatewayClient::new(&config(&server)).unwrap();
        let statement = SqlStatement::new("SELECT ?", vec![SqlValue::Integer(1)]);

        client
            .batch(BatchMode::Write, std::slice::from_ref(&statement))
            .await
            .unwrap();
        client
            .batch(BatchMode::Write, std::slice::from_ref(&statement))
            .await
            .unwrap();

        first.assert_async().await;
        second.assert_async().await;
        assert_eq!(
            client.bookmark.read().await.as_deref(),
            Some("0002-bookmark")
        );
    }
}
