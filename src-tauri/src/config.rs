use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::io::Write;

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
    PostgreSQL {
        host: String,
        port: u16,
        user: String,
        password: Option<String>,
        database: String,
        ssl: bool,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SystemConfig {
    pub language: String,
    pub theme: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "system".to_string(),
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
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
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
        config.database = Some(DatabaseConnection::PostgreSQL {
            host: "localhost".to_string(),
            port: 5432,
            user: "postgres".to_string(),
            password: Some("password".to_string()),
            database: "english_in_use".to_string(),
            ssl: false,
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
