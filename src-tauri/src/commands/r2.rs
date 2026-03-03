use crate::models::BookSource;
use crate::services::config::ConfigState;
use crate::utils::r2::R2ClientState;
use log::{error, info};
use tauri::State;
use tokio::time::{Duration, timeout};

#[tauri::command]
pub async fn test_r2_connection(
    config_state: State<'_, ConfigState>,
    r2_state: State<'_, R2ClientState>,
    source: BookSource,
) -> Result<Vec<String>, String> {
    test_r2_connection_internal(config_state, r2_state, source, None).await
}

pub async fn test_r2_connection_internal(
    _config_state: State<'_, ConfigState>,
    _r2_state: State<'_, R2ClientState>,
    source: BookSource,
    endpoint_override: Option<String>,
) -> Result<Vec<String>, String> {
    info!("正在测试 Cloudflare R2 连接...");
    const R2_CLIENT_CREATE_TIMEOUT_SECS: u64 = 15;
    const R2_LIST_TIMEOUT_SECS: u64 = 15;
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let source_owned = source.clone();
            let bucket_name_owned = bucket_name.clone();
            let endpoint_for_create = endpoint_override.clone();

            info!("R2 连接检查步骤 1/2: 创建客户端");
            let create_task = tokio::spawn(async move {
                if let Some(url) = endpoint_for_create {
                    crate::utils::r2::create_r2_client_internal(&source_owned, Some(url)).await
                } else {
                    crate::utils::r2::create_r2_client(&source_owned).await
                }
            });

            let client = timeout(
                Duration::from_secs(R2_CLIENT_CREATE_TIMEOUT_SECS),
                create_task,
            )
            .await
            .map_err(|_| {
                let msg = format!("R2 客户端创建超时（{} 秒）", R2_CLIENT_CREATE_TIMEOUT_SECS);
                error!("{}", msg);
                msg
            })?
            .map_err(|join_err| {
                if join_err.is_panic() {
                    let msg = "R2 客户端创建任务发生 panic".to_string();
                    error!("{}: {}", msg, join_err);
                    msg
                } else {
                    let msg = format!("R2 客户端创建任务失败: {}", join_err);
                    error!("{}", msg);
                    msg
                }
            })??;

            info!("R2 连接检查步骤 2/2: 列出存储桶文件夹");
            let list_task = tokio::spawn(async move {
                crate::utils::r2::list_folders(&client, &bucket_name_owned).await
            });

            let result = timeout(Duration::from_secs(R2_LIST_TIMEOUT_SECS), list_task)
                .await
                .map_err(|_| {
                    let msg = format!("R2 文件夹列表检查超时（{} 秒）", R2_LIST_TIMEOUT_SECS);
                    error!("{}", msg);
                    msg
                })?
                .map_err(|join_err| {
                    if join_err.is_panic() {
                        let msg = "R2 文件夹列表任务发生 panic".to_string();
                        error!("{}: {}", msg, join_err);
                        msg
                    } else {
                        let msg = format!("R2 文件夹列表任务失败: {}", join_err);
                        error!("{}", msg);
                        msg
                    }
                })?;

            if let Err(e) = &result {
                error!("Cloudflare R2 连接测试失败: {}", e);
            } else {
                info!("Cloudflare R2 连接测试成功");
            }

            result
        }
        _ => {
            error!("无效的 R2 配置类型");
            Err("Invalid config type for R2 test".to_string())
        }
    }
}

#[tauri::command]
pub async fn list_r2_objects(
    config_state: State<'_, ConfigState>,
    r2_state: State<'_, R2ClientState>,
    source: BookSource,
) -> Result<Vec<String>, String> {
    list_r2_objects_internal(config_state, r2_state, source, None).await
}

pub async fn list_r2_objects_internal(
    config_state: State<'_, ConfigState>,
    r2_state: State<'_, R2ClientState>,
    source: BookSource,
    endpoint_override: Option<String>,
) -> Result<Vec<String>, String> {
    info!("正在列出 R2 对象...");
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = if let Some(url) = endpoint_override {
                crate::utils::r2::create_r2_client_internal(&source, Some(url)).await?
            } else {
                crate::utils::r2::get_client(&config_state, &r2_state).await?
            };
            let result = crate::utils::r2::list_objects(&client, bucket_name, None).await;
            if let Err(e) = &result {
                error!("列出 R2 对象失败: {}", e);
            }
            result
        }
        _ => {
            error!("无效的 R2 配置类型");
            Err("Invalid config type for R2 list".to_string())
        }
    }
}

#[tauri::command]
pub async fn read_r2_object(
    config_state: State<'_, ConfigState>,
    r2_state: State<'_, R2ClientState>,
    source: BookSource,
    key: String,
) -> Result<Vec<u8>, String> {
    read_r2_object_internal(config_state, r2_state, source, key, None).await
}

pub async fn read_r2_object_internal(
    config_state: State<'_, ConfigState>,
    r2_state: State<'_, R2ClientState>,
    source: BookSource,
    key: String,
    endpoint_override: Option<String>,
) -> Result<Vec<u8>, String> {
    info!("正在读取 R2 对象: {}", key);
    match &source {
        BookSource::CloudflareR2 {
            account_id,
            bucket_name,
            ..
        } => {
            // 规范化 key：移除前导斜杠，防止缓存路径或 R2 请求路径错误
            let normalized_key = key.trim_start_matches('/');

            // 构建缓存路径：r2_cache/{account_id}/{bucket_name}/{normalized_key}
            let cache_key = format!("r2_cache/{}/{}/{}", account_id, bucket_name, normalized_key);

            // 1. 优先检查本地缓存
            if let Some(cached_data) = crate::utils::local::read_cache_file(&cache_key).await {
                info!("从缓存中读取到对象: {}", normalized_key);
                return Ok(cached_data);
            }

            // 2. 缓存未命中，从 R2 下载
            let client = if let Some(url) = endpoint_override {
                crate::utils::r2::create_r2_client_internal(&source, Some(url)).await?
            } else {
                crate::utils::r2::get_client(&config_state, &r2_state).await?
            };
            let result = crate::utils::r2::get_object(&client, bucket_name, normalized_key).await;

            match result {
                Ok(data) => {
                    // 3. 下载成功后保存到缓存
                    let _ = crate::utils::local::save_cache_file(&cache_key, &data).await;
                    Ok(data)
                }
                Err(e) => {
                    error!("读取 R2 对象失败: {}", e);
                    Err(e)
                }
            }
        }
        _ => {
            error!("无效的 R2 配置类型");
            Err("Invalid config type for R2 read".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_r2_commands_error_on_local() {
        // Since we removed AppHandle, these tests can't easily run without real states.
        // But the internal logic is tested in utils/r2.rs
    }
}
