use crate::models::AppConfig;
use log::{debug, info};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tauri::{AppHandle, Manager};

pub struct ConfigState(pub RwLock<AppConfig>);

pub fn get_config_path_from_context(context: &tauri::Context) -> PathBuf {
    let identifier = &context.config().identifier;
    // This logic mimics tauri's internal resolution for app_config_dir
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let mut path = home;
            path.push("Library/Application Support");
            path.push(identifier);
            return path.join("config.toml");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
            let mut path = appdata;
            path.push(identifier);
            return path.join("config.toml");
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let mut path =
            if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
                config_home
            } else if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                let mut p = home;
                p.push(".config");
                p
            } else {
                PathBuf::from(".")
            };
        path.push(identifier);
        return path.join("config.toml");
    }
    PathBuf::from("config.toml")
}

pub fn load_initial(context: &tauri::Context) -> AppConfig {
    let path = get_config_path_from_context(context);
    AppConfig::load_from_path(&path).unwrap_or_default()
}

pub fn get_config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("Could not resolve app config dir")
        .join("config.toml")
}

pub fn load(app: &AppHandle) -> AppConfig {
    let path = get_config_path(app);
    AppConfig::load_from_path(&path).unwrap_or_default()
}

pub fn save(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = get_config_path(app);
    config.save_to_path(&path)
}

// Extension trait to keep logic separated from data
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
        let config: Self = toml::from_str(&content).map_err(|e| e.to_string())?;
        info!("成功从路径加载配置: {:?}", path);
        Ok(config)
    }

    fn save_to_path(&self, path: &Path) -> Result<(), String> {
        debug!("尝试保存配置到路径: {:?}", path);
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                debug!("创建配置目录: {:?}", parent);
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
        info!("成功保存配置到路径: {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BookSource;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_config() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "/test/path".to_string(),
        });

        config.save_to_path(path).expect("Failed to save config");

        let loaded = AppConfig::load_from_path(path).expect("Failed to load config");
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_load_non_existent() {
        let path = Path::new("/tmp/non_existent_config_12345.toml");
        let config = AppConfig::load_from_path(path).unwrap();
        assert_eq!(config, AppConfig::default());
    }
}
