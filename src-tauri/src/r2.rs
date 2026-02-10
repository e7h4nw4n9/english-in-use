use aws_config::Region;
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider};
use aws_sdk_s3::Client;
use crate::config::BookSource;

pub async fn create_r2_client(source: &BookSource) -> Result<Client, String> {
    if let BookSource::CloudflareR2 {
        account_id,
        access_key_id,
        secret_access_key,
        ..
    } = source
    {
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

        Ok(Client::from_conf(s3_config))
    } else {
        Err("Invalid BookSource type".to_string())
    }
}

pub async fn list_objects(client: &Client, bucket: &str) -> Result<Vec<String>, String> {
    let resp = client
        .list_objects_v2()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| format!("Failed to list objects: {}", e))?;

    let objects = resp
        .contents()
        .iter()
        .filter_map(|obj| obj.key().map(|k| k.to_string()))
        .collect();

    Ok(objects)
}

pub async fn get_object(client: &Client, bucket: &str, key: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| format!("Failed to get object: {}", e))?;

    let data = resp
        .body
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?;

    Ok(data.into_bytes().to_vec())
}
