//! 图书命令的应用层门面，保持对外导出稳定并组织各领域子模块。

mod dependency_sync;
mod exercise;
mod metadata;
mod overlay;
mod paths;
mod resources;

pub use exercise::{ExerciseHtmlResponse, get_exercise_html};
pub use metadata::{BookMetadataResponse, get_book_metadata};
pub use resources::{resolve_book_asset, resolve_exercise_resource, resolve_page_resource};

pub(super) use super::infrastructure::{
    DownloadBatchOptions, DownloadProgress, download_r2_objects_concurrently,
};

const ERR_OVERLAY_NO_COURSE_ID: &str = "ERR_OVERLAY_NO_COURSE_ID";
const ERR_OVERLAY_INVALID_STRUCTURE: &str = "ERR_OVERLAY_INVALID_STRUCTURE";
const ERR_PATH_OUTSIDE_BASE: &str = "ERR_PATH_OUTSIDE_BASE";
const ERR_PATH_INVALID_RELATIVE: &str = "ERR_PATH_INVALID_RELATIVE";
const ERR_RESOURCE_NOT_FOUND: &str = "ERR_RESOURCE_NOT_FOUND";

/// 生成带稳定错误码的错误文本，便于前端识别错误类别。
///
/// # 参数
/// - `code`：稳定的机器可读错误码。
/// - `message`：提供给日志和用户界面的具体错误信息。
pub(super) fn coded_error(code: &str, message: impl Into<String>) -> String {
    format!("[{}] {}", code, message.into())
}
