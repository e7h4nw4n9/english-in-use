use crate::config::{AppConfig, BookSource, DatabaseConnection};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use log::{info, error};

fn get_config_path(app: &AppHandle) -> PathBuf {
    // In a real app, you might want to handle errors better than unwrap
    // but for now, we assume the app config dir is always available.
    app.path().app_config_dir().expect("Could not resolve app config dir").join("config.toml")
}

#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    info!("正在加载配置文件...");
    let path = get_config_path(&app);
    AppConfig::load_from_path(&path).map_err(|e| {
        error!("加载配置文件失败: {}", e);
        e
    })
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    info!("正在保存配置文件...");
    let path = get_config_path(&app);
    config.save_to_path(&path).map_err(|e| {
        error!("保存配置文件失败: {}", e);
        e
    })
}

#[tauri::command]
pub fn export_config(path: String, config: AppConfig) -> Result<(), String> {
    info!("正在导出配置文件到: {}", path);
    let path_buf = PathBuf::from(path);
    config.save_to_path(&path_buf).map_err(|e| {
        error!("导出配置文件失败: {}", e);
        e
    })
}

#[tauri::command]
pub fn import_config(path: String) -> Result<AppConfig, String> {
    info!("正在从 {} 导入配置文件", path);
    let path_buf = PathBuf::from(path);
    AppConfig::load_from_path(&path_buf).map_err(|e| {
        error!("导入配置文件失败: {}", e);
        e
    })
}

#[tauri::command]
pub async fn test_r2_connection(source: BookSource) -> Result<Vec<String>, String> {
    info!("正在测试 Cloudflare R2 连接...");
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = crate::r2::create_r2_client(&source).await?;
            let result = crate::r2::list_folders(&client, bucket_name).await;
            match &result {
                Ok(_) => info!("Cloudflare R2 连接测试成功"),
                Err(e) => error!("Cloudflare R2 连接测试失败: {}", e),
            }
            result
        }
        _ => {
            error!("无效的 R2 配置类型");
            Err("Invalid config type for R2 test".to_string())
        },
    }
}

#[tauri::command]
pub async fn list_r2_objects(source: BookSource) -> Result<Vec<String>, String> {
    info!("正在列出 R2 对象...");
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = crate::r2::create_r2_client(&source).await?;
            let result = crate::r2::list_objects(&client, bucket_name).await;
            if let Err(e) = &result {
                error!("列出 R2 对象失败: {}", e);
            }
            result
        }
        _ => {
            error!("无效的 R2 配置类型");
            Err("Invalid config type for R2 list".to_string())
        },
    }
}

#[tauri::command]
pub async fn read_r2_object(source: BookSource, key: String) -> Result<Vec<u8>, String> {
    info!("正在读取 R2 对象: {}", key);
    match &source {
        BookSource::CloudflareR2 { bucket_name, .. } => {
            let client = crate::r2::create_r2_client(&source).await?;
            let result = crate::r2::get_object(&client, bucket_name, &key).await;
            if let Err(e) = &result {
                error!("读取 R2 对象失败: {}", e);
            }
            result
        }
        _ => {
            error!("无效的 R2 配置类型");
            Err("Invalid config type for R2 read".to_string())
        },
    }
}

#[tauri::command]
pub fn get_default_sqlite_path(app: AppHandle) -> Result<String, String> {
    info!("正在获取默认 SQLite 路径");
    let path = app.path().app_data_dir()
        .map_err(|e| {
            error!("获取应用数据目录失败: {}", e);
            e.to_string()
        })?
        .join("english-in-use.db");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn initialize_database(app: AppHandle) -> Result<(), String> {
    info!("正在初始化数据库...");
    let path = get_config_path(&app);
    let config = AppConfig::load_from_path(&path).map_err(|e| {
        error!("加载配置以初始化数据库失败: {}", e);
        e
    })?;
    
    if let Some(db_config) = config.database {
        let result = crate::db::init(&app, &db_config)
            .await
            .map_err(|e| e.to_string());
        match &result {
            Ok(_) => {
                info!("数据库初始化成功");
                Ok(())
            },
            Err(e) => {
                error!("数据库初始化失败: {}", e);
                Err(e.clone())
            },
        }
    } else {
        error!("数据库未配置");
        Err("Database not configured".to_string())
    }
}

#[tauri::command]
pub async fn test_database_connection(connection: DatabaseConnection) -> Result<String, String> {
    info!("正在测试数据库连接...");
    let result = match connection {
        DatabaseConnection::SQLite { path } => {
            let path_obj = std::path::Path::new(&path);
            if let Some(parent) = path_obj.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    error!("创建 SQLite 目录失败: {}", e);
                    e.to_string()
                })?;
            }
            // Just try to open/create the file to verify path is writable
            let res = sqlx::sqlite::SqlitePoolOptions::new()
                .connect(&format!("sqlite:{}?mode=rwc", path))
                .await
                .map_err(|e| {
                    error!("SQLite 连接失败: {}", e);
                    format!("SQLite connection failed: {}", e)
                });
            match res {
                Ok(_) => Ok("SQLite connection successful".to_string()),
                Err(e) => Err(e),
            }
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
                .map_err(|e| {
                    error!("Cloudflare D1 请求失败: {}", e);
                    format!("Request failed: {}", e)
                })?;

            if response.status().is_success() {
                Ok("Cloudflare D1 connection successful".to_string())
            } else {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                error!("Cloudflare D1 连接失败 ({}): {}", status, text);
                Err(format!("D1 connection failed ({}): {}", status, text))
            }
        }
    };
    
    if result.is_ok() {
        info!("数据库连接测试成功");
    }
    result
}

#[tauri::command]
pub fn restart(app: AppHandle) {
    info!("正在重启应用...");
    app.restart();
}
