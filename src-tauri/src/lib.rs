pub mod commands;
pub mod database;
pub mod models;
pub mod services;
pub mod utils;

use tauri::Emitter;
use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn check_connection_status(
    app: tauri::AppHandle,
) -> Result<models::ConnectionStatus, String> {
    Ok(services::status::run_check(&app).await)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    // 加载初始配置
    let initial_config = services::config::load_initial(&context);
    let log_level = std::str::FromStr::from_str(&initial_config.system.log_level)
        .unwrap_or(log::LevelFilter::Info);

    let config_state = services::config::ConfigState(std::sync::RwLock::new(initial_config));

    let book_cache = commands::books::BookCacheState {
        cache: moka::future::Cache::builder()
            .time_to_idle(std::time::Duration::from_secs(5 * 60)) // 5分钟滑动窗口
            .build(),
    };

    tauri::Builder::default()
        .manage(config_state)
        .manage(database::DbState::default())
        .manage(book_cache)
        .manage(utils::r2::R2ClientState::default())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("app".to_string()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .level(log_level)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            // 初始化全局应用数据目录常量
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");
            crate::utils::local::init_app_data_dir(app_data_dir);

            // 初始化全局应用缓存目录常量
            let app_cache_dir = app
                .path()
                .app_cache_dir()
                .expect("Failed to resolve app cache directory");
            crate::utils::local::init_app_cache_dir(app_cache_dir);

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 初始化数据库
                if let Err(e) = services::db_init::init_database(&handle).await {
                    log::error!("数据库初始化失败: {}", e);
                }

                services::status::monitor_connections(handle).await;
            });

            let handle = app.handle();

            let settings_item =
                MenuItem::with_id(handle, "settings", "Settings...", true, Some("CmdOrCtrl+,"))?;
            let quit_item = PredefinedMenuItem::quit(handle, None)?;

            let app_submenu = Submenu::with_items(
                handle,
                "App",
                true,
                &[
                    &settings_item,
                    &PredefinedMenuItem::separator(handle)?,
                    &quit_item,
                ],
            )?;

            let edit_submenu = Submenu::with_items(
                handle,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(handle, None)?,
                    &PredefinedMenuItem::redo(handle, None)?,
                    &PredefinedMenuItem::separator(handle)?,
                    &PredefinedMenuItem::cut(handle, None)?,
                    &PredefinedMenuItem::copy(handle, None)?,
                    &PredefinedMenuItem::paste(handle, None)?,
                    &PredefinedMenuItem::separator(handle)?,
                    &PredefinedMenuItem::select_all(handle, None)?,
                ],
            )?;

            let menu = Menu::with_items(handle, &[&app_submenu, &edit_submenu])?;
            app.set_menu(menu)?;

            app.on_menu_event(move |app, event| {
                if event.id == "settings" {
                    let _ = app.emit("open-settings", ());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::config::load_config,
            commands::config::save_config,
            commands::config::export_config,
            commands::config::import_config,
            commands::db::get_default_sqlite_path,
            commands::r2::test_r2_connection,
            commands::r2::list_r2_objects,
            commands::r2::read_r2_object,
            commands::db::test_database_connection,
            commands::db::initialize_database,
            commands::db::get_migration_versions,
            commands::db::get_current_db_version,
            commands::db::execute_migration_up,
            commands::db::execute_migration_down,
            commands::books::get_books,
            commands::books::get_book_cover,
            commands::books::get_book_metadata,
            commands::books::resolve_page_resource,
            commands::books::resolve_book_asset,
            commands::books::resolve_exercise_resource,
            commands::books::get_reading_progress,
            commands::books::update_reading_progress,
            commands::system::restart,
            check_connection_status
        ])
        .run(context)
        .expect("error while running tauri application");
}
