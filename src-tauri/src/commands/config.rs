use crate::models::AppConfig;
use crate::services::config::{self, AppConfigExt, ConfigState};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use tauri::{AppHandle, State};
use tauri_plugin_fs::FilePath;

fn resolve_file_path(path: &str) -> Result<PathBuf, String> {
    let file_path = match FilePath::from_str(path) {
        Ok(file_path) => file_path,
        Err(never) => match never {},
    };
    file_path.into_path().map_err(|e| e.to_string())
}

fn swap_private_var_prefix(path: &PathBuf) -> Option<PathBuf> {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix("/private/var/") {
        return Some(PathBuf::from(format!("/var/{}", rest)));
    }
    if let Some(rest) = raw.strip_prefix("/var/") {
        return Some(PathBuf::from(format!("/private/var/{}", rest)));
    }
    None
}

fn resolve_existing_dir_with_alias(path: &PathBuf) -> Option<PathBuf> {
    if path.exists() && path.is_dir() {
        return Some(path.clone());
    }
    if let Some(alias) = swap_private_var_prefix(path) {
        if alias.exists() && alias.is_dir() {
            return Some(alias);
        }
    }
    None
}

#[cfg(target_os = "ios")]
fn is_likely_ios_icloud_path(path: &PathBuf) -> bool {
    let raw = path.to_string_lossy();
    raw.contains("/Mobile Documents/") || raw.contains("com~apple~CloudDocs")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBookSourceValidation {
    pub ok: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn load_config(state: State<ConfigState>) -> Result<AppConfig, String> {
    info!("正在从缓存加载配置文件...");
    let config = state.0.read().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: AppConfig,
) -> Result<(), String> {
    info!("正在保存配置文件...");

    // 保存到磁盘
    config::save(&app, &config).map_err(|e| {
        error!("保存配置文件失败: {}", e);
        e
    })?;

    // 更新缓存
    let mut cache = state.0.write().map_err(|e| e.to_string())?;
    *cache = config;

    Ok(())
}

#[tauri::command]
pub fn export_config(path: String, config: AppConfig) -> Result<(), String> {
    info!("正在导出配置文件到: {}", path);
    let path_buf = resolve_file_path(&path)?;
    config.save_to_path(&path_buf).map_err(|e| {
        error!("导出配置文件失败: {}", e);
        e
    })
}

#[tauri::command]
pub fn import_config(path: String) -> Result<AppConfig, String> {
    info!("正在从 {} 导入配置文件", path);
    let path_buf = resolve_file_path(&path)?;
    if !path_buf.exists() {
        return Err(format!("配置文件不存在: {}", path_buf.display()));
    }
    if path_buf.is_dir() {
        return Err(format!("配置文件路径不能是目录: {}", path_buf.display()));
    }
    AppConfig::load_from_path(&path_buf).map_err(|e| {
        error!("导入配置文件失败: {}", e);
        e
    })
}

#[tauri::command]
pub fn validate_local_book_source(path: String) -> Result<LocalBookSourceValidation, String> {
    let base_path_input = resolve_file_path(&path)?;
    let base_path = resolve_existing_dir_with_alias(&base_path_input);
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let Some(base_path) = base_path else {
        #[cfg(target_os = "ios")]
        if is_likely_ios_icloud_path(&base_path_input) {
            warnings.push(format!(
                "iPadOS iCloud 目录受系统权限限制，跳过严格目录校验: {}",
                base_path_input.display()
            ));
            return Ok(LocalBookSourceValidation {
                ok: true,
                warnings,
                errors,
            });
        }

        if !base_path_input.exists() {
            errors.push(format!(
                "本地图书根目录不存在: {}",
                base_path_input.display()
            ));
        } else if !base_path_input.is_dir() {
            errors.push(format!(
                "本地图书根路径不是目录: {}",
                base_path_input.display()
            ));
        } else {
            errors.push(format!(
                "本地图书根目录不可访问: {}",
                base_path_input.display()
            ));
        }
        return Ok(LocalBookSourceValidation {
            ok: false,
            warnings,
            errors,
        });
    };

    let books_path = base_path.join("books");
    if !books_path.exists() {
        errors.push(format!("缺少 books 目录: {}", books_path.display()));
    } else if !books_path.is_dir() {
        errors.push(format!("books 路径不是目录: {}", books_path.display()));
    }

    let courses_path = base_path.join("courses");
    if !courses_path.exists() {
        warnings.push(format!(
            "未找到 courses 目录（可先仅阅读，练习可能不可用）: {}",
            courses_path.display()
        ));
    } else if !courses_path.is_dir() {
        warnings.push(format!(
            "courses 路径不是目录（练习可能不可用）: {}",
            courses_path.display()
        ));
    }

    let ok = errors.is_empty();
    Ok(LocalBookSourceValidation {
        ok,
        warnings,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BookSource;
    use tempfile::{NamedTempFile, tempdir};

    #[cfg(unix)]
    fn as_file_url(path: &std::path::Path) -> String {
        format!("file://{}", path.display())
    }

    #[cfg(windows)]
    fn as_file_url(path: &std::path::Path) -> String {
        format!("file:///{}", path.display().to_string().replace('\\', "/"))
    }

    #[test]
    fn test_export_import_config() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "test/path".to_string(),
        });

        // Test export
        export_config(path.clone(), config.clone()).expect("Export failed");

        // Test import
        let imported = import_config(path).expect("Import failed");
        assert_eq!(config, imported);
    }

    #[test]
    fn test_import_config_accepts_file_url() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "test/path".to_string(),
        });

        export_config(path, config.clone()).expect("Export failed");

        let file_url = as_file_url(file.path());
        let imported = import_config(file_url).expect("Import from file URL failed");
        assert_eq!(config, imported);
    }

    #[test]
    fn test_import_config_missing_file_should_error() {
        let file = NamedTempFile::new().unwrap();
        let missing_path = file.path().to_path_buf();
        drop(file);

        let err = import_config(missing_path.to_string_lossy().to_string()).unwrap_err();
        assert!(err.contains("配置文件不存在"));
    }

    #[test]
    fn test_validate_local_book_source_ok_with_warning_for_missing_courses() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("books")).unwrap();

        let result = validate_local_book_source(dir.path().to_string_lossy().to_string()).unwrap();
        assert!(result.ok);
        assert!(result.errors.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_validate_local_book_source_fails_without_books() {
        let dir = tempdir().unwrap();

        let result = validate_local_book_source(dir.path().to_string_lossy().to_string()).unwrap();
        assert!(!result.ok);
        assert!(!result.errors.is_empty());
    }
}
