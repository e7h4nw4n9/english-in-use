use crate::config::{AppConfig, BookSource, DatabaseConnection};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
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
    debug!("执行 R2 状态检查...");
    match source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            match crate::r2::create_r2_client(source).await {
                Ok(client) => match crate::r2::list_folders(&client, bucket_name).await {
                    Ok(_) => ServiceStatus::Connected,
                    Err(e) => {
                        error!("R2 状态检查失败: {}", e);
                        ServiceStatus::Disconnected(e)
                    }
                },
                Err(e) => {
                    error!("R2 客户端创建失败 (检查时): {}", e);
                    ServiceStatus::Disconnected(e)
                }
            }
        }
        _ => ServiceStatus::NotConfigured,
    }
}

pub async fn check_d1(connection: &DatabaseConnection) -> ServiceStatus {
    debug!("执行 D1 状态检查...");
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
            match client.get(&url).bearer_auth(api_token).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        ServiceStatus::Connected
                    } else {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        error!("D1 状态检查失败 ({}): {}", status, text);
                        ServiceStatus::Disconnected(format!(
                            "D1 connection failed ({}): {}",
                            status, text
                        ))
                    }
                }
                Err(e) => {
                    error!("D1 状态检查请求失败: {}", e);
                    ServiceStatus::Disconnected(format!("Request failed: {}", e))
                }
            }
        }
        _ => ServiceStatus::NotConfigured,
    }
}

pub async fn run_check(app: &AppHandle) -> ConnectionStatus {
    info!("正在执行全量服务状态检查...");
    let config_path = app
        .path()
        .app_config_dir()
        .expect("Could not resolve app config dir")
        .join("config.toml");
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

    debug!("服务状态检查结果: R2: {:?}, D1: {:?}", status.r2, status.d1);
    status
}

pub async fn monitor_connections(app: AppHandle) {
    info!("启动连接状态监控任务");
    loop {
        let config_path = app
            .path()
            .app_config_dir()
            .expect("Could not resolve app config dir")
            .join("config.toml");
        let config = AppConfig::load_from_path(&config_path).unwrap_or_default();

        let has_r2 = matches!(config.book_source, Some(BookSource::CloudflareR2 { .. }));
        let has_d1 = matches!(
            config.database,
            Some(DatabaseConnection::CloudflareD1 { .. })
        );

        let sleep_duration = if config.system.enable_auto_check && (has_r2 || has_d1) {
            let status = run_check(&app).await;
            let _ = app.emit("connection-status-update", status);
            debug!(
                "下次状态检查将在 {} 分钟后执行",
                config.system.check_interval_mins
            );
            Duration::from_secs(config.system.check_interval_mins as u64 * 60)
        } else {
            Duration::from_secs(60) // Check config again after 1 minute
        };

        time::sleep(sleep_duration).await;
    }
}
