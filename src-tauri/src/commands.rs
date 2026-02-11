use crate::config::{AppConfig, BookSource, DatabaseConnection};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn get_config_path(app: &AppHandle) -> PathBuf {
    // In a real app, you might want to handle errors better than unwrap
    // but for now, we assume the app config dir is always available.
    app.path().app_config_dir().expect("Could not resolve app config dir").join("config.toml")
}

#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    let path = get_config_path(&app);
    AppConfig::load_from_path(&path)
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let path = get_config_path(&app);
    config.save_to_path(&path)
}

#[tauri::command]
pub fn export_config(path: String, config: AppConfig) -> Result<(), String> {
    let path = PathBuf::from(path);
    config.save_to_path(&path)
}

#[tauri::command]
pub fn import_config(path: String) -> Result<AppConfig, String> {
    let path = PathBuf::from(path);
    AppConfig::load_from_path(&path)
}

#[tauri::command]
pub async fn test_r2_connection(source: BookSource) -> Result<Vec<String>, String> {
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = crate::r2::create_r2_client(&source).await?;
            crate::r2::list_folders(&client, bucket_name).await
        }
        _ => Err("Invalid config type for R2 test".to_string()),
    }
}

#[tauri::command]
pub async fn list_r2_objects(source: BookSource) -> Result<Vec<String>, String> {
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = crate::r2::create_r2_client(&source).await?;
            crate::r2::list_objects(&client, bucket_name).await
        }
        _ => Err("Invalid config type for R2 list".to_string()),
    }
}

#[tauri::command]
pub async fn read_r2_object(source: BookSource, key: String) -> Result<Vec<u8>, String> {
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = crate::r2::create_r2_client(&source).await?;
            crate::r2::get_object(&client, bucket_name, &key).await
        }
        _ => Err("Invalid config type for R2 read".to_string()),
    }
}

#[tauri::command]
pub fn get_default_sqlite_path(app: AppHandle) -> Result<String, String> {
    let path = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("english-in-use.db");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn test_database_connection(connection: DatabaseConnection) -> Result<String, String> {
    match connection {
        DatabaseConnection::SQLite { path } => {
            let path = std::path::Path::new(&path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            // Just try to open/create the file to verify path is writable
            rusqlite::Connection::open(path)
                .map_err(|e| format!("SQLite connection failed: {}", e))?;
            Ok("SQLite connection successful".to_string())
        }
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
            let response = client
                .get(&url)
                .bearer_auth(api_token)
                .send()
                .await
                .map_err(|e| format!("Request failed: {}", e))?;

            if response.status().is_success() {
                Ok("Cloudflare D1 connection successful".to_string())
            } else {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                Err(format!("D1 connection failed ({}): {}", status, text))
            }
        }
    }
}

#[tauri::command]
pub fn restart(app: AppHandle) {
    app.restart();
}
