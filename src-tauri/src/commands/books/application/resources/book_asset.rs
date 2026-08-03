//! 图书相对资产的安全解析与按需下载。

use super::*;

/// 解析图书内的任意资产，并强制限制在目标图书目录内。
///
/// # 参数
/// - `app`：用于访问缓存、资源目录和 R2 状态的应用句柄。
/// - `config_state`：当前图书来源配置。
/// - `product_code`：目标图书产品码。
/// - `relative_path`：图书目录内的安全相对路径。
pub async fn resolve_book_asset<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    relative_path: String,
) -> Result<String, String> {
    let (book_source, base_path) = {
        let config = config_state.0.read().map_err(|e| e.to_string())?;
        let source = config.book_source.clone();
        let path = match &source {
            Some(BookSource::Local { path }) => {
                let base = PathBuf::from(path);
                crate::utils::local::ensure_path_not_in_project_temp(
                    &base,
                    "book_source.local.path",
                )?;
                base.join("books")
            }
            _ => {
                let cache_dir = app
                    .path()
                    .app_cache_dir()
                    .map_err(|e| format!("无法获取缓存目录: {}", e))?;
                cache_dir.join("books")
            }
        };
        Ok::<(Option<BookSource>, PathBuf), String>((source, path))
    }?;

    let ebook_path = base_path.join(&product_code);
    let mut ebook_candidates = vec![ebook_path.clone()];
    ebook_candidates.extend(
        bundle_books_base_paths(&app)
            .into_iter()
            .map(|base| base.join(&product_code)),
    );

    // 处理 URL 编码的路径
    let safe_rel_path_raw = urlencoding::decode(&relative_path)
        .map(|s| s.into_owned())
        .unwrap_or(relative_path);
    let safe_rel_path =
        normalize_safe_relative_path(&safe_rel_path_raw, "resolve_book_asset.relative_path")?;
    let safe_rel_path_str = path_to_slash_string(&safe_rel_path);

    // 尝试两个可能的本地路径：直接路径和 assets/ 下的路径
    let mut paths_to_try = Vec::new();
    for candidate in &ebook_candidates {
        paths_to_try.push(candidate.join(&safe_rel_path));
        paths_to_try.push(candidate.join("assets").join(&safe_rel_path));
    }

    let mut asset_path = None;
    for path in &paths_to_try {
        if file_exists_and_non_empty(path) {
            asset_path = Some(path.clone());
            break;
        }
    }

    if asset_path.is_none() {
        if let Some(BookSource::CloudflareGateway {}) = book_source {
            info!("资源文件缺失，尝试从 R2 下载: {}", safe_rel_path_str);
            let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
            let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

            // 尝试下载两个可能的 Key：直接路径和 assets/ 下的路径
            let keys = [
                format!("books/{}/assets/{}", product_code, safe_rel_path_str),
                format!("books/{}/{}", product_code, safe_rel_path_str),
            ];

            let mut final_path: Option<PathBuf> = None;

            for (i, key) in keys.iter().enumerate() {
                // 如果是用 assets/ 开头的 Key 下载成功的，保存到 assets/ 子目录。
                let target_path = if i == 0 {
                    ebook_path.join("assets").join(&safe_rel_path)
                } else {
                    ebook_path.join(&safe_rel_path)
                };
                if crate::utils::r2::download_object_to_path(&client, key, &target_path)
                    .await
                    .is_ok()
                {
                    final_path = Some(target_path);
                    break;
                }
            }

            if let Some(path) = final_path {
                asset_path = Some(path);
            } else {
                return Err(coded_error(
                    ERR_RESOURCE_NOT_FOUND,
                    format!("从 R2 下载资源文件失败。尝试过的 Key: {:?}", keys),
                ));
            }
        } else {
            return Err(coded_error(
                ERR_RESOURCE_NOT_FOUND,
                format!(
                    "资源文件不存在且未配置云端源。尝试过的本地路径: {:?}",
                    paths_to_try
                ),
            ));
        }
    }

    let final_asset_path = asset_path.ok_or("无法定位资源文件")?;

    #[cfg(not(test))]
    {
        Ok(final_asset_path
            .to_str()
            .ok_or("Invalid path encoding")?
            .to_string())
    }
    #[cfg(test)]
    {
        let _ = app;
        Ok(final_asset_path.to_str().unwrap().to_string())
    }
}
