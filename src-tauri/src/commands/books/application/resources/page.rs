//! 页面图片的本地优先解析与按需下载。

use super::*;

/// 解析页面图片资源，必要时先补齐元数据或从 R2 下载缺失图片。
///
/// # 参数
/// - `app`：用于访问缓存、资源目录和 R2 状态的应用句柄。
/// - `config_state`：当前图书来源配置。
/// - `product_code`：目标图书产品码。
/// - `page_label`：需要解析的页面标签。
pub async fn resolve_page_resource<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    page_label: String,
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

    let writable_ebook_path = base_path.join(&product_code);
    let mut ebook_candidates = vec![writable_ebook_path.clone()];
    ebook_candidates.extend(
        bundle_books_base_paths(&app)
            .into_iter()
            .map(|base| base.join(&product_code)),
    );
    let ebook_path = ebook_candidates
        .iter()
        .find(|candidate| {
            candidate
                .join("assets")
                .join("imgbook-meta")
                .join("book.json")
                .is_file()
        })
        .cloned()
        .unwrap_or_else(|| writable_ebook_path.clone());
    let book_json_path = ebook_path
        .join("assets")
        .join("imgbook-meta")
        .join("book.json");

    if matches!(book_source, Some(BookSource::CloudflareGateway { .. }))
        && !file_exists_and_non_empty(&book_json_path)
    {
        info!(
            "页面资源依赖元数据缺失，尝试先同步元数据: product_code={}, ebook_path={}, book_json_path={}",
            product_code,
            ebook_path.display(),
            book_json_path.display()
        );
        let _ = get_book_metadata(app.clone(), config_state.clone(), product_code.clone()).await?;
    }

    if !ebook_path.exists() {
        return Err(coded_error(
            ERR_RESOURCE_NOT_FOUND,
            format!("找不到书籍资源路径: {:?}", ebook_path),
        ));
    }

    if !file_exists_and_non_empty(&book_json_path) {
        return Err(coded_error(
            ERR_RESOURCE_NOT_FOUND,
            format!("找不到页面映射文件 book.json: {}", book_json_path.display()),
        ));
    }

    let book_json = MetadataService::parse_book_json(&book_json_path)
        .map_err(|e| format!("解析 book.json 失败: {}", e))?;

    let page_info = book_json
        .pages
        .page
        .iter()
        .find(|p| p.pagelabel == page_label)
        .ok_or_else(|| {
            coded_error(
                ERR_RESOURCE_NOT_FOUND,
                format!("未找到页码标签: {}", page_label),
            )
        })?;

    let image_rel_path_raw = format!(
        "assets/{}{}",
        book_json.paths.pagexl_lrg_img_folder, page_info.bgimage
    );

    // 解码路径以处理 %20 等字符，确保在本地文件系统和 R2 Key 中使用原始字符
    let image_rel_path_raw = urlencoding::decode(&image_rel_path_raw)
        .map(|s| s.into_owned())
        .unwrap_or(image_rel_path_raw);
    let image_rel_path = normalize_safe_relative_path(
        &image_rel_path_raw,
        "resolve_page_resource.image_relative_path",
    )?;
    let image_rel_path_str = path_to_slash_string(&image_rel_path);

    let image_path = ebook_path.join(&image_rel_path);
    let cache_image_path = writable_ebook_path.join(&image_rel_path);
    let resolved_image_path = if file_exists_and_non_empty(&image_path) {
        image_path
    } else if file_exists_and_non_empty(&cache_image_path) {
        cache_image_path.clone()
    } else if let Some(BookSource::CloudflareGateway {}) = book_source {
        info!(
            "资源文件缺失，尝试从 R2 下载: product_code={}, key_path={}, read_path={}, cache_path={}",
            product_code,
            image_rel_path_str,
            image_path.display(),
            cache_image_path.display()
        );
        let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
        let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

        let key = format!("books/{}/{}", product_code, image_rel_path_str);

        if crate::utils::r2::download_object_to_path(&client, &key, &cache_image_path)
            .await
            .is_ok()
        {
            cache_image_path
        } else {
            return Err(coded_error(
                ERR_RESOURCE_NOT_FOUND,
                format!("从 R2 下载资源文件失败。Key: {}", key),
            ));
        }
    } else {
        return Err(coded_error(
            ERR_RESOURCE_NOT_FOUND,
            format!(
                "图片文件不存在且未配置云端源: read_path={}, cache_path={}",
                image_path.display(),
                cache_image_path.display()
            ),
        ));
    };

    #[cfg(not(test))]
    {
        Ok(resolved_image_path
            .to_str()
            .ok_or("Invalid path encoding")?
            .to_string())
    }
    #[cfg(test)]
    {
        let _ = app;
        Ok(resolved_image_path.to_str().unwrap().to_string())
    }
}
