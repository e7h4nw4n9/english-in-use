use crate::models::BookSource;
use log::{error, info};

#[tauri::command]
pub async fn test_r2_connection(source: BookSource) -> Result<Vec<String>, String> {
    test_r2_connection_internal(source, None).await
}

pub async fn test_r2_connection_internal(
    source: BookSource,
    endpoint_override: Option<String>,
) -> Result<Vec<String>, String> {
    info!("正在测试 Cloudflare R2 连接...");
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client =
                crate::utils::r2::create_r2_client_internal(&source, endpoint_override).await?;
            let result = crate::utils::r2::list_folders(&client, bucket_name).await;
            match &result {
                Ok(_) => info!("Cloudflare R2 连接测试成功"),
                Err(e) => error!("Cloudflare R2 连接测试失败: {}", e),
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
pub async fn list_r2_objects(source: BookSource) -> Result<Vec<String>, String> {
    list_r2_objects_internal(source, None).await
}

pub async fn list_r2_objects_internal(
    source: BookSource,
    endpoint_override: Option<String>,
) -> Result<Vec<String>, String> {
    info!("正在列出 R2 对象...");
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client =
                crate::utils::r2::create_r2_client_internal(&source, endpoint_override).await?;
            let result = crate::utils::r2::list_objects(&client, bucket_name).await;
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
pub async fn read_r2_object(source: BookSource, key: String) -> Result<Vec<u8>, String> {
    read_r2_object_internal(source, key, None).await
}

pub async fn read_r2_object_internal(
    source: BookSource,
    key: String,
    endpoint_override: Option<String>,
) -> Result<Vec<u8>, String> {
    info!("正在读取 R2 对象: {}", key);
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client =
                crate::utils::r2::create_r2_client_internal(&source, endpoint_override).await?;
            let result = crate::utils::r2::get_object(&client, bucket_name, &key).await;
            if let Err(e) = &result {
                error!("读取 R2 对象失败: {}", e);
            }
            result
        }
        _ => {
            error!("无效的 R2 配置类型");
            Err("Invalid config type for R2 read".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_r2_commands_error_on_local() {
        let source = BookSource::Local {
            path: "/tmp".to_string(),
        };
        assert!(
            test_r2_connection_internal(source.clone(), None)
                .await
                .is_err()
        );
        assert!(
            list_r2_objects_internal(source.clone(), None)
                .await
                .is_err()
        );
        assert!(
            read_r2_object_internal(source, "key".to_string(), None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_list_r2_objects_command_mock() {
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
                    <Contents><Key>file1.txt</Key><Size>10</Size></Contents>
                    <KeyCount>1</KeyCount>
                </ListBucketResult>"#,
            )
            .create_async()
            .await;

        let source = BookSource::CloudflareR2 {
            account_id: "id".to_string(),
            bucket_name: "test-bucket".to_string(),
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            public_url: None,
        };

        let result = list_r2_objects_internal(source, Some(url)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "file1.txt");
    }

    #[tokio::test]
    async fn test_test_r2_connection_command_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
                    <CommonPrefixes><Prefix>folder1/</Prefix></CommonPrefixes>
                </ListBucketResult>"#,
            )
            .create_async()
            .await;

        let source = BookSource::CloudflareR2 {
            account_id: "id".to_string(),
            bucket_name: "test-bucket".to_string(),
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            public_url: None,
        };

        let result = test_r2_connection_internal(source, Some(url))
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "folder1");
    }

    #[tokio::test]
    async fn test_read_r2_object_command_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body("content")
            .create_async()
            .await;

        let source = BookSource::CloudflareR2 {
            account_id: "id".to_string(),
            bucket_name: "test-bucket".to_string(),
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            public_url: None,
        };

        let result = read_r2_object_internal(source, "file.txt".to_string(), Some(url))
            .await
            .unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "content");
    }
}
