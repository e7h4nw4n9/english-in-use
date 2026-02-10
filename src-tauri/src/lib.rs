pub mod config;
pub mod commands;
pub mod r2;

use tauri::menu::{Menu, MenuItem, Submenu, PredefinedMenuItem};
use tauri::Emitter;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle();
            
            let settings_item = MenuItem::with_id(handle, "settings", "Settings...", true, Some("CmdOrCtrl+,"))?;
            let quit_item = PredefinedMenuItem::quit(handle, None)?;
            
            let app_submenu = Submenu::with_items(
                handle,
                "App",
                true,
                &[&settings_item, &PredefinedMenuItem::separator(handle)?, &quit_item],
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
            commands::load_config,
            commands::save_config,
            commands::export_config,
            commands::import_config,
            commands::test_r2_connection,
            commands::list_r2_objects,
            commands::read_r2_object,
            commands::test_postgresql_connection,
            commands::restart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
