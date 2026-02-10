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
pub async fn test_r2_connection(source: BookSource) -> Result<String, String> {
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = crate::r2::create_r2_client(&source).await?;
            // Try to list 1 object to verify connection
            client
                .list_objects_v2()
                .bucket(bucket_name)
                .max_keys(1)
                .send()
                .await
                .map_err(|e| format!("R2 connection failed: {}", e))?;
            
            Ok("Connection successful".to_string())
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
pub async fn test_postgresql_connection(connection: DatabaseConnection) -> Result<String, String> {
    match connection {
        DatabaseConnection::PostgreSQL {
            host,
            port,
            user,
            password,
            database,
            ssl,
        } => {
            let mut config = postgres::Config::new();
            config
                .host(&host)
                .port(port)
                .user(&user)
                .dbname(&database);

            if let Some(pwd) = password {
                config.password(&pwd);
            }

            if ssl {
                let connector = native_tls::TlsConnector::new()
                    .map_err(|e| format!("Failed to create TLS connector: {}", e))?;
                let connector = postgres_native_tls::MakeTlsConnector::new(connector);
                config
                    .connect(connector)
                    .map_err(|e| format!("PostgreSQL connection failed: {}", e))?;
            } else {
                config
                    .connect(postgres::NoTls)
                    .map_err(|e| format!("PostgreSQL connection failed: {}", e))?;
            }

            Ok("Database connection successful".to_string())
        }
    }
}

#[tauri::command]
pub fn restart(app: AppHandle) {
    app.restart();
}
