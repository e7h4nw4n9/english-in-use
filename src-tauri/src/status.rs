use serde::{Deserialize, Serialize};
use crate::config::{AppConfig, BookSource, DatabaseConnection};
use tauri::{AppHandle, Manager, Emitter};
use std::time::Duration;
use tokio::time;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", content = "message")]
pub enum ServiceStatus {
    Connected,
    Disconnected(String),
    NotConfigured,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub r2: ServiceStatus,
    pub d1: ServiceStatus,
}

pub async fn check_r2(source: &BookSource) -> ServiceStatus {
    match source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            match crate::r2::create_r2_client(source).await {
                Ok(client) => {
                    match crate::r2::list_folders(&client, bucket_name).await {
                        Ok(_) => ServiceStatus::Connected,
                        Err(e) => ServiceStatus::Disconnected(e),
                    }
                }
                Err(e) => ServiceStatus::Disconnected(e),
            }
        }
        _ => ServiceStatus::NotConfigured,
    }
}

pub async fn check_d1(connection: &DatabaseConnection) -> ServiceStatus {
    match connection {
        DatabaseConnection::CloudflareD1 {
            account_id,
            database_id,
            api_token,
        } => {
            let url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/d1/database/{}",
                account_id, database_id
            );
            
            let client = reqwest::Client::new();
            match client
                .get(&url)
                .bearer_auth(api_token)
                .send()
                .await {
                Ok(response) => {
                    if response.status().is_success() {
                        ServiceStatus::Connected
                    } else {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        ServiceStatus::Disconnected(format!("D1 connection failed ({}): {}", status, text))
                    }
                }
                Err(e) => ServiceStatus::Disconnected(format!("Request failed: {}", e)),
            }
        }
        _ => ServiceStatus::NotConfigured,
    }
}

pub async fn run_check(app: &AppHandle) -> ConnectionStatus {
    let config_path = app.path().app_config_dir().expect("Could not resolve app config dir").join("config.toml");
    let config = AppConfig::load_from_path(&config_path).unwrap_or_default();

    let mut status = ConnectionStatus {
        r2: ServiceStatus::NotConfigured,
        d1: ServiceStatus::NotConfigured,
    };

    if let Some(source) = &config.book_source {
        status.r2 = check_r2(source).await;
    }

    if let Some(db) = &config.database {
        status.d1 = check_d1(db).await;
    }

    status
}

pub async fn monitor_connections(app: AppHandle) {
    loop {
        let config_path = app.path().app_config_dir().expect("Could not resolve app config dir").join("config.toml");
        let config = AppConfig::load_from_path(&config_path).unwrap_or_default();

        let has_r2 = matches!(config.book_source, Some(BookSource::CloudflareR2 { .. }));
        let has_d1 = matches!(config.database, Some(DatabaseConnection::CloudflareD1 { .. }));

        let sleep_duration = if config.system.enable_auto_check && (has_r2 || has_d1) {
            let status = run_check(&app).await;
            let _ = app.emit("connection-status-update", status);
            Duration::from_secs(config.system.check_interval_mins as u64 * 60)
        } else {
            Duration::from_secs(60) // Check config again after 1 minute
        };

        time::sleep(sleep_duration).await;
    }
}
