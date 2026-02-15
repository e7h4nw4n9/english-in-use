use crate::models::DatabaseConnection;
use log::{error, info};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_default_sqlite_path() -> Result<String, String> {
    info!("正在获取默认 SQLite 路径");
    let path = crate::utils::local::get_app_data_dir()?.join("english-in-use.db");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn initialize_database(app: AppHandle) -> Result<bool, String> {
    info!("正在通过命令初始化数据库...");
    crate::services::db_init::init_database(&app).await
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

#[tauri::command]
pub async fn get_migration_versions() -> Result<Vec<String>, String> {
    use crate::database::migrations::MIGRATIONS;
    Ok(MIGRATIONS.iter().map(|m| m.version.to_string()).collect())
}

#[tauri::command]
pub async fn get_current_db_version(
    state: State<'_, crate::database::DbState>,
) -> Result<String, String> {
    let db_guard = state.db.read().await;
    if let Some(db) = db_guard.as_ref() {
        db.get_version().await.map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub async fn execute_migration_up(
    app: AppHandle,
    state: State<'_, crate::database::DbState>,
    target_version: Option<String>,
) -> Result<(), String> {
    let db_guard = state.db.read().await;
    if let Some(db) = db_guard.as_ref() {
        crate::database::migrate_up(db.as_ref(), target_version.as_deref())
            .await
            .map_err(|e| e.to_string())?;

        // 迁移成功后确保标记为已初始化
        let _ = crate::services::db_init::mark_as_initialized();
        Ok(())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub async fn execute_migration_down(
    state: State<'_, crate::database::DbState>,
    target_version: Option<String>,
) -> Result<(), String> {
    let db_guard = state.db.read().await;
    if let Some(db) = db_guard.as_ref() {
        crate::database::migrate_down(db.as_ref(), target_version.as_deref())
            .await
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
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
