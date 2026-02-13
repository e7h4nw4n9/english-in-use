use crate::models::{BookSource, ConnectionStatus, DatabaseConnection, ServiceStatus};
use log::{debug, info};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time;

pub async fn run_check(app: &AppHandle) -> ConnectionStatus {
    info!("正在执行全量服务状态检查...");
    let config = crate::services::config::load(app);
    run_check_logic(&config).await
}

pub async fn run_check_logic(config: &crate::models::AppConfig) -> ConnectionStatus {
    let mut status = ConnectionStatus {
        r2: ServiceStatus::NotConfigured,
        d1: ServiceStatus::NotConfigured,
    };

    if let Some(source) = &config.book_source {
        status.r2 = crate::utils::r2::check_status(source).await;
    }

    if let Some(db) = &config.database {
        status.d1 = crate::database::check_status(db).await;
    }

    debug!("服务状态检查结果: R2: {:?}, D1: {:?}", status.r2, status.d1);
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, BookSource, ServiceStatus};

    #[tokio::test]
    async fn test_run_check_logic_not_configured() {
        let config = AppConfig::default();
        let status = run_check_logic(&config).await;
        assert_eq!(status.r2, ServiceStatus::NotConfigured);
        assert_eq!(status.d1, ServiceStatus::NotConfigured);
    }

    #[tokio::test]
    async fn test_run_check_logic_configured_local() {
        let mut config = AppConfig::default();
        config.book_source = Some(BookSource::Local {
            path: "/tmp".to_string(),
        });
        let status = run_check_logic(&config).await;
        assert_eq!(status.r2, ServiceStatus::NotConfigured); // Local is not R2
    }
}

pub async fn monitor_connections(app: AppHandle) {
    info!("启动连接状态监控任务");
    loop {
        let config = crate::services::config::load(&app);

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
