use log::info;
use tauri::AppHandle;

#[tauri::command]
pub fn restart(app: AppHandle) {
    info!("正在重启应用...");
    app.restart();
}

#[tauri::command]
pub fn get_platform() -> String {
    // 返回当前操作系统平台：windows, macos, linux, android, ios
    if cfg!(target_os = "windows") {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "android") {
        "android".to_string()
    } else if cfg!(target_os = "ios") {
        "ios".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    }
}
