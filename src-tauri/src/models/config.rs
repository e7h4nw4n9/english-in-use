use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_enable_auto_check")]
    pub enable_auto_check: bool,
    #[serde(default = "default_check_interval")]
    pub check_interval_mins: u32,
}

fn default_language() -> String {
    "en".to_string()
}
fn default_theme() -> String {
    "system".to_string()
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::new();
        assert_eq!(config.book_source, None);
        assert_eq!(config.database, None);
        assert_eq!(config.system.language, "en");
        assert_eq!(config.system.theme, "system");
        assert_eq!(config.system.log_level, "info");
    }

    #[test]
    fn test_serialization() {
        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "/tmp/books".to_string(),
        });

        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("type = \"Local\""));
        assert!(toml_str.contains("path = \"/tmp/books\""));
    }

    #[test]
    fn test_malformed_toml() {
        let malformed_toml = r#"
            [system]
            language = "en"
            check_interval_mins = "invalid_number" 
        "#;
        let result: Result<AppConfig, _> = toml::from_str(malformed_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_fields_defaults() {
        let toml_str = r#"
            [system]
            language = "zh"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.system.language, "zh");
        assert_eq!(config.system.theme, "system"); // Default from Default impl or serde default?
        // Note: SystemConfig Default impl is used by AppConfig Default.
        // But serde(default) on fields uses the function.
        assert_eq!(config.system.log_level, "info");
        assert_eq!(config.system.check_interval_mins, 5);
    }
}
