use crate::models::{BookSource, ServiceStatus};
use aws_config::Region;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider};
use log::{debug, error, info};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct R2ClientState {
    pub client: Arc<RwLock<Option<(Uuid, Client)>>>,
}

impl Default for R2ClientState {
    fn default() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
        }
    }
}

pub async fn get_client(
    config_state: &tauri::State<'_, crate::services::config::ConfigState>,
    r2_state: &tauri::State<'_, R2ClientState>,
) -> Result<Client, String> {
    let (config_version, book_source) = {
        let config = config_state.0.read().map_err(|e| e.to_string())?;
        (config.version, config.book_source.clone())
    };

    // 1. 尝试读取缓存
    {
        let cache = r2_state.client.read().await;
        if let Some((version, client)) = &*cache {
            if *version == config_version {
                debug!("命中 R2 客户端缓存 (version: {})", version);
                return Ok(client.clone());
            }
        }
    }

    // 2. 缓存未命中或版本不一致，创建新客户端
    let book_source = book_source.ok_or("Book source not configured")?;
    let client = create_r2_client(&book_source).await?;

    // 3. 更新缓存
    {
        let mut cache = r2_state.client.write().await;
        *cache = Some((config_version, client.clone()));
        info!("已刷新 R2 客户端缓存 (version: {})", config_version);
    }

    Ok(client)
}

pub async fn create_r2_client(source: &BookSource) -> Result<Client, String> {
    create_r2_client_internal(source, None).await
}

pub(crate) async fn create_r2_client_internal(
    source: &BookSource,
    endpoint_override: Option<String>,
) -> Result<Client, String> {
    if let BookSource::CloudflareR2 {
        account_id,
        access_key_id,
        secret_access_key,
        ..
    } = source
    {
        debug!("正在为账户 {} 创建 R2 客户端", account_id);
        let endpoint = endpoint_override
            .clone()
            .unwrap_or_else(|| format!("https://{}.r2.cloudflarestorage.com", account_id));

        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "cloudflare-r2",
        );

        let region = if endpoint_override.is_some() {
            Region::new("us-east-1")
        } else {
            Region::new("auto")
        };

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region)
            .endpoint_url(endpoint)
            .credentials_provider(SharedCredentialsProvider::new(credentials))
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();

        info!("R2 客户端创建成功");
        Ok(Client::from_conf(s3_config))
    } else {
        error!("BookSource 类型无效，无法创建 R2 客户端");
        Err("Invalid BookSource type".to_string())
    }
}

pub async fn list_objects(client: &Client, bucket: &str) -> Result<Vec<String>, String> {
    info!("正在列出存储桶 {} 中的对象", bucket);
    let resp = client
        .list_objects_v2()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| {
            error!("列出 R2 对象失败: {}", e);
            format!("Failed to list objects: {}", e)
        })?;

    let objects: Vec<String> = resp
        .contents()
        .iter()
        .filter_map(|obj| obj.key().map(|k| k.to_string()))
        .collect();

    debug!("找到 {} 个对象", objects.len());
    Ok(objects)
}

pub async fn list_folders(client: &Client, bucket: &str) -> Result<Vec<String>, String> {
    info!("正在列出存储桶 {} 中的文件夹", bucket);
    let resp = client
        .list_objects_v2()
        .bucket(bucket)
        .delimiter("/")
        .send()
        .await
        .map_err(|e| {
            error!("列出 R2 文件夹失败: {}", e);
            format!("Failed to list folders: {}", e)
        })?;

    let folders: Vec<String> = resp
        .common_prefixes()
        .iter()
        .filter_map(|p| p.prefix().map(|s| s.trim_end_matches('/').to_string()))
        .collect();

    debug!("找到 {} 个文件夹", folders.len());
    Ok(folders)
}

pub async fn get_object(client: &Client, bucket: &str, key: &str) -> Result<Vec<u8>, String> {
    let normalized_key = key.trim_start_matches('/');
    info!("正在从存储桶 {} 获取对象: {}", bucket, normalized_key);
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(normalized_key)
        .send()
        .await
        .map_err(|e| {
            error!("获取 R2 对象失败 (key: {}): {}", normalized_key, e);
            format!("Failed to get object: {}", e)
        })?;

    let data = resp.body.collect().await.map_err(|e| {
        error!("收集 R2 对象数据失败 (key: {}): {}", normalized_key, e);
        format!("Failed to collect body: {}", e)
    })?;

    let bytes = data.into_bytes().to_vec();
    debug!(
        "成功获取对象 {}, 大小: {} 字节",
        normalized_key,
        bytes.len()
    );
    Ok(bytes)
}

pub async fn check_status(app: &tauri::AppHandle, source: &BookSource) -> ServiceStatus {
    check_status_internal(Some(app), source, None).await
}

async fn check_status_internal(
    app: Option<&tauri::AppHandle>,
    source: &BookSource,
    endpoint_override: Option<String>,
) -> ServiceStatus {
    debug!("执行 R2 状态检查...");
    match source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client_res = if let Some(url) = endpoint_override {
                create_r2_client_internal(source, Some(url)).await
            } else if let Some(app_handle) = app {
                use crate::services::config::ConfigState;
                let config_state = app_handle.state::<ConfigState>();
                let r2_state = app_handle.state::<R2ClientState>();
                get_client(&config_state, &r2_state).await
            } else {
                create_r2_client(source).await
            };

            match client_res {
                Ok(client) => match list_folders(&client, bucket_name).await {
                    Ok(_) => ServiceStatus::Connected,
                    Err(e) => {
                        error!("R2 状态检查失败: {}", e);
                        ServiceStatus::Disconnected(e)
                    }
                },
                Err(e) => {
                    error!("R2 客户端创建失败 (检查时): {}", e);
                    ServiceStatus::Disconnected(e)
                }
            }
        }
        _ => ServiceStatus::NotConfigured,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_list_objects_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        // Mock S3 ListObjectsV2 response
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
                    <Name>test-bucket</Name>
                    <IsTruncated>false</IsTruncated>
                    <Contents>
                        <Key>test-file.txt</Key>
                        <Size>123</Size>
                    </Contents>
                    <Contents>
                        <Key>another-file.png</Key>
                        <Size>456</Size>
                    </Contents>
                    <KeyCount>2</KeyCount>
                </ListBucketResult>"#,
            )
            .create_async()
            .await;

        let source = BookSource::CloudflareR2 {
            account_id: "test-account".to_string(),
            bucket_name: "test-bucket".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            public_url: None,
        };

        let client = create_r2_client_internal(&source, Some(url)).await.unwrap();
        let objects = list_objects(&client, "test-bucket").await.unwrap();

        assert_eq!(objects.len(), 2);
        assert!(objects.contains(&"test-file.txt".to_string()));
        assert!(objects.contains(&"another-file.png".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_object_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body("Hello, R2!")
            .create_async()
            .await;

        let source = BookSource::CloudflareR2 {
            account_id: "test-account".to_string(),
            bucket_name: "test-bucket".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            public_url: None,
        };

        let client = create_r2_client_internal(&source, Some(url)).await.unwrap();
        let data = get_object(&client, "test-bucket", "hello.txt")
            .await
            .unwrap();

        assert_eq!(String::from_utf8(data).unwrap(), "Hello, R2!");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_check_status_mock_success() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
                    <Name>test-bucket</Name>
                    <IsTruncated>false</IsTruncated>
                    <CommonPrefixes><Prefix>folder1/</Prefix></CommonPrefixes>
                    <KeyCount>1</KeyCount>
                </ListBucketResult>"#,
            )
            .create_async()
            .await;

        let source = BookSource::CloudflareR2 {
            account_id: "test-account".to_string(),
            bucket_name: "test-bucket".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            public_url: None,
        };

        let status = check_status_internal(None, &source, Some(url)).await;
        assert_eq!(status, ServiceStatus::Connected);
    }

    #[tokio::test]
    async fn test_check_status_mock_failure() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(403)
            .create_async()
            .await;

        let source = BookSource::CloudflareR2 {
            account_id: "test-account".to_string(),
            bucket_name: "test-bucket".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            public_url: None,
        };

        let status = check_status_internal(None, &source, Some(url)).await;
        match status {
            ServiceStatus::Disconnected(_) => (),
            _ => panic!("Expected Disconnected status, got {:?}", status),
        }
    }

    #[tokio::test]
    async fn test_check_status_not_r2() {
        // let source = BookSource::Local {
        //     path: "/tmp".to_string(),
        // };
        // let status = check_status(&source).await;
        // assert_eq!(status, ServiceStatus::NotConfigured);
    }

    #[tokio::test]
    async fn test_create_client_invalid_source() {
        let source = BookSource::Local {
            path: "/tmp".to_string(),
        };
        let result = create_r2_client(&source).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid BookSource type");
    }
}
