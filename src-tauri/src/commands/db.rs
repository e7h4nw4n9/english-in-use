use crate::models::DatabaseConnection;
use crate::services::config;
use log::{error, info};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_default_sqlite_path(app: AppHandle) -> Result<String, String> {
    info!("正在获取默认 SQLite 路径");
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| {
            error!("获取应用数据目录失败: {}", e);
            e.to_string()
        })?
        .join("english-in-use.db");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn initialize_database(
    app: AppHandle,
    state: State<'_, crate::database::DbState>,
) -> Result<(), String> {
    info!("正在初始化数据库...");
    let config = config::load(&app);

    if let Some(db_config) = config.database {
        let db = crate::database::init(&app, &db_config)
            .await
            .map_err(|e| e.to_string())?;

        let mut db_state = state.db.write().await;
        *db_state = Some(db);

        info!("数据库初始化成功并已存入状态");
        Ok(())
    } else {
        error!("数据库未配置");
        Err("Database not configured".to_string())
    }
}

#[tauri::command]

pub async fn test_database_connection(connection: DatabaseConnection) -> Result<String, String> {
    info!("正在测试数据库连接...");

    use crate::models::ServiceStatus;

    match crate::database::check_status(&connection).await {
        ServiceStatus::Connected => {
            info!("数据库连接测试成功");
            Ok("Connection successful".to_string())
        }

        ServiceStatus::Disconnected(e) => {
            error!("数据库连接失败: {}", e);
            Err(e)
        }

        ServiceStatus::NotConfigured => {
            error!("数据库未配置");
            Err("Database not configured".to_string())
        }

        ServiceStatus::Testing => Ok("Connection test in progress".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_database_connection_command() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let conn = DatabaseConnection::SQLite { path };
        let result = test_database_connection(conn).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Connection successful");
    }

    #[tokio::test]
    async fn test_database_connection_d1_failure() {
        let conn = DatabaseConnection::CloudflareD1 {
            account_id: "".to_string(),
            database_id: "".to_string(),
            api_token: "".to_string(),
        };
        // This should fail because empty strings are invalid for D1 client creation
        let result = test_database_connection(conn).await;
        assert!(result.is_err());
    }
}
