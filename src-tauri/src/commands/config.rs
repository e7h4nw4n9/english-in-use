use crate::models::{AppConfig, DatabaseConnection};
use crate::services::config::{self, AppConfigExt, ConfigState};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_fs::FilePath;

/// 将文件 URL 或普通路径统一解析为文件系统路径。
///
/// # 参数
/// - `path`：目标文件或目录路径。
fn resolve_file_path(path: &str) -> Result<PathBuf, String> {
    let file_path = match FilePath::from_str(path) {
        Ok(file_path) => file_path,
        Err(never) => match never {},
    };
    file_path.into_path().map_err(|e| e.to_string())
}

/// 在 iOS 私有容器路径和标准 /var 路径之间生成别名。
///
/// # 参数
/// - `path`：目标文件或目录路径。
fn swap_private_var_prefix(path: &Path) -> Option<PathBuf> {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix("/private/var/") {
        return Some(PathBuf::from(format!("/var/{}", rest)));
    }
    if let Some(rest) = raw.strip_prefix("/var/") {
        return Some(PathBuf::from(format!("/private/var/{}", rest)));
    }
    None
}

/// 解析真实存在的目录，并兼容 iOS /private/var 路径别名。
///
/// # 参数
/// - `path`：目标文件或目录路径。
fn resolve_existing_dir_with_alias(path: &Path) -> Option<PathBuf> {
    if path.exists() && path.is_dir() {
        return Some(path.to_path_buf());
    }
    if let Some(alias) = swap_private_var_prefix(path)
        && alias.exists()
        && alias.is_dir()
    {
        return Some(alias);
    }
    None
}

#[cfg(target_os = "ios")]
fn is_likely_ios_icloud_path(path: &PathBuf) -> bool {
    let raw = path.to_string_lossy();
    raw.contains("/Mobile Documents/") || raw.contains("com~apple~CloudDocs")
}

fn is_cloud_synced_sqlite_path(path: &str) -> bool {
    let normalized = path.to_lowercase();
    [
        "mobile documents",
        "com~apple~clouddocs",
        "onedrive -",
        "onedrive",
        "dropbox",
        "google drive",
        "googledrive",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

/// 拒绝位于云同步目录中的 SQLite 路径，避免数据库锁和同步损坏。
///
/// # 参数
/// - `config`：需要校验并保存的完整应用配置。
fn validate_sqlite_path_not_cloud(config: &AppConfig) -> Result<(), String> {
    if let Some(DatabaseConnection::SQLite { path }) = &config.database
        && is_cloud_synced_sqlite_path(path)
    {
        return Err(format!("SQLite 路径位于云盘同步目录，已拒绝保存: {}", path));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBookSourceValidation {
    pub ok: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[tauri::command]
/// 读取当前内存中的应用配置。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
pub fn load_config(state: State<ConfigState>) -> Result<AppConfig, String> {
    info!("正在从缓存加载配置文件...");
    let config = state.0.read().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
/// 校验、持久化并更新当前应用配置。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `state`：对应命令使用的共享状态。
/// - `config`：数据库或应用配置。
pub fn save_config(
    app: AppHandle,
    state: State<ConfigState>,
    mut config: AppConfig,
) -> Result<(), String> {
    info!("正在保存配置文件...");
    if let Some(gateway) = &mut config.cloudflare_gateway {
        *gateway = gateway.normalized()?;
    }
    config.validate()?;
    config.version = uuid::Uuid::new_v4();
    config.gateway_configuration_required = false;
    validate_sqlite_path_not_cloud(&config)?;

    let mut cache = state.0.write().map_err(|e| e.to_string())?;
    let old_local_path = match &cache.book_source {
        Some(crate::models::BookSource::Local { path }) => Some(path.clone()),
        _ => None,
    };
    let new_local_path = match &config.book_source {
        Some(crate::models::BookSource::Local { path }) => Some(path.clone()),
        _ => None,
    };
    let local_path_changed = old_local_path != new_local_path;

    if local_path_changed && let Some(path) = &new_local_path {
        app.asset_protocol_scope()
            .allow_directory(path, true)
            .map_err(|e| format!("扩展本地图书资源访问范围失败: {e}"))?;
    }

    if let Err(e) = config::save(&app, &config) {
        if local_path_changed && let Some(path) = &new_local_path {
            let _ = app.asset_protocol_scope().forbid_directory(path, true);
        }
        error!("保存配置文件失败: {}", e);
        return Err(e);
    }

    *cache = config;

    if local_path_changed
        && let Some(path) = old_local_path
        && let Err(e) = app.asset_protocol_scope().forbid_directory(path, true)
    {
        log::warn!("收回旧图书目录访问范围失败: {}", e);
    }

    Ok(())
}

#[tauri::command]
/// 将配置导出到指定文件，并按选择保留或移除密钥。
///
/// # 参数
/// - `path`：目标文件或目录路径。
/// - `config`：数据库或应用配置。
/// - `include_secrets`：导出文件是否保留远程服务密钥。
pub fn export_config(
    path: String,
    mut config: AppConfig,
    include_secrets: bool,
) -> Result<(), String> {
    info!("正在导出配置文件到: {}", path);
    let path_buf = resolve_file_path(&path)?;
    if !include_secrets && let Some(gateway) = &mut config.cloudflare_gateway {
        gateway.access_token.clear();
    }
    config.save_to_path(&path_buf).map_err(|e| {
        error!("导出配置文件失败: {}", e);
        e
    })
}

#[tauri::command]
/// 从 TOML 文件导入并校验应用配置。
///
/// # 参数
/// - `path`：目标文件或目录路径。
pub fn import_config(path: String) -> Result<AppConfig, String> {
    info!("正在从 {} 导入配置文件", path);
    let path_buf = resolve_file_path(&path)?;
    if !path_buf.exists() {
        return Err(format!("配置文件不存在: {}", path_buf.display()));
    }
    if path_buf.is_dir() {
        return Err(format!("配置文件路径不能是目录: {}", path_buf.display()));
    }
    let config = AppConfig::load_from_path(&path_buf).map_err(|e| {
        error!("导入配置文件失败: {}", e);
        e
    })?;
    config.validate_import()?;
    Ok(config)
}

#[tauri::command]
/// 校验本地图书目录结构及可选课程资源。
///
/// # 参数
/// - `path`：目标文件或目录路径。
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
    use crate::models::CloudflareGatewayConfig;
    use crate::models::{BookSource, DatabaseConnection};
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
        export_config(path.clone(), config.clone(), true).expect("Export failed");

        // Test import
        let imported = import_config(path).expect("Import failed");
        assert_eq!(config, imported);
    }

    #[test]
    fn test_export_config_redacts_secrets_by_default() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::CloudflareGateway {});
        config.database = Some(DatabaseConnection::CloudflareGateway {});
        config.cloudflare_gateway = Some(CloudflareGatewayConfig {
            base_url: "https://gateway.example.com".to_string(),
            access_token: "gateway-token".to_string(),
        });

        export_config(path.clone(), config, false).expect("Export failed");
        let exported = import_config(path).expect("Import failed");

        let gateway = exported
            .cloudflare_gateway
            .expect("Expected gateway config");
        assert!(gateway.access_token.is_empty());
    }

    #[test]
    fn test_import_config_accepts_file_url() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "test/path".to_string(),
        });

        export_config(path, config.clone(), true).expect("Export failed");

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

    #[test]
    fn test_is_cloud_synced_sqlite_path_positive_cases() {
        assert!(is_cloud_synced_sqlite_path(
            "/Users/test/Library/Mobile Documents/com~apple~CloudDocs/app.db"
        ));
        assert!(is_cloud_synced_sqlite_path(
            "/Users/test/OneDrive - Work/app.db"
        ));
        assert!(is_cloud_synced_sqlite_path(
            "/Users/test/Google Drive/app.db"
        ));
    }

    #[test]
    fn test_is_cloud_synced_sqlite_path_negative_case() {
        assert!(!is_cloud_synced_sqlite_path(
            "/Users/test/Documents/english-in-use.db"
        ));
    }

    #[test]
    fn test_validate_sqlite_path_not_cloud_rejects_cloud_path() {
        let mut config = AppConfig::new();
        config.database = Some(DatabaseConnection::SQLite {
            path: "/Users/test/Dropbox/english-in-use.db".to_string(),
        });

        let result = validate_sqlite_path_not_cloud(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("云盘同步目录"));
    }

    #[test]
    fn test_validate_sqlite_path_not_cloud_accepts_local_path() {
        let mut config = AppConfig::new();
        config.database = Some(DatabaseConnection::SQLite {
            path: "/Users/test/Documents/english-in-use.db".to_string(),
        });

        let result = validate_sqlite_path_not_cloud(&config);
        assert!(result.is_ok());
    }
}
