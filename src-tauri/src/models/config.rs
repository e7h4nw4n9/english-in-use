use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", content = "details")]
pub enum BookSource {
    Local { path: String },
    CloudflareGateway {},
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", content = "details")]
pub enum DatabaseConnection {
    SQLite { path: String },
    CloudflareGateway {},
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CloudflareGatewayConfig {
    pub base_url: String,
    pub access_token: String,
}

impl CloudflareGatewayConfig {
    /// 校验并规范化网关地址与访问令牌。
    pub fn normalized(&self) -> Result<Self, String> {
        let mut url = reqwest::Url::parse(self.base_url.trim())
            .map_err(|error| format!("Cloudflare 网关地址无效: {error}"))?;
        if !matches!(url.scheme(), "https" | "http") {
            return Err("Cloudflare 网关仅支持 HTTP(S) 地址".to_string());
        }
        let is_loopback = url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
        if url.scheme() != "https" && !is_loopback {
            return Err("Cloudflare 网关必须使用 HTTPS".to_string());
        }
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("Cloudflare 网关地址不能包含用户信息、查询参数或片段".to_string());
        }
        if !url.path().ends_with('/') {
            let normalized_path = format!("{}/", url.path());
            url.set_path(&normalized_path);
        }
        let access_token = self.access_token.trim();
        if access_token.is_empty() {
            return Err("Cloudflare 网关访问令牌不能为空".to_string());
        }
        Ok(Self {
            base_url: url.to_string(),
            access_token: access_token.to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SystemConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_enable_debug_tools")]
    pub enable_debug_tools: bool,
    #[serde(default = "default_enable_auto_check")]
    pub enable_auto_check: bool,
    #[serde(default = "default_check_interval")]
    pub check_interval_mins: u32,
    #[serde(default = "default_auto_start_study_timer")]
    pub auto_start_study_timer: bool,
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
fn default_enable_debug_tools() -> bool {
    false
}
fn default_enable_auto_check() -> bool {
    true
}
fn default_check_interval() -> u32 {
    5
}
fn default_auto_start_study_timer() -> bool {
    false
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "system".to_string(),
            log_level: "info".to_string(),
            enable_debug_tools: false,
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    #[serde(skip, default = "uuid::Uuid::new_v4")]
    pub version: uuid::Uuid,
    #[serde(default)]
    pub system: SystemConfig,
    pub book_source: Option<BookSource>,
    pub database: Option<DatabaseConnection>,
    #[serde(default)]
    pub cloudflare_gateway: Option<CloudflareGatewayConfig>,
    #[serde(default)]
    pub gateway_configuration_required: bool,
}

impl PartialEq for AppConfig {
    fn eq(&self, other: &Self) -> bool {
        self.system == other.system
            && self.book_source == other.book_source
            && self.database == other.database
            && self.cloudflare_gateway == other.cloudflare_gateway
            && self.gateway_configuration_required == other.gateway_configuration_required
    }
}

impl AppConfig {
    /// 创建默认配置。
    pub fn new() -> Self {
        Self::default()
    }

    /// 校验所有配置来源都必须满足的基础约束。
    fn validate_common(&self) -> Result<(), String> {
        if !matches!(self.system.language.as_str(), "en" | "zh") {
            return Err(format!("不支持的语言: {}", self.system.language));
        }
        if !matches!(self.system.theme.as_str(), "system" | "light" | "dark") {
            return Err(format!("不支持的主题: {}", self.system.theme));
        }
        if !matches!(
            self.system.log_level.as_str(),
            "error" | "warn" | "info" | "debug" | "trace"
        ) {
            return Err(format!("不支持的日志级别: {}", self.system.log_level));
        }
        if !(1..=1440).contains(&self.system.check_interval_mins) {
            return Err("自动检查间隔必须在 1 到 1440 分钟之间".to_string());
        }

        if let Some(BookSource::Local { path }) = &self.book_source
            && path.trim().is_empty()
        {
            return Err("本地图书目录不能为空".to_string());
        }

        if let Some(DatabaseConnection::SQLite { path }) = &self.database
            && path.trim().is_empty()
        {
            return Err("SQLite 路径不能为空".to_string());
        }

        Ok(())
    }

    /// 加载配置时只校验不会阻止旧版云配置迁移的公共字段。
    pub(crate) fn validate_common_for_load(&self) -> Result<(), String> {
        self.validate_common()
    }

    /// 校验即将投入运行的完整配置。
    pub fn validate(&self) -> Result<(), String> {
        self.validate_common()?;
        let uses_gateway = matches!(self.book_source, Some(BookSource::CloudflareGateway { .. }))
            || matches!(
                self.database,
                Some(DatabaseConnection::CloudflareGateway { .. })
            );
        if uses_gateway {
            let gateway = self
                .cloudflare_gateway
                .as_ref()
                .ok_or_else(|| "需要配置 Cloudflare 网关".to_string())?;
            gateway.normalized()?;
        }
        Ok(())
    }

    /// 校验导入配置；允许凭据被成对脱敏，保存前仍会执行完整校验。
    pub fn validate_import(&self) -> Result<(), String> {
        self.validate_common()?;
        if let Some(gateway) = &self.cloudflare_gateway
            && gateway.base_url.trim().is_empty()
        {
            return Err("Cloudflare 网关地址不能为空".to_string());
        }
        Ok(())
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
        assert!(!config.system.enable_debug_tools);
        assert!(!config.system.auto_start_study_timer);
    }

    #[test]
    fn test_serialization() {
        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "tmp/books".to_string(),
        });

        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("type = \"Local\""));
        assert!(toml_str.contains("path = \"tmp/books\""));
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
        assert!(!config.system.enable_debug_tools);
        assert_eq!(config.system.check_interval_mins, 5);
        assert!(!config.system.auto_start_study_timer);
    }

    #[test]
    fn test_validation_rejects_zero_check_interval() {
        let mut config = AppConfig::new();
        config.system.check_interval_mins = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_rejects_incomplete_remote_config() {
        let mut config = AppConfig::new();
        config.database = Some(DatabaseConnection::CloudflareGateway {});
        config.cloudflare_gateway = Some(CloudflareGatewayConfig {
            base_url: "https://gateway.example.com".to_string(),
            access_token: String::new(),
        });

        assert!(config.validate().is_err());
    }
}
