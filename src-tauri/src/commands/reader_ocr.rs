//! 阅读器选词的原生命令。

use crate::services::{config::ConfigState, reader_ocr};
use tauri::{AppHandle, State};

/// 根据书籍及页码识别页面，不接受网页传来的任意本地路径。
#[tauri::command]
pub async fn recognize_reader_page(
    app: AppHandle,
    config: State<'_, ConfigState>,
    product_code: String,
    page_label: String,
) -> Result<serde_json::Value, String> {
    reader_ocr::validate_product_code(&product_code)?;
    let path = std::path::PathBuf::from(
        crate::commands::books::resolve_page_resource(
            app,
            config,
            product_code.clone(),
            page_label,
        )
        .await?,
    );
    // 允许书籍目录本身是用户配置的链接，但不允许书页链接越出该书籍目录。
    let book_root = path
        .ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .is_some_and(|name| name == product_code.as_str())
        })
        .ok_or("无法确认书页所属书籍目录")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(book_root) {
        return Err("书页资源越出书籍目录".into());
    }
    reader_ocr::recognize(canonical).await
}

/// 后台调用原生剪贴板，避免在主线程阻塞 Tauri 命令处理。
#[tauri::command]
pub async fn copy_reader_word(word: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || reader_ocr::copy_word(&word))
        .await
        .map_err(|error| error.to_string())?
}
