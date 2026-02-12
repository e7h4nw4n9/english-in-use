use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", content = "details")]
pub enum BookSource {
    Local {
        path: String,
    },
    CloudflareR2 {
        account_id: String,
        bucket_name: String,
        access_key_id: String,
        secret_access_key: String,
        public_url: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", content = "details")]
pub enum DatabaseConnection {
    SQLite {
        path: String,
    },
    CloudflareD1 {
        account_id: String,
        database_id: String,
        api_token: String,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SystemConfig {
    pub language: String,
    pub theme: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_enable_auto_check")]
    pub enable_auto_check: bool,
    #[serde(default = "default_check_interval")]
    pub check_interval_mins: u32,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_enable_auto_check() -> bool {
    true
}
fn default_check_interval() -> u32 {
    5
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "system".to_string(),
            log_level: "info".to_string(),
            enable_auto_check: true,
            check_interval_mins: 5,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub system: SystemConfig,
    pub book_source: Option<BookSource>,
    pub database: Option<DatabaseConnection>,
}

impl AppConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
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

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
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
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = AppConfig::new();
        assert_eq!(config.book_source, None);
        assert_eq!(config.database, None);
        assert_eq!(config.system.language, "en");
        assert_eq!(config.system.theme, "system");
    }

    #[test]
    fn test_save_and_load_local_source() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "/tmp/books".to_string(),
        });
        config.system.language = "zh".to_string();

        config.save_to_path(path).expect("Failed to save config");

        let loaded_config = AppConfig::load_from_path(path).expect("Failed to load config");
        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_save_and_load_database_config() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        let mut config = AppConfig::new();

        // Test SQLite
        config.database = Some(DatabaseConnection::SQLite {
            path: "/path/to/db.sqlite".to_string(),
        });
        config.save_to_path(path).expect("Failed to save config");
        let loaded_config = AppConfig::load_from_path(path).expect("Failed to load config");
        assert_eq!(config, loaded_config);

        // Test Cloudflare D1
        config.database = Some(DatabaseConnection::CloudflareD1 {
            account_id: "acc_id".to_string(),
            database_id: "db_id".to_string(),
            api_token: "token".to_string(),
        });
        config.save_to_path(path).expect("Failed to save config");
        let loaded_config = AppConfig::load_from_path(path).expect("Failed to load config");
        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_save_and_load_r2_source() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::CloudflareR2 {
            account_id: "acc_id".to_string(),
            bucket_name: "my-bucket".to_string(),
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            public_url: Some("https://pub.url".to_string()),
        });

        config.save_to_path(path).expect("Failed to save config");

        let loaded_config = AppConfig::load_from_path(path).expect("Failed to load config");
        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_load_non_existent_file() {
        let path = Path::new("/non/existent/path/config.toml");
        let config = AppConfig::load_from_path(path).unwrap();
        assert_eq!(config, AppConfig::default());
    }
}
