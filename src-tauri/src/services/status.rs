use crate::models::{BookSource, ConnectionStatus, DatabaseConnection, ServiceStatus};
use crate::services::config::ConfigState;
use log::{debug, info};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time;

pub async fn run_check(app: &AppHandle) -> ConnectionStatus {
    info!("正在执行全量服务状态检查...");
    let config = {
        let state = app.state::<ConfigState>();
        let config = state.0.read().unwrap();
        config.clone()
    };
    run_check_logic(app, &config).await
}

pub async fn run_check_logic(
    app: &AppHandle,
    config: &crate::models::AppConfig,
) -> ConnectionStatus {
    let app_c1 = app.clone();
    run_check_logic_internal(
        config,
        move |source| {
            let app = app_c1.clone();
            let source = source.clone();
            async move { crate::utils::r2::check_status(&app, &source).await }
        },
        move |db| {
            let db = db.clone();
            async move { crate::database::check_status(&db).await }
        },
    )
    .await
}

pub async fn run_check_logic_internal<FR2, FDB>(
    config: &crate::models::AppConfig,
    check_r2: impl Fn(&BookSource) -> FR2,
    check_db: impl Fn(&DatabaseConnection) -> FDB,
) -> ConnectionStatus
where
    FR2: std::future::Future<Output = ServiceStatus>,
    FDB: std::future::Future<Output = ServiceStatus>,
{
    let mut status = ConnectionStatus {
        r2: ServiceStatus::NotConfigured,
        d1: ServiceStatus::NotConfigured,
    };

    if let Some(source) = &config.book_source {
        status.r2 = check_r2(source).await;
    }

    if let Some(db) = &config.database {
        status.d1 = check_db(db).await;
    }

    debug!("服务状态检查结果: R2: {:?}, D1: {:?}", status.r2, status.d1);
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, BookSource, DatabaseConnection, ServiceStatus};

    #[tokio::test]
    async fn test_run_check_logic_not_configured() {
        let config = AppConfig::default();
        let status = run_check_logic_internal(
            &config,
            |_| async { ServiceStatus::Connected },
            |_| async { ServiceStatus::Connected },
        )
        .await;
        assert_eq!(status.r2, ServiceStatus::NotConfigured);
        assert_eq!(status.d1, ServiceStatus::NotConfigured);
    }

    #[tokio::test]
    async fn test_run_check_logic_configured() {
        let mut config = AppConfig::default();
        config.book_source = Some(BookSource::Local {
            path: "/tmp".to_string(),
        });
        config.database = Some(DatabaseConnection::SQLite {
            path: "/tmp/test.db".to_string(),
        });

        let status = run_check_logic_internal(
            &config,
            |_| async { ServiceStatus::Connected },
            |_| async { ServiceStatus::Disconnected("Error".to_string()) },
        )
        .await;

        assert_eq!(status.r2, ServiceStatus::Connected);
        assert_eq!(status.d1, ServiceStatus::Disconnected("Error".to_string()));
    }
}

pub async fn monitor_connections(app: AppHandle) {
    info!("启动连接状态监控任务");
    loop {
        let config = {
            let state = app.state::<ConfigState>();
            let config = state.0.read().unwrap();
            config.clone()
        };

        let has_r2 = matches!(config.book_source, Some(BookSource::CloudflareR2 { .. }));
        let has_d1 = matches!(
            config.database,
            Some(DatabaseConnection::CloudflareD1 { .. })
        );

        let sleep_duration = if config.system.enable_auto_check && (has_r2 || has_d1) {
            let status = run_check_logic(&app, &config).await;
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
