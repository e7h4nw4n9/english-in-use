use log::info;
use tauri::AppHandle;

#[tauri::command]
pub fn restart(app: AppHandle) {
    info!("正在重启应用...");
    app.restart();
}
