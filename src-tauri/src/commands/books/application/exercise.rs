//! 练习 HTML 的资源准备、内容改写和运行时兼容处理。

mod rewriter;
mod source;

use log::info;
use tauri::{AppHandle, Runtime, State};

use self::rewriter::rewrite_exercise_html;
use self::source::prepare_exercise_source;

/// 已处理的练习 HTML 及其资源基准 URL。
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseHtmlResponse {
    /// 可直接交给前端渲染的 HTML。
    pub html: String,
    /// 练习入口文件对应的资源 URL。
    pub url: String,
}

/// 准备可离线运行的练习 HTML，并注入资源映射与运行时保护逻辑。
///
/// # 参数
/// - `app`：用于访问缓存和后台下载状态的应用句柄。
/// - `config_state`：当前图书来源配置。
/// - `product_code`：目标图书产品码。
/// - `resource_id`：目标练习资源标识。
pub async fn get_exercise_html<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    resource_id: String,
) -> Result<ExerciseHtmlResponse, String> {
    info!("正在准备内联练习 HTML (resource_id: {})", resource_id);

    let source = prepare_exercise_source(app, config_state, &product_code, &resource_id).await?;
    let index_url = source.index_url.clone();
    let html = rewrite_exercise_html(source, &resource_id)?;

    crate::utils::exercise_asset::cache_processed_html(index_url.clone(), html.clone());
    Ok(ExerciseHtmlResponse {
        html,
        url: index_url,
    })
}
