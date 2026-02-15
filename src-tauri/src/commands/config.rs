use crate::models::AppConfig;
use crate::services::config::{self, AppConfigExt, ConfigState};
use log::{error, info};
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn load_config(state: State<ConfigState>) -> Result<AppConfig, String> {
    info!("正在从缓存加载配置文件...");
    let config = state.0.read().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: AppConfig,
) -> Result<(), String> {
    info!("正在保存配置文件...");

    // 保存到磁盘
    config::save(&app, &config).map_err(|e| {
        error!("保存配置文件失败: {}", e);
        e
    })?;

    // 更新缓存
    let mut cache = state.0.write().map_err(|e| e.to_string())?;
    *cache = config;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BookSource;
    use tempfile::NamedTempFile;

    #[test]
    fn test_export_import_config() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let mut config = AppConfig::new();
        config.book_source = Some(BookSource::Local {
            path: "/test/path".to_string(),
        });

        // Test export
        export_config(path.clone(), config.clone()).expect("Export failed");

        // Test import
        let imported = import_config(path).expect("Import failed");
        assert_eq!(config, imported);
    }
}
