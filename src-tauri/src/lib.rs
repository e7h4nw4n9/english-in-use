pub mod commands;
pub mod database;
pub mod models;
pub mod services;
pub mod utils;

use std::str::FromStr;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Config, Manager};

fn get_log_level(config: &Config) -> log::LevelFilter {
    let identifier = &config.identifier;

    // Resolve config directory based on platform rules
    let config_dir = {
        #[cfg(target_os = "macos")]
        {
            std::env::var_os("HOME").map(|home| {
                let mut path = std::path::PathBuf::from(home);
                path.push("Library/Application Support");
                path.push(identifier);
                path
            })
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("APPDATA").map(|appdata| {
                let mut path = std::path::PathBuf::from(appdata);
                path.push(identifier);
                path
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|home| {
                        let mut path = std::path::PathBuf::from(home);
                        path.push(".config");
                        path
                    })
                })
                .map(|mut path| {
                    path.push(identifier);
                    path
                })
        }
    };

    if let Some(mut path) = config_dir {
        path.push("config.toml");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<crate::models::AppConfig>(&content) {
                    return log::LevelFilter::from_str(&config.system.log_level)
                        .unwrap_or(log::LevelFilter::Info);
                }
            }
        }
    }

    log::LevelFilter::Info
}

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
    let log_level = get_log_level(context.config());

    tauri::Builder::default()
        .manage(database::DbState::default())
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
        .setup(|app| {
            // 初始化全局应用数据目录常量
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");
            crate::utils::local::init_app_data_dir(app_data_dir);

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
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
            commands::books::get_books,
            commands::books::get_book_cover,
            commands::system::restart,
            check_connection_status
        ])
        .run(context)
        .expect("error while running tauri application");
}
