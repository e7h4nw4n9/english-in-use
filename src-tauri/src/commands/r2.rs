use crate::models::{BookSource, CloudflareGatewayConfig, ConnectionStatus, ServiceStatus};
use crate::services::config::ConfigState;
use crate::utils::gateway::GatewayClientState;
use log::{error, info};
use tauri::State;
use tokio::time::{Duration, timeout};

#[tauri::command]
/// 使用尚未保存的配置测试统一 Cloudflare 网关。
pub async fn test_cloudflare_gateway(
    gateway: CloudflareGatewayConfig,
) -> Result<ConnectionStatus, String> {
    let client = crate::utils::gateway::GatewayClient::new(&gateway)?;
    let health = timeout(Duration::from_secs(15), client.health())
        .await
        .map_err(|_| "Cloudflare 网关连接检查超时（15 秒）".to_string())??;
    let database = if health.database == "connected" {
        ServiceStatus::Connected
    } else {
        ServiceStatus::Disconnected("D1 网关连接不可用".to_string())
    };
    let r2 = if health.r2 == "connected" {
        ServiceStatus::Connected
    } else {
        ServiceStatus::Disconnected("R2 网关连接不可用".to_string())
    };
    Ok(ConnectionStatus { r2, database })
}

#[tauri::command]
/// 通过当前网关配置列出 R2 对象。
pub async fn list_r2_objects(
    config_state: State<'_, ConfigState>,
    gateway_state: State<'_, GatewayClientState>,
    source: BookSource,
) -> Result<Vec<String>, String> {
    if !matches!(source, BookSource::CloudflareGateway { .. }) {
        return Err("无效的 Cloudflare 网关图书源".to_string());
    }
    info!("正在通过网关列出 R2 对象");
    let client = crate::utils::r2::get_client(&config_state, &gateway_state).await?;
    crate::utils::r2::list_objects(&client, None).await
}

#[tauri::command]
/// 通过当前网关配置读取 R2 对象，并保留本地离线缓存。
pub async fn read_r2_object(
    config_state: State<'_, ConfigState>,
    gateway_state: State<'_, GatewayClientState>,
    source: BookSource,
    key: String,
) -> Result<Vec<u8>, String> {
    if !matches!(source, BookSource::CloudflareGateway { .. }) {
        return Err("无效的 Cloudflare 网关图书源".to_string());
    }
    let normalized_key = key.trim_start_matches('/');
    let cache_key = format!("r2_cache/{normalized_key}");
    if let Some(cached_data) = crate::utils::local::read_cache_file(&cache_key).await {
        return Ok(cached_data);
    }
    let client = crate::utils::r2::get_client(&config_state, &gateway_state).await?;
    match crate::utils::r2::get_object(&client, normalized_key).await {
        Ok(data) => {
            let _ = crate::utils::local::save_cache_file(&cache_key, &data).await;
            Ok(data)
        }
        Err(error) => {
            error!("读取 R2 对象失败: {error}");
            Err(error)
        }
    }
}
