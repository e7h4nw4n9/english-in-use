//! 练习入口资源、课程定义和远程资源包解析。

use super::*;

/// 构造边界明确的练习资源对象前缀。
fn exercise_object_prefix(container_code: &str, relative_path: &str) -> String {
    format!("courses/{container_code}/assets/{relative_path}/")
}

/// 解析练习入口文件，允许在本地缺失时从 R2 下载练习包。
///
/// # 参数
/// - `app`：用于访问缓存、资源目录和 R2 状态的应用句柄。
/// - `config_state`：当前图书来源配置。
/// - `product_code`：目标图书产品码。
/// - `resource_id`：练习资源标识。
pub async fn resolve_exercise_resource<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    resource_id: String,
) -> Result<String, String> {
    resolve_exercise_resource_internal(app, config_state, product_code, resource_id, false).await
}

/// 执行练习资源解析，并按调用方要求决定是否发送下载进度事件。
///
/// # 参数
/// - `app`：用于访问缓存、资源目录和 R2 状态的应用句柄。
/// - `config_state`：当前图书来源配置。
/// - `product_code`：目标图书产品码。
/// - `resource_id`：练习资源标识。
/// - `emit_progress`：是否向前端发送下载进度事件。
pub(in crate::commands::books::application) async fn resolve_exercise_resource_internal<
    R: Runtime,
>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    resource_id: String,
    emit_progress: bool,
) -> Result<String, String> {
    let (book_source, books_base_path, courses_base_path) = {
        let config = config_state.0.read().map_err(|e| e.to_string())?;
        let source = config.book_source.clone();
        let (books_path, courses_path) = match &source {
            Some(BookSource::Local { path }) => {
                let base = PathBuf::from(path);
                crate::utils::local::ensure_path_not_in_project_temp(
                    &base,
                    "book_source.local.path",
                )?;
                (base.join("books"), base.join("courses"))
            }
            _ => {
                let cache_dir = app
                    .path()
                    .app_cache_dir()
                    .map_err(|e| format!("无法获取缓存目录: {}", e))?;
                (cache_dir.join("books"), cache_dir.join("courses"))
            }
        };
        Ok::<(Option<BookSource>, PathBuf, PathBuf), String>((source, books_path, courses_path))
    }?;

    let overlay_config = load_book_overlay_config(
        &app,
        &config_state,
        &book_source,
        &books_base_path,
        &product_code,
    )
    .await?;
    let container_code = extract_container_code_from_overlay(&overlay_config, &product_code)?;

    let container_path = courses_base_path.join(&container_code);
    let mut container_paths = vec![container_path.clone()];
    for base in bundle_courses_base_paths(&app) {
        push_unique_path(&mut container_paths, base.join(&container_code));
    }

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("无法获取缓存目录: {}", e))?;
    let cache_container_path = cache_dir.join("courses").join(&container_code);
    push_unique_path(&mut container_paths, cache_container_path.clone());

    let con_def_candidates: Vec<PathBuf> = container_paths
        .iter()
        .map(|base| base.join("meta").join("definition.json"))
        .collect();

    let con_def_path = if let Some(path) = con_def_candidates
        .iter()
        .find(|candidate| candidate.exists())
        .cloned()
    {
        path
    } else {
        // 本地缺少课程定义时再从 R2 下载。
        if let Some(BookSource::CloudflareGateway {}) = &book_source {
            let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
            let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;
            let target_path = cache_container_path.join("meta").join("definition.json");
            let key = format!("courses/{}/meta/definition.json", container_code);

            if let Ok(data) = crate::utils::r2::get_object(&client, &key).await {
                write_bytes_to_path(&target_path, &data)?;
                target_path
            } else {
                return Err(coded_error(
                    ERR_RESOURCE_NOT_FOUND,
                    format!("找不到练习资源路径且无法从 R2 下载: {:?}", container_paths),
                ));
            }
        } else {
            return Err(coded_error(
                ERR_RESOURCE_NOT_FOUND,
                format!("找不到练习资源路径: {:?}", container_paths),
            ));
        }
    };

    let content = std::fs::read_to_string(&con_def_path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let res_info = &v["resources"]["generic"][&resource_id]["ext-cup-xapi"];
    let url_raw = res_info["url"].as_str().ok_or_else(|| {
        coded_error(
            ERR_OVERLAY_INVALID_STRUCTURE,
            format!("资源 ID {} 缺少 ext-cup-xapi url", resource_id),
        )
    })?;
    let safe_url_rel = normalize_safe_relative_path(url_raw, "ext-cup-xapi.url")?;
    let safe_url_rel_str = path_to_slash_string(&safe_url_rel);

    // 根据 definition.json 中的 URL 路径构造 index.html 候选位置。
    let cache_index_path = cache_container_path
        .join("assets")
        .join(&safe_url_rel)
        .join("index.html");
    let mut search_paths: Vec<PathBuf> = container_paths
        .iter()
        .map(|base| base.join("assets").join(&safe_url_rel).join("index.html"))
        .collect();
    push_unique_path(&mut search_paths, cache_index_path.clone());

    for path in &search_paths {
        if file_exists_and_non_empty(path) {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    // 本地候选均不存在且配置了 R2 时，再下载完整练习资源。
    if let Some(BookSource::CloudflareGateway {}) = &book_source {
        info!(
            "本地缺失练习资源 (url: {})，尝试从 R2 下载",
            safe_url_rel_str
        );
        let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
        let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

        // 末尾斜杠用于隔离相似资源名，例如 unit1 与 unit10。
        let prefix = exercise_object_prefix(&container_code, &safe_url_rel_str);

        // 获取该前缀下的所有文件
        let objects = crate::utils::r2::list_objects(&client, Some(&prefix))
            .await
            .map_err(|e| format!("列出 R2 练习资源失败: {}", e))?;

        if objects.is_empty() {
            return Err(coded_error(
                ERR_RESOURCE_NOT_FOUND,
                format!("在 R2 上未找到练习资源 (prefix: {})", prefix),
            ));
        }

        info!("同步中: 发现 {} 个文件需要同步", objects.len());
        let stats = download_r2_objects_concurrently(
            &app,
            &client,
            objects,
            DownloadBatchOptions {
                cache_container_path: cache_container_path.clone(),
                container_code: &container_code,
                progress: emit_progress.then_some(DownloadProgress::new(
                    &product_code,
                    Some(&resource_id),
                    "resource",
                )),
            },
        )
        .await;
        info!(
            "练习资源并发同步完成: product_code={}, resource_id={}, total={}, downloaded={}, skipped={}, failed={}",
            product_code,
            resource_id,
            stats.total_files,
            stats.downloaded_files,
            stats.skipped_files,
            stats.failed_files
        );

        if crate::utils::cache::is_non_empty_file(&cache_index_path).await {
            return Ok(cache_index_path.to_string_lossy().to_string());
        }
    }

    Err(coded_error(
        ERR_RESOURCE_NOT_FOUND,
        format!(
            "在本地及 R2 均未找到练习资源 (index.html): {:?}",
            search_paths
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::exercise_object_prefix;

    #[test]
    fn object_prefix_isolates_similar_resource_names() {
        let prefix = exercise_object_prefix("course", "unit1");
        assert_eq!(prefix, "courses/course/assets/unit1/");
        assert!(!"courses/course/assets/unit10/index.html".starts_with(&prefix));
    }
}
