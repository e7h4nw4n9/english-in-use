use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::fs;

/// 全局静态变量，用于存储应用数据目录
pub static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
/// 全局静态变量，用于存储应用缓存目录
pub static APP_CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 初始化应用数据目录（由 lib.rs 在启动时调用）
pub fn init_app_data_dir(path: PathBuf) {
    if APP_DATA_DIR.set(path).is_err() {
        debug!("APP_DATA_DIR 已经初始化过");
    }
}

/// 初始化应用缓存目录（由 lib.rs 在启动时调用）
pub fn init_app_cache_dir(path: PathBuf) {
    if APP_CACHE_DIR.set(path).is_err() {
        debug!("APP_CACHE_DIR 已经初始化过");
    }
}

/// 获取应用默认的数据存储目录
pub fn get_app_data_dir() -> Result<&'static PathBuf, String> {
    APP_DATA_DIR
        .get()
        .ok_or_else(|| "应用数据目录未初始化".to_string())
}

/// 获取应用默认的缓存目录
pub fn get_app_cache_dir() -> Result<&'static PathBuf, String> {
    APP_CACHE_DIR
        .get()
        .ok_or_else(|| "应用缓存目录未初始化".to_string())
}

/// 从本地应用数据目录读取文件
pub async fn read_app_file(key: &str) -> Option<Vec<u8>> {
    let data_dir = get_app_data_dir().ok()?;
    let data_dir_str = data_dir.to_string_lossy();
    let safe_key = key.trim_start_matches('/');

    // 重构：直接调用 read_file
    match read_file(&data_dir_str, safe_key).await {
        Ok(bytes) => Some(bytes),
        Err(_) => None,
    }
}

/// 从本地应用缓存目录读取文件
pub async fn read_cache_file(key: &str) -> Option<Vec<u8>> {
    let cache_dir = get_app_cache_dir().ok()?;
    let cache_dir_str = cache_dir.to_string_lossy();
    let safe_key = key.trim_start_matches('/');

    match read_file(&cache_dir_str, safe_key).await {
        Ok(bytes) => Some(bytes),
        Err(_) => None,
    }
}

/// 将文件保存到本地应用数据目录，并保持路径结构
pub async fn save_app_file(key: &str, data: &[u8]) -> Result<String, String> {
    save_to_dir(get_app_data_dir()?, key, data).await
}

/// 将文件保存到本地应用缓存目录，并保持路径结构
pub async fn save_cache_file(key: &str, data: &[u8]) -> Result<String, String> {
    save_to_dir(get_app_cache_dir()?, key, data).await
}

async fn save_to_dir(dir: &PathBuf, key: &str, data: &[u8]) -> Result<String, String> {
    // 确保 key 是相对路径，防止 PathBuf::join 时如果 key 以 / 开头导致直接指向根目录
    let safe_key = key.trim_start_matches('/');
    let local_path = dir.join(safe_key);

    // 确保父目录存在
    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| {
            error!("创建本地目录失败: {}", e);
            format!("Failed to create local directory: {}", e)
        })?;
    }

    // 保存文件到本地
    fs::write(&local_path, data).await.map_err(|e| {
        error!("保存文件到本地失败: {}", e);
        format!("Failed to save file locally: {}", e)
    })?;

    let path_str = local_path.to_string_lossy().to_string();
    info!("文件已成功保存到本地: {}", path_str);
    Ok(path_str)
}

/// 读取本地文件
pub async fn read_file(base_path: &str, relative_path: &str) -> Result<Vec<u8>, String> {
    let mut path = PathBuf::from(base_path);
    path.push(relative_path);

    if !path.exists() {
        debug!("文件不存在: {:?}", path);
        return Err(format!("File not found: {:?}", path));
    }

    info!("正在读取本地文件: {:?}", path);

    fs::read(&path).await.map_err(|e| {
        error!("读取文件失败 ({:?}): {}", path, e);
        format!("Failed to read file: {}", e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_to_dir_and_read_file() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().to_path_buf();
        let relative_path = "subdir/test.txt";
        let content = b"hello world";

        // Test saving
        let saved_path = save_to_dir(&base_path, relative_path, content)
            .await
            .unwrap();
        assert!(PathBuf::from(&saved_path).exists());

        // Test reading
        let read_content = read_file(&base_path.to_string_lossy(), relative_path)
            .await
            .unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_read_non_existent_file() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().to_string_lossy();
        let result = read_file(&base_path, "none.txt").await;
        assert!(result.is_err());
    }
}
