use crate::utils::gateway::{GatewayClient, GatewayClientState};
use log::debug;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

struct TemporaryFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TemporaryFileGuard {
    /// 创建临时文件清理守卫。
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    /// 标记临时文件已经成功提交。
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// 获取与当前配置版本匹配的网关客户端。
pub async fn get_client(
    config_state: &tauri::State<'_, crate::services::config::ConfigState>,
    gateway_state: &tauri::State<'_, GatewayClientState>,
) -> Result<GatewayClient, String> {
    let (version, gateway) = {
        let config = config_state.0.read().map_err(|e| e.to_string())?;
        (config.version, config.cloudflare_gateway.clone())
    };
    let gateway = gateway.ok_or_else(|| "需要配置 Cloudflare 网关".to_string())?;
    gateway_state.get(version, &gateway).await
}

/// 列出指定前缀下的全部对象键。
pub async fn list_objects(
    client: &GatewayClient,
    prefix: Option<&str>,
) -> Result<Vec<String>, String> {
    let objects = client.list_objects(prefix).await?;
    debug!("网关返回 {} 个对象", objects.len());
    Ok(objects)
}

/// 读取对象的完整字节内容；仅用于元数据和小型资源。
pub async fn get_object(client: &GatewayClient, key: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get_object_response(key)
        .await?
        .ok_or_else(|| format!("OBJECT_NOT_FOUND: {key}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取网关对象响应失败: {e}"))?;
    if bytes.is_empty() {
        return Err(format!("网关对象为空: {key}"));
    }
    Ok(bytes.to_vec())
}

/// 获取可选对象，并短暂记住明确的 404，避免重复探测不存在的 overlay。
pub async fn get_optional_object(
    client: &GatewayClient,
    key: &str,
) -> Result<Option<Vec<u8>>, String> {
    if client.optional_not_found(key).await {
        return Ok(None);
    }
    let Some(response) = client.get_object_response(key).await? else {
        client.remember_optional_not_found(key).await;
        return Ok(None);
    };
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取网关对象响应失败: {e}"))?;
    if bytes.is_empty() {
        return Err(format!("网关对象为空: {key}"));
    }
    Ok(Some(bytes.to_vec()))
}

/// 删除无效的普通缓存文件；其他路径类型返回错误。
async fn remove_invalid_cache_file(path: &Path) -> Result<(), String> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_file() => tokio::fs::remove_file(path)
            .await
            .map_err(|e| format!("删除无效缓存文件失败 (path: {}): {e}", path.display())),
        Ok(_) => Err(format!("缓存目标不是普通文件: {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "检查缓存目标失败 (path: {}): {error}",
            path.display()
        )),
    }
}

/// 流式下载对象，并通过同目录临时文件原子替换目标文件。
pub async fn download_object_to_path(
    client: &GatewayClient,
    key: &str,
    target_path: &Path,
) -> Result<(), String> {
    let singleflight = client.download_lock(key, target_path)?;
    let _guard = singleflight.lock().await;
    if crate::utils::cache::is_non_empty_file(target_path).await {
        return Ok(());
    }
    remove_invalid_cache_file(target_path).await?;

    let parent = target_path
        .parent()
        .ok_or_else(|| format!("下载目标缺少父目录: {}", target_path.display()))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| format!("创建下载目录失败 (path: {}): {e}", parent.display()))?;
    let file_name = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("下载目标文件名无效: {}", target_path.display()))?;
    let temporary_path = parent.join(format!(".{file_name}.{}.part", Uuid::new_v4()));

    let mut temporary_file_guard = TemporaryFileGuard::new(temporary_path.clone());
    let result = async {
        let mut response = client
            .get_object_response(key)
            .await?
            .ok_or_else(|| format!("OBJECT_NOT_FOUND: {key}"))?;
        let mut file = tokio::fs::File::create(&temporary_path)
            .await
            .map_err(|e| format!("创建临时下载文件失败: {e}"))?;
        let mut bytes_written = 0usize;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("读取网关对象流失败: {e}"))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("写入临时下载文件失败: {e}"))?;
            bytes_written += chunk.len();
        }
        if bytes_written == 0 {
            return Err(format!("网关对象为空: {key}"));
        }
        file.flush()
            .await
            .map_err(|e| format!("刷新临时下载文件失败: {e}"))?;
        file.sync_all()
            .await
            .map_err(|e| format!("同步临时下载文件失败: {e}"))?;
        drop(file);
        tokio::fs::rename(&temporary_path, target_path)
            .await
            .map_err(|e| format!("提交下载文件失败: {e}"))
    }
    .await;

    if result.is_ok() {
        temporary_file_guard.disarm();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CloudflareGatewayConfig;
    use mockito::{Matcher, Server};

    fn test_config(server: &Server) -> CloudflareGatewayConfig {
        CloudflareGatewayConfig {
            base_url: format!("{}/", server.url()),
            access_token: "token".to_string(),
        }
    }

    #[tokio::test]
    async fn test_list_objects_follows_cursor() {
        let mut server = Server::new_async().await;
        let first = server
            .mock("GET", "/v1/r2/list")
            .match_header("authorization", "Bearer token")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("limit".into(), "1000".into()),
                Matcher::UrlEncoded("prefix".into(), "books/demo/".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"keys":["books/demo/a"],"cursor":"next","truncated":true}"#)
            .create_async()
            .await;
        let second = server
            .mock("GET", "/v1/r2/list")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("limit".into(), "1000".into()),
                Matcher::UrlEncoded("prefix".into(), "books/demo/".into()),
                Matcher::UrlEncoded("cursor".into(), "next".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"keys":["books/demo/b"],"cursor":null,"truncated":false}"#)
            .create_async()
            .await;
        let client = GatewayClient::new(&test_config(&server)).unwrap();

        let objects = list_objects(&client, Some("books/demo/")).await.unwrap();

        assert_eq!(objects, vec!["books/demo/a", "books/demo/b"]);
        first.assert_async().await;
        second.assert_async().await;
    }

    #[tokio::test]
    async fn test_download_is_atomic_and_reuses_existing_file() {
        let mut server = Server::new_async().await;
        let object = server
            .mock("GET", "/v1/r2/objects/books/demo/page.jpg")
            .match_header("authorization", "Bearer token")
            .with_status(200)
            .with_body("image")
            .expect(1)
            .create_async()
            .await;
        let client = GatewayClient::new(&test_config(&server)).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("page.jpg");
        tokio::fs::write(&target, []).await.unwrap();

        download_object_to_path(&client, "books/demo/page.jpg", &target)
            .await
            .unwrap();
        download_object_to_path(&client, "books/demo/page.jpg", &target)
            .await
            .unwrap();

        assert_eq!(tokio::fs::read(target).await.unwrap(), b"image");
        object.assert_async().await;
    }

    #[tokio::test]
    async fn optional_not_found_cache_is_scoped_to_client() {
        let server = Server::new_async().await;
        let first = GatewayClient::new(&test_config(&server)).unwrap();
        let second = GatewayClient::new(&test_config(&server)).unwrap();

        first
            .remember_optional_not_found("books/demo/overlay.json")
            .await;

        assert!(first.optional_not_found("books/demo/overlay.json").await);
        assert!(!second.optional_not_found("books/demo/overlay.json").await);
    }

    #[tokio::test]
    async fn optional_object_caches_structured_not_found_response() {
        let mut server = Server::new_async().await;
        let object = server
            .mock("GET", "/v1/r2/objects/books/demo/overlay.json")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"error":{"code":"OBJECT_NOT_FOUND","message":"missing","requestId":"request"}}"#,
            )
            .expect(1)
            .create_async()
            .await;
        let client = GatewayClient::new(&test_config(&server)).unwrap();

        assert_eq!(
            get_optional_object(&client, "books/demo/overlay.json")
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            get_optional_object(&client, "books/demo/overlay.json")
                .await
                .unwrap(),
            None
        );
        object.assert_async().await;
    }
}
