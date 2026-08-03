//! 练习入口文件、依赖目录和本地协议 URL 的准备。

use log::info;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Runtime, State};

use super::super::dependency_sync::{local_deps_ready, trigger_deps_download};
use super::super::resources::resolve_exercise_resource_internal;

/// HTML 改写阶段所需的本地文件和协议 URL。
pub(super) struct ExerciseSource {
    pub(super) html: String,
    pub(super) data_js_content_raw: Option<String>,
    pub(super) engine_dir: PathBuf,
    pub(super) dp_dir: PathBuf,
    pub(super) engine_base_url: String,
    pub(super) dp_base_url: String,
    pub(super) media_base_url: String,
    pub(super) index_url: String,
}

/// 下载并校验练习依赖，随后读取入口 HTML 与可选 data.js。
///
/// # 参数
/// - `app`：用于解析缓存和资源路径的应用句柄。
/// - `config_state`：当前图书来源配置。
/// - `product_code`：目标图书产品码。
/// - `resource_id`：目标练习资源标识。
pub(super) async fn prepare_exercise_source<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: &str,
    resource_id: &str,
) -> Result<ExerciseSource, String> {
    info!(
        "练习加载前检查 deps 就绪状态: product_code={}, resource_id={}",
        product_code, resource_id
    );
    trigger_deps_download(
        app.clone(),
        product_code.to_string(),
        Some(resource_id.to_string()),
        true,
        true,
    )
    .await?;
    info!(
        "练习加载前 deps 同步完成: product_code={}, resource_id={}",
        product_code, resource_id
    );

    let index_path_str = resolve_exercise_resource_internal(
        app,
        config_state,
        product_code.to_string(),
        resource_id.to_string(),
        true,
    )
    .await?;
    let index_path = PathBuf::from(&index_path_str);
    let index_path = index_path.canonicalize().unwrap_or(index_path);
    let index_dir = index_path.parent().ok_or("无法获取练习目录")?;
    let assets_dir = index_dir
        .parent()
        .and_then(|path| path.parent())
        .ok_or("无法获取 assets 目录")?;

    if !local_deps_ready(assets_dir) {
        return Err(format!(
            "练习依赖尚未就绪，请稍后重试 (assets: {})",
            assets_dir.display()
        ));
    }

    let html = std::fs::read_to_string(&index_path)
        .map_err(|error| format!("读取 index.html 失败: {}", error))?;
    let data_js_path = index_dir.join("data.js");
    let data_js_content_raw = if data_js_path.exists() {
        std::fs::read_to_string(&data_js_path).ok()
    } else {
        None
    };

    let engine_dir = assets_dir.join("deps").join("engine");
    let dp_dir = assets_dir.join("deps").join("dp");
    let media_dir = index_dir.join("media");
    let index_dir_url = format!(
        "{}/",
        to_exercise_asset_url(index_dir).trim_end_matches('/')
    );

    Ok(ExerciseSource {
        html,
        data_js_content_raw,
        engine_base_url: to_exercise_asset_url(&engine_dir),
        dp_base_url: to_exercise_asset_url(&dp_dir),
        media_base_url: to_exercise_asset_url(&media_dir),
        index_url: format!("{}index.html", index_dir_url),
        engine_dir,
        dp_dir,
    })
}

/// 将本地路径转换为练习资产自定义协议 URL。
///
/// # 参数
/// - `path`：需要转换的本地绝对路径。
pub(super) fn to_exercise_asset_url(path: &Path) -> String {
    #[cfg(target_os = "windows")]
    {
        format!(
            "eiuasset://localhost/{}",
            path.to_string_lossy()
                .replace("\\\\", "/")
                .replace(' ', "%20")
                .replace(':', "%3A")
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!(
            "eiuasset://localhost{}",
            path.to_string_lossy().replace(' ', "%20")
        )
    }
}
