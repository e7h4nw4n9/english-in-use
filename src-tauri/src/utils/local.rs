use log::{debug, error, info};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use tokio::fs;

const ERR_FORBIDDEN_PROJECT_TEMP: &str = "ERR_FORBIDDEN_PROJECT_TEMP";

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

pub fn ensure_path_not_in_project_temp(path: &Path, field_name: &str) -> Result<(), String> {
    if is_path_in_project_temp(path) {
        return Err(format!(
            "[{}] {} points to forbidden project temp directory: {}",
            ERR_FORBIDDEN_PROJECT_TEMP,
            field_name,
            path.display()
        ));
    }
    Ok(())
}

pub fn is_path_in_project_temp(path: &Path) -> bool {
    let cwd = std::env::current_dir().ok();
    let candidates =
        project_temp_root_candidates_from(cwd.as_deref(), Path::new(env!("CARGO_MANIFEST_DIR")));
    is_path_in_project_temp_with_candidates(path, &candidates)
}

fn is_path_in_project_temp_with_candidates(path: &Path, candidates: &[PathBuf]) -> bool {
    let Ok(canonical_path) = path.canonicalize() else {
        return false;
    };

    candidates
        .iter()
        .filter_map(|candidate| candidate.canonicalize().ok())
        .any(|temp_root| canonical_path == temp_root || canonical_path.starts_with(&temp_root))
}

fn project_temp_root_candidates_from(cwd: Option<&Path>, manifest_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![manifest_dir.join("..").join("temp")];

    if let Some(cwd) = cwd {
        candidates.push(cwd.join("temp"));
        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("temp"));
        }
    }

    candidates
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
    if !dir.exists() {
        fs::create_dir_all(dir).await.map_err(|e| {
            error!("创建本地根目录失败: {}", e);
            format!("Failed to create local base directory: {}", e)
        })?;
    }

    // 确保 key 是安全的相对路径，防止目录穿越和绝对路径写入。
    let local_path = resolve_path_within_base(dir, key, false)?;

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
    let base = PathBuf::from(base_path);
    let path = resolve_path_within_base(&base, relative_path, true)?;

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

fn resolve_path_within_base(
    base_path: &Path,
    relative_path: &str,
    require_exists: bool,
) -> Result<PathBuf, String> {
    let base_canonical = base_path.canonicalize().map_err(|e| {
        format!(
            "[ERR_PATH_INVALID_BASE] Invalid base path {:?}: {}",
            base_path, e
        )
    })?;
    let normalized_relative = normalize_relative_path(relative_path)?;
    let candidate = base_canonical.join(&normalized_relative);

    if require_exists && !candidate.exists() {
        return Err(format!(
            "[ERR_RESOURCE_NOT_FOUND] File not found: {:?}",
            candidate
        ));
    }

    if let Ok(canonical_candidate) = candidate.canonicalize() {
        if !canonical_candidate.starts_with(&base_canonical) {
            return Err(format!(
                "[ERR_PATH_OUTSIDE_BASE] Resolved path escapes base directory (base: {}, path: {})",
                base_canonical.display(),
                canonical_candidate.display()
            ));
        }
    }

    Ok(candidate)
}

fn normalize_relative_path(input: &str) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("[ERR_PATH_INVALID_RELATIVE] Empty relative path".to_string());
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(format!(
            "[ERR_PATH_INVALID_RELATIVE] Absolute path is not allowed: {}",
            trimmed
        ));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                return Err(format!(
                    "[ERR_PATH_OUTSIDE_BASE] Parent segment is not allowed: {}",
                    trimmed
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "[ERR_PATH_INVALID_RELATIVE] Invalid path component in: {}",
                    trimmed
                ));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(format!(
            "[ERR_PATH_INVALID_RELATIVE] Relative path is empty after normalization: {}",
            trimmed
        ));
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;
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

    #[tokio::test]
    async fn test_read_file_rejects_parent_dir_traversal() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().to_string_lossy();
        let result = read_file(&base_path, "../outside.txt").await;
        let err = result.unwrap_err();
        assert!(err.contains("[ERR_PATH_OUTSIDE_BASE]"));
    }

    #[tokio::test]
    async fn test_save_to_dir_rejects_absolute_path() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().to_path_buf();

        let result = save_to_dir(&base_path, "/tmp/test.txt", b"test").await;
        let err = result.unwrap_err();
        assert!(err.contains("[ERR_PATH_INVALID_RELATIVE]"));
    }

    #[test]
    fn test_is_path_in_project_temp_with_candidates_true_for_subdir() {
        let workspace = tempdir().unwrap();
        let manifest_dir = workspace.path().join("src-tauri");
        let project_temp = workspace.path().join("temp");
        let nested_path = project_temp.join("books").join("essgiuebk");

        create_dir_all(&manifest_dir).unwrap();
        create_dir_all(&nested_path).unwrap();

        let candidates = project_temp_root_candidates_from(Some(&manifest_dir), &manifest_dir);
        assert!(is_path_in_project_temp_with_candidates(
            &nested_path,
            &candidates
        ));
    }

    #[test]
    fn test_is_path_in_project_temp_with_candidates_false_for_non_temp_path() {
        let workspace = tempdir().unwrap();
        let manifest_dir = workspace.path().join("src-tauri");
        let non_temp_path = workspace.path().join("assets").join("books");

        create_dir_all(&manifest_dir).unwrap();
        create_dir_all(&non_temp_path).unwrap();

        let candidates = project_temp_root_candidates_from(Some(&manifest_dir), &manifest_dir);
        assert!(!is_path_in_project_temp_with_candidates(
            &non_temp_path,
            &candidates
        ));
    }
}
