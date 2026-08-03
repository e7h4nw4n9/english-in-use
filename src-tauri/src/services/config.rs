use crate::models::{AppConfig, BookSource, DatabaseConnection, SystemConfig};
use log::{debug, info};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tauri::{AppHandle, Manager};

pub struct ConfigState(pub RwLock<AppConfig>);

#[derive(serde::Deserialize)]
struct LegacyAppConfig {
    #[serde(default)]
    system: SystemConfig,
    book_source: Option<LegacyBookSource>,
    database: Option<LegacyDatabaseConnection>,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", content = "details")]
enum LegacyBookSource {
    Local { path: String },
    CloudflareR2 {},
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", content = "details")]
enum LegacyDatabaseConnection {
    SQLite { path: String },
    CloudflareD1 {},
}

/** 将旧版直连配置转换为需要重新填写网关凭据的新配置。 */
fn convert_legacy_config(legacy: LegacyAppConfig) -> AppConfig {
    let mut gateway_configuration_required = false;
    let book_source = legacy.book_source.map(|source| match source {
        LegacyBookSource::Local { path } => BookSource::Local { path },
        LegacyBookSource::CloudflareR2 { .. } => {
            gateway_configuration_required = true;
            BookSource::CloudflareGateway {}
        }
    });
    let database = legacy.database.map(|connection| match connection {
        LegacyDatabaseConnection::SQLite { path } => DatabaseConnection::SQLite { path },
        LegacyDatabaseConnection::CloudflareD1 { .. } => {
            gateway_configuration_required = true;
            DatabaseConnection::CloudflareGateway {}
        }
    });
    AppConfig {
        system: legacy.system,
        book_source,
        database,
        cloudflare_gateway: None,
        gateway_configuration_required,
        ..AppConfig::default()
    }
}

fn build_config_path_from_parts(
    identifier: &str,
    platform: &str,
    home: Option<PathBuf>,
    appdata: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
) -> PathBuf {
    let mut path = match platform {
        "macos" | "ios" => {
            let Some(home) = home else {
                return PathBuf::from("config.toml");
            };
            home.join("Library/Application Support")
        }
        "windows" => {
            let Some(appdata) = appdata else {
                return PathBuf::from("config.toml");
            };
            appdata
        }
        _ => {
            if let Some(config_home) = xdg_config_home {
                config_home
            } else if let Some(home) = home {
                home.join(".config")
            } else {
                PathBuf::from(".")
            }
        }
    };

    path.push(identifier);
    path.join("config.toml")
}

/// 根据 Tauri 上下文解析启动阶段的配置文件路径。
///
/// # 参数
/// - `context`：Tauri 应用上下文。
pub fn get_config_path_from_context(context: &tauri::Context) -> PathBuf {
    let identifier = &context.config().identifier;

    // 这里保持与 Tauri 的 app_config_dir 解析规则一致，并遵循 iOS 平台目录约定。
    let platform = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "other"
    };

    build_config_path_from_parts(
        identifier,
        platform,
        std::env::var_os("HOME").map(PathBuf::from),
        std::env::var_os("APPDATA").map(PathBuf::from),
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
    )
}

/// 加载启动配置，失败时返回默认配置。
///
/// # 参数
/// - `context`：Tauri 应用上下文。
pub fn load_initial(context: &tauri::Context) -> AppConfig {
    let path = get_config_path_from_context(context);
    AppConfig::load_from_path(&path)
        .and_then(|config| {
            config.validate_common_for_load()?;
            Ok(config)
        })
        .unwrap_or_else(|err| {
            log::error!("加载初始配置失败 (path: {}): {}", path.display(), err);
            AppConfig::default()
        })
}

/// 解析运行阶段的应用配置文件路径。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
pub fn get_config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("Could not resolve app config dir")
        .join("config.toml")
}

/// 加载运行阶段配置，失败时返回默认配置。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
pub fn load(app: &AppHandle) -> AppConfig {
    let path = get_config_path(app);
    AppConfig::load_from_path(&path)
        .and_then(|config| {
            config.validate_common_for_load()?;
            Ok(config)
        })
        .unwrap_or_else(|err| {
            log::error!("加载配置失败 (path: {}): {}", path.display(), err);
            AppConfig::default()
        })
}

/// 将已校验配置写入应用配置文件。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `config`：数据库或应用配置。
pub fn save(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = get_config_path(app);
    config.save_to_path(&path)
}

// 通过扩展 trait 将配置文件读写逻辑与状态数据分离。
pub trait AppConfigExt {
    fn load_from_path(path: &Path) -> Result<AppConfig, String>;
    fn save_to_path(&self, path: &Path) -> Result<(), String>;
}

impl AppConfigExt for AppConfig {
    fn load_from_path(path: &Path) -> Result<Self, String> {
        debug!("尝试从路径加载配置: {:?}", path);
        if !path.exists() {
            debug!("配置文件不存在，返回默认配置");
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let config: Self = match toml::from_str(&content) {
            Ok(config) => config,
            Err(current_error) => match toml::from_str::<LegacyAppConfig>(&content) {
                Ok(legacy) => {
                    info!("检测到旧版 Cloudflare 直连配置，需要重新配置私有网关");
                    convert_legacy_config(legacy)
                }
                Err(_) => return Err(current_error.to_string()),
            },
        };
        info!("成功从路径加载配置: {:?}", path);
        Ok(config)
    }

    fn save_to_path(&self, path: &Path) -> Result<(), String> {
        debug!("尝试保存配置到路径: {:?}", path);
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            debug!("创建配置目录: {:?}", parent);
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("配置文件名无效: {}", path.display()))?;
        let temporary_path = parent.join(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary_path).map_err(|e| e.to_string())?;
        if let Err(err) = file.write_all(content.as_bytes()) {
            drop(file);
            let _ = fs::remove_file(&temporary_path);
            return Err(err.to_string());
        }
        if let Err(err) = file.sync_all() {
            drop(file);
            let _ = fs::remove_file(&temporary_path);
            return Err(err.to_string());
        }
        drop(file);

        if let Err(err) = fs::rename(&temporary_path, path) {
            #[cfg(windows)]
            if path.exists() {
                let backup_path =
                    parent.join(format!(".{file_name}.{}.backup", uuid::Uuid::new_v4()));
                fs::rename(path, &backup_path)
                    .map_err(|backup_err| format!("备份旧配置文件失败: {backup_err}"))?;
                match fs::rename(&temporary_path, path) {
                    Ok(()) => {
                        let _ = fs::remove_file(backup_path);
                    }
                    Err(replace_err) => {
                        let _ = fs::rename(&backup_path, path);
                        let _ = fs::remove_file(&temporary_path);
                        return Err(format!("替换配置文件失败: {replace_err}"));
                    }
                }
            } else {
                let _ = fs::remove_file(&temporary_path);
                return Err(format!("原子替换配置文件失败: {err}"));
            }
            #[cfg(not(windows))]
            {
                let _ = fs::remove_file(&temporary_path);
                return Err(format!("原子替换配置文件失败: {err}"));
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
        }
        info!("成功保存配置到路径: {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BookSource;
    use tempfile::{NamedTempFile, tempdir};

    #[test]
    fn test_build_config_path_uses_ios_application_support_convention() {
        let path = build_config_path_from_parts(
            "com.example.englishinuse",
            "ios",
            Some(PathBuf::from("/Users/demo")),
            None,
            None,
        );

        assert_eq!(
            path,
            PathBuf::from(
                "/Users/demo/Library/Application Support/com.example.englishinuse/config.toml"
            )
        );
    }

    #[test]
    fn test_build_config_path_uses_xdg_when_available() {
        let path = build_config_path_from_parts(
            "com.example.englishinuse",
            "other",
            Some(PathBuf::from("/Users/demo")),
            None,
            Some(PathBuf::from("/tmp/xdg")),
        );

        assert_eq!(
            path,
            PathBuf::from("/tmp/xdg/com.example.englishinuse/config.toml")
        );
    }

    #[test]
    fn test_save_and_load_config() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "test/path".to_string(),
        });

        config.save_to_path(path).expect("Failed to save config");

        let loaded = AppConfig::load_from_path(path).expect("Failed to load config");
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_load_non_existent() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("non_existent_config.toml");
        let config = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn test_load_legacy_cloudflare_config_requires_gateway_configuration() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(
            file.path(),
            r#"
                [system]
                language = "zh"

                [book_source]
                type = "CloudflareR2"
                [book_source.details]
                account_id = "account"
                bucket_name = "bucket"
                access_key_id = "access"
                secret_access_key = "secret"

                [database]
                type = "CloudflareD1"
                [database.details]
                account_id = "account"
                database_id = "database"
                api_token = "token"
            "#,
        )
        .unwrap();

        let config = AppConfig::load_from_path(file.path()).unwrap();

        assert_eq!(config.system.language, "zh");
        assert!(matches!(
            config.book_source,
            Some(BookSource::CloudflareGateway {})
        ));
        assert!(matches!(
            config.database,
            Some(DatabaseConnection::CloudflareGateway {})
        ));
        assert!(config.cloudflare_gateway.is_none());
        assert!(config.gateway_configuration_required);
    }
}
