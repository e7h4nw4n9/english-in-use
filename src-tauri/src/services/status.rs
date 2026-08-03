use crate::models::{BookSource, ConnectionStatus, DatabaseConnection, ServiceStatus};
use crate::services::config::ConfigState;
use log::{debug, info};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time;

/// 读取当前配置并检查所有已配置服务的连接状态。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
pub async fn run_check(app: &AppHandle) -> ConnectionStatus {
    info!("正在执行全量服务状态检查...");
    let config = {
        let state = app.state::<ConfigState>();
        let config = state
            .0
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        config.clone()
    };
    run_check_logic(app, &config).await
}

/// 使用真实检查器计算当前服务连接状态。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `config`：数据库或应用配置。
pub async fn run_check_logic(
    app: &AppHandle,
    config: &crate::models::AppConfig,
) -> ConnectionStatus {
    let app_for_gateway = app.clone();
    let gateway = config.cloudflare_gateway.clone();
    let version = config.version;
    run_check_logic_internal(
        config,
        move || {
            let app = app_for_gateway.clone();
            let gateway = gateway.clone();
            async move {
                let gateway = gateway.ok_or_else(|| "需要配置 Cloudflare 网关".to_string())?;
                let state = app.state::<crate::utils::gateway::GatewayClientState>();
                state.get(version, &gateway).await?.health().await
            }
        },
        move |db| {
            let db = db.clone();
            async move { crate::database::check_status(&db, None).await }
        },
    )
    .await
}

/// 使用注入的检查器计算服务连接状态。
///
/// # 参数
/// - `config`：数据库或应用配置。
/// - `check_gateway`：用于检查统一网关状态的异步函数。
/// - `check_db`：用于检查数据库状态的异步函数。
pub async fn run_check_logic_internal<FG, FDB>(
    config: &crate::models::AppConfig,
    check_gateway: impl Fn() -> FG,
    check_db: impl Fn(&DatabaseConnection) -> FDB,
) -> ConnectionStatus
where
    FG: std::future::Future<Output = Result<crate::utils::gateway::GatewayHealth, String>>,
    FDB: std::future::Future<Output = ServiceStatus>,
{
    let uses_gateway_books = matches!(
        config.book_source,
        Some(BookSource::CloudflareGateway { .. })
    );
    let uses_gateway_database = matches!(
        config.database,
        Some(DatabaseConnection::CloudflareGateway { .. })
    );
    let gateway_check = async {
        if !uses_gateway_books && !uses_gateway_database {
            None
        } else if config.cloudflare_gateway.is_none() {
            Some(Err("需要配置 Cloudflare 网关".to_string()))
        } else {
            Some(check_gateway().await)
        }
    };
    let local_database_check = async {
        match &config.database {
            Some(database @ DatabaseConnection::SQLite { .. }) => check_db(database).await,
            None => ServiceStatus::NotConfigured,
            Some(DatabaseConnection::CloudflareGateway { .. }) => ServiceStatus::NotConfigured,
        }
    };
    let (gateway_result, local_database) = tokio::join!(gateway_check, local_database_check);
    let gateway_status = |service: &str, unavailable_message: &str| match &gateway_result {
        Some(Ok(health)) if service == "r2" && health.r2 == "connected" => ServiceStatus::Connected,
        Some(Ok(health)) if service == "database" && health.database == "connected" => {
            ServiceStatus::Connected
        }
        Some(Ok(_)) => ServiceStatus::Disconnected(unavailable_message.to_string()),
        Some(Err(_)) if config.cloudflare_gateway.is_none() => ServiceStatus::NotConfigured,
        Some(Err(error)) => ServiceStatus::Disconnected(error.clone()),
        None => ServiceStatus::NotConfigured,
    };
    let r2 = if uses_gateway_books {
        gateway_status("r2", "R2 网关连接不可用")
    } else {
        ServiceStatus::NotConfigured
    };
    let database = if uses_gateway_database {
        gateway_status("database", "D1 网关连接不可用")
    } else {
        local_database
    };
    let status = ConnectionStatus { r2, database };

    debug!(
        "服务状态检查结果: R2: {:?}, Database: {:?}",
        status.r2, status.database
    );
    status
}

/// 按配置间隔持续检查连接并向前端发送状态事件。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
pub async fn monitor_connections(app: AppHandle) {
    info!("启动连接状态监控任务");
    loop {
        let config = {
            let state = app.state::<ConfigState>();
            let config = state
                .0
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            config.clone()
        };

        let has_r2 = matches!(
            config.book_source,
            Some(BookSource::CloudflareGateway { .. })
        );
        let has_d1 = matches!(
            config.database,
            Some(DatabaseConnection::CloudflareGateway { .. })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, BookSource, DatabaseConnection, ServiceStatus};

    #[tokio::test]
    async fn test_run_check_logic_not_configured() {
        let config = AppConfig::default();
        let status = run_check_logic_internal(
            &config,
            || async { panic!("不应检查网关") },
            |_| async { ServiceStatus::Connected },
        )
        .await;
        assert_eq!(status.r2, ServiceStatus::NotConfigured);
        assert_eq!(status.database, ServiceStatus::NotConfigured);
    }

    #[tokio::test]
    async fn test_run_check_logic_local_database() {
        let config = AppConfig {
            book_source: Some(BookSource::Local {
                path: "tmp".to_string(),
            }),
            database: Some(DatabaseConnection::SQLite {
                path: "tmp/test.db".to_string(),
            }),
            ..AppConfig::default()
        };

        let status = run_check_logic_internal(
            &config,
            || async { panic!("不应检查网关") },
            |_| async { ServiceStatus::Disconnected("Error".to_string()) },
        )
        .await;

        assert_eq!(status.r2, ServiceStatus::NotConfigured);
        assert_eq!(
            status.database,
            ServiceStatus::Disconnected("Error".to_string())
        );
    }

    #[tokio::test]
    async fn test_run_check_logic_mixed_gateway_books_and_sqlite_database() {
        let config = AppConfig {
            book_source: Some(BookSource::CloudflareGateway {}),
            database: Some(DatabaseConnection::SQLite {
                path: "tmp/test.db".to_string(),
            }),
            cloudflare_gateway: Some(crate::models::CloudflareGatewayConfig {
                base_url: "https://gateway.example.com/".to_string(),
                access_token: "token".to_string(),
            }),
            ..AppConfig::default()
        };
        let status = run_check_logic_internal(
            &config,
            || async {
                Ok(crate::utils::gateway::GatewayHealth {
                    database: "disconnected".to_string(),
                    r2: "connected".to_string(),
                    schema_version: None,
                    request_id: "request".to_string(),
                })
            },
            |_| async { ServiceStatus::Connected },
        )
        .await;

        assert_eq!(status.r2, ServiceStatus::Connected);
        assert_eq!(status.database, ServiceStatus::Connected);
    }

    #[tokio::test]
    async fn test_missing_gateway_only_affects_selected_cloud_service() {
        let config = AppConfig {
            book_source: Some(BookSource::Local {
                path: "tmp".to_string(),
            }),
            database: Some(DatabaseConnection::CloudflareGateway {}),
            ..AppConfig::default()
        };
        let status = run_check_logic_internal(
            &config,
            || async { panic!("缺少配置时不应检查网关") },
            |_| async { panic!("不应检查 SQLite") },
        )
        .await;

        assert_eq!(status.r2, ServiceStatus::NotConfigured);
        assert_eq!(status.database, ServiceStatus::NotConfigured);
    }
}
