use crate::database::{self, DbState};
use crate::services::config;
use log::{debug, info};
use std::fs;
use tauri::{AppHandle, Manager};

/// 获取数据库初始化标志文件路径
fn get_init_flag_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    }
    Ok(cache_dir.join(".db_initialized"))
}

/// 将数据库标记为已初始化
pub fn mark_as_initialized(app: &AppHandle) -> Result<(), String> {
    let path = get_init_flag_path(app)?;
    fs::write(&path, b"initialized").map_err(|e| e.to_string())?;
    debug!("已创建数据库初始化标志文件: {:?}", path);
    Ok(())
}

/// 初始化数据库并根据缓存标志执行迁移。返回布尔值表示是否执行了新的迁移。
pub async fn init_database(app: &AppHandle) -> Result<bool, String> {
    let config = config::load(app);
    let db_config = match config.database {
        Some(db_config) => db_config,
        None => {
            debug!("未配置数据库，跳过初始化");
            return Ok(false);
        }
    };

    let init_flag_path = get_init_flag_path(app)?;
    let mut migrated = false;

    // 初始化数据库连接
    let db = database::init(app, &db_config)
        .await
        .map_err(|e| e.to_string())?;

    // 如果标志文件不存在，说明是第一次运行或需要强制初始化/迁移
    if !init_flag_path.exists() {
        info!("检测到数据库未初始化（缓存文件不存在），正在执行迁移...");
        database::migrate_up(db.as_ref(), None)
            .await
            .map_err(|e| e.to_string())?;

        // 创建标志文件
        mark_as_initialized(app)?;
        info!("数据库初始化及迁移完成");
        migrated = true;
    } else {
        debug!("数据库已初始化（缓存文件已存在）");
    }

    // 将数据库句柄存入 DbState 以便全局使用
    let db_state = app.state::<DbState>();
    let mut db_guard = db_state.db.write().await;
    *db_guard = Some(db);

    Ok(migrated)
}
