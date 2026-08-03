//! 图书和练习资源的安全路径、资源根目录与缓存文件辅助逻辑。

use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};

use super::{ERR_PATH_INVALID_RELATIVE, ERR_PATH_OUTSIDE_BASE, coded_error};

/// 将外部提供的相对路径规范化，并拒绝绝对路径和父目录穿越。
///
/// # 参数
/// - `input`：待校验的原始相对路径。
/// - `field_name`：用于错误信息的字段名称。
pub(super) fn normalize_safe_relative_path(
    input: &str,
    field_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(coded_error(
            ERR_PATH_INVALID_RELATIVE,
            format!("{} is empty", field_name),
        ));
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(coded_error(
            ERR_PATH_INVALID_RELATIVE,
            format!("{} must be relative: {}", field_name, trimmed),
        ));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                return Err(coded_error(
                    ERR_PATH_OUTSIDE_BASE,
                    format!(
                        "{} contains parent traversal segment and is rejected: {}",
                        field_name, trimmed
                    ),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(coded_error(
                    ERR_PATH_INVALID_RELATIVE,
                    format!("{} has invalid path component: {}", field_name, trimmed),
                ));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(coded_error(
            ERR_PATH_INVALID_RELATIVE,
            format!(
                "{} becomes empty after normalization: {}",
                field_name, trimmed
            ),
        ));
    }

    Ok(normalized)
}

/// 将平台路径转换为统一使用正斜杠的资源路径。
///
/// # 参数
/// - `path`：需要转换的文件系统路径。
pub(super) fn path_to_slash_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// 获取应用包内可能承载资源的根目录，并去除重复路径。
///
/// # 参数
/// - `app`：用于解析 Tauri 资源目录的应用句柄。
pub(super) fn bundle_resource_roots<R: Runtime>(app: &AppHandle<R>) -> Vec<PathBuf> {
    let Ok(resource_dir) = app.path().resource_dir() else {
        return Vec::new();
    };
    let mut roots = vec![resource_dir.clone(), resource_dir.join("assets")];
    roots.sort();
    roots.dedup();
    roots
}

/// 获取应用包内所有可能的图书资源根目录。
///
/// # 参数
/// - `app`：用于解析 Tauri 资源目录的应用句柄。
pub(super) fn bundle_books_base_paths<R: Runtime>(app: &AppHandle<R>) -> Vec<PathBuf> {
    bundle_resource_roots(app)
        .into_iter()
        .map(|root| root.join("books"))
        .collect()
}

/// 获取应用包内所有可能的课程资源根目录。
///
/// # 参数
/// - `app`：用于解析 Tauri 资源目录的应用句柄。
pub(super) fn bundle_courses_base_paths<R: Runtime>(app: &AppHandle<R>) -> Vec<PathBuf> {
    bundle_resource_roots(app)
        .into_iter()
        .map(|root| root.join("courses"))
        .collect()
}

/// 在保持原有顺序的前提下追加尚未出现的候选路径。
///
/// # 参数
/// - `paths`：需要更新的候选路径集合。
/// - `candidate`：准备追加的候选路径。
pub(super) fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if paths.iter().any(|path| path == &candidate) {
        return;
    }
    paths.push(candidate);
}

/// 判断目标是否为存在且非空的文件，避免复用中断下载留下的空缓存。
///
/// # 参数
/// - `path`：需要检查的文件路径。
pub(super) fn file_exists_and_non_empty(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

/// 创建缺失的父目录并将字节写入目标文件。
///
/// # 参数
/// - `path`：目标文件路径。
/// - `data`：需要写入的完整文件内容。
pub(super) fn write_bytes_to_path(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败 (path: {}): {}", parent.display(), e))?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("目标文件名无效: {}", path.display()))?;
    let temporary_path = path.with_file_name(format!(".{file_name}.{}.part", uuid::Uuid::new_v4()));
    if let Err(error) = std::fs::write(&temporary_path, data) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(format!(
            "写入临时文件失败 (path: {}): {error}",
            temporary_path.display()
        ));
    }
    if let Err(error) = std::fs::rename(&temporary_path, path) {
        let _ = std::fs::remove_file(&temporary_path);
        if file_exists_and_non_empty(path) {
            return Ok(());
        }
        return Err(format!("提交文件失败 (path: {}): {error}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ERR_PATH_OUTSIDE_BASE, normalize_safe_relative_path};

    #[test]
    fn test_normalize_safe_relative_path_rejects_parent_dir() {
        let err = normalize_safe_relative_path("../secret/file", "test.path").unwrap_err();
        assert!(err.contains(ERR_PATH_OUTSIDE_BASE));
    }
}
