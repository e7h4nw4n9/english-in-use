use crate::models::DatabaseConnection;
use log::{error, info};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tauri::{AppHandle, State};
use tauri_plugin_fs::FilePath;
use tokio::time::{Duration, timeout};

fn resolve_file_path(path: &str) -> Result<PathBuf, String> {
    let file_path = match FilePath::from_str(path) {
        Ok(file_path) => file_path,
        Err(never) => match never {},
    };
    file_path.into_path().map_err(|e| e.to_string())
}

fn choose_existing_sqlite_file(dir: &Path) -> Result<Option<PathBuf>, String> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("读取目录失败 (path: {}): {}", dir.display(), e))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if ext == "db" || ext == "sqlite" || ext == "sqlite3" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if candidates.is_empty() {
        return Ok(None);
    }

    candidates.sort();
    if let Some(preferred) = candidates.iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("english-in-use.db"))
            .unwrap_or(false)
    }) {
        return Ok(Some(preferred.clone()));
    }

    Ok(candidates.into_iter().next())
}

fn resolve_sqlite_path_internal(path: &Path) -> Result<PathBuf, String> {
    if path.exists() && path.is_dir() {
        if let Some(existing) = choose_existing_sqlite_file(path)? {
            return Ok(existing);
        }
        return Ok(path.join("english-in-use.db"));
    }

    Ok(path.to_path_buf())
}

#[tauri::command]
pub fn get_default_sqlite_path() -> Result<String, String> {
    info!("正在获取默认 SQLite 路径");
    let path = crate::utils::local::get_app_data_dir()?.join("english-in-use.db");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn resolve_sqlite_path(path: String) -> Result<String, String> {
    let path_buf = resolve_file_path(&path)?;
    let resolved = resolve_sqlite_path_internal(&path_buf)?;
    Ok(resolved.to_string_lossy().to_string())
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
    const DB_CHECK_TIMEOUT_SECS: u64 = 15;

    let status = timeout(
        Duration::from_secs(DB_CHECK_TIMEOUT_SECS),
        crate::database::check_status(&connection),
    )
    .await
    .map_err(|_| {
        let msg = format!("数据库连接检查超时（{} 秒）", DB_CHECK_TIMEOUT_SECS);
        error!("{}", msg);
        msg
    })?;

    match status {
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

    #[test]
    fn test_resolve_sqlite_path_uses_existing_db_file_in_directory() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("existing.sqlite");
        std::fs::write(&existing, b"db").unwrap();

        let resolved = resolve_sqlite_path_internal(dir.path()).unwrap();
        assert_eq!(resolved, existing);
    }

    #[test]
    fn test_resolve_sqlite_path_defaults_to_english_in_use_db_when_directory_has_no_db() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"note").unwrap();

        let resolved = resolve_sqlite_path_internal(dir.path()).unwrap();
        assert_eq!(resolved, dir.path().join("english-in-use.db"));
    }

    #[test]
    fn test_resolve_sqlite_path_keeps_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("custom.db");
        std::fs::write(&file, b"db").unwrap();

        let resolved = resolve_sqlite_path_internal(&file).unwrap();
        assert_eq!(resolved, file);
    }
}
