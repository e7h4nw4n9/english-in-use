use crate::config::BookSource;
use aws_config::Region;
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider};
use aws_sdk_s3::Client;
use log::{debug, error, info};

pub async fn create_r2_client(source: &BookSource) -> Result<Client, String> {
    if let BookSource::CloudflareR2 {
        account_id,
        access_key_id,
        secret_access_key,
        ..
    } = source
    {
        debug!("正在为账户 {} 创建 R2 客户端", account_id);
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "cloudflare-r2",
        );

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(Region::new("auto"))
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
    info!("正在从存储桶 {} 获取对象: {}", bucket, key);
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| {
            error!("获取 R2 对象失败: {}", e);
            format!("Failed to get object: {}", e)
        })?;

    let data = resp.body.collect().await.map_err(|e| {
        error!("收集 R2 对象数据失败: {}", e);
        format!("Failed to collect body: {}", e)
    })?;

    let bytes = data.into_bytes().to_vec();
    debug!("成功获取对象，大小: {} 字节", bytes.len());
    Ok(bytes)
}
