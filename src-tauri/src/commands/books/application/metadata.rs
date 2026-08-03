//! 图书元数据的本地优先加载、R2 回退和响应组装。

use crate::models::BookSource;
use crate::models::book_metadata::{PageIndex, TocNode};
use crate::services::book_metadata::MetadataService;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime, State};

use super::dependency_sync::trigger_deps_download;
use super::overlay::{extract_container_code_from_overlay, load_book_overlay_config};
use super::paths::{
    bundle_books_base_paths, bundle_courses_base_paths, file_exists_and_non_empty,
    write_bytes_to_path,
};
use super::{ERR_RESOURCE_NOT_FOUND, coded_error};

/// 前端阅读器加载一本图书所需的聚合元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMetadataResponse {
    /// 图书目录树。
    pub toc: Vec<TocNode>,
    /// 可选的练习目录树。
    pub exercise_toc: Option<Vec<TocNode>>,
    /// 按页面标签索引的页面信息。
    pub pages: HashMap<String, PageIndex>,
    /// 保持阅读顺序的页面标签。
    pub page_labels: Vec<String>,
    /// 页面设计宽度。
    pub page_width: f64,
    /// 页面设计高度。
    pub page_height: f64,
}

/// 获取图书元数据；优先使用本地或包内资源，缺失时从 R2 下载到缓存。
///
/// # 参数
/// - `app`：用于访问应用目录、资源目录和后台状态的应用句柄。
/// - `config_state`：当前图书来源配置。
/// - `product_code`：目标图书产品码。
pub async fn get_book_metadata<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
) -> Result<BookMetadataResponse, String> {
    info!("正在获取书籍元数据 (product_code: {})", product_code);

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

    let ebook_path_target = base_path.join(&product_code);
    let mut ebook_path_candidates = vec![ebook_path_target.clone()];
    ebook_path_candidates.extend(
        bundle_books_base_paths(&app)
            .into_iter()
            .map(|base| base.join(&product_code)),
    );
    let def_path = ebook_path_target.join("meta").join("definition.json");
    let book_json_path = ebook_path_target
        .join("assets")
        .join("imgbook-meta")
        .join("book.json");
    info!(
        "书籍元数据路径检查: product_code={}, base_path={}, target={}, definition_exists={}, book_json_exists={}, candidate_count={}",
        product_code,
        base_path.display(),
        ebook_path_target.display(),
        def_path.exists(),
        book_json_path.exists(),
        ebook_path_candidates.len()
    );

    let mut ebook_path = ebook_path_candidates.iter().find_map(|candidate| {
        let candidate_def = candidate.join("meta").join("definition.json");
        let candidate_book_json = candidate
            .join("assets")
            .join("imgbook-meta")
            .join("book.json");
        if file_exists_and_non_empty(&candidate_def)
            && file_exists_and_non_empty(&candidate_book_json)
        {
            Some(candidate.clone())
        } else {
            None
        }
    });

    if ebook_path.is_none()
        && let Some(BookSource::CloudflareGateway {}) = &book_source
    {
        info!("元数据缺失，尝试从 R2 下载...");
        let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
        let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

        // 统一下载到 single 结构
        let target_path = ebook_path_target.clone();
        let def_path = target_path.join("meta").join("definition.json");
        let bj_path = target_path
            .join("assets")
            .join("imgbook-meta")
            .join("book.json");
        let ov_path = target_path
            .join("assets")
            .join("imgbook-meta")
            .join("book-overlays.json");

        // 仅尝试 single key 模式
        let def_key = format!("books/{}/meta/definition.json", product_code);
        let bj_key = format!("books/{}/assets/imgbook-meta/book.json", product_code);
        let ov_key = format!(
            "books/{}/assets/imgbook-meta/book-overlays.json",
            product_code
        );
        let definition_missing = !file_exists_and_non_empty(&def_path);
        let book_missing = !file_exists_and_non_empty(&bj_path);
        let overlay_missing = !file_exists_and_non_empty(&ov_path);
        let (definition_result, book_result, overlay_result) = tokio::join!(
            async {
                if definition_missing {
                    crate::utils::r2::get_object(&client, &def_key)
                        .await
                        .map(Some)
                } else {
                    Ok(None)
                }
            },
            async {
                if book_missing {
                    crate::utils::r2::get_object(&client, &bj_key)
                        .await
                        .map(Some)
                } else {
                    Ok(None)
                }
            },
            async {
                if overlay_missing {
                    crate::utils::r2::get_optional_object(&client, &ov_key).await
                } else {
                    Ok(None)
                }
            },
        );

        let definition_data =
            definition_result.map_err(|error| coded_error(ERR_RESOURCE_NOT_FOUND, error))?;
        let book_data = book_result.map_err(|error| coded_error(ERR_RESOURCE_NOT_FOUND, error))?;
        if let Some(data) = definition_data {
            write_bytes_to_path(&def_path, &data)?;
        }
        if let Some(data) = book_data {
            write_bytes_to_path(&bj_path, &data)?;
        }
        match overlay_result {
            Ok(Some(data)) => write_bytes_to_path(&ov_path, &data)?,
            Ok(None) => {}
            Err(error) => warn!("下载可选叠加层失败: {error}"),
        }

        if file_exists_and_non_empty(&def_path) && file_exists_and_non_empty(&bj_path) {
            ebook_path = Some(target_path);
        }
    }

    let ebook_path = ebook_path.ok_or_else(|| {
        coded_error(
            ERR_RESOURCE_NOT_FOUND,
            format!(
            "找不到书籍资源文件。请确认路径正确且包含 definition.json 和 book.json。尝试过的目录: {:?}",
            ebook_path_candidates
            ),
        )
    })?;

    let def_path = ebook_path.join("meta").join("definition.json");
    let book_json_path = ebook_path
        .join("assets")
        .join("imgbook-meta")
        .join("book.json");
    let overlay_path = ebook_path
        .join("assets")
        .join("imgbook-meta")
        .join("book-overlays.json");
    info!(
        "书籍资源路径确定: product_code={}, ebook_path={}, definition_path={}, book_json_path={}, overlay_path={}, overlay_exists={}",
        product_code,
        ebook_path.display(),
        def_path.display(),
        book_json_path.display(),
        overlay_path.display(),
        overlay_path.exists()
    );

    let definition = MetadataService::parse_definition(&def_path)
        .map_err(|e| format!("解析 definition.json 失败: {}", e))?;
    let book_json = MetadataService::parse_book_json(&book_json_path)
        .map_err(|e| format!("解析 book.json 失败: {}", e))?;

    let overlay_config = match load_book_overlay_config(
        &app,
        &config_state,
        &book_source,
        &base_path,
        &product_code,
    )
    .await
    {
        Ok(config) => {
            info!("成功解析叠加层配置 (pages: {})", config.pages.page.len());
            Some(config)
        }
        Err(e) => {
            warn!(
                "加载叠加层配置失败，继续返回基础阅读元数据: product_code={}, error={}",
                product_code, e
            );
            None
        }
    };

    let mut exercise_mapping = None;
    let mut module_mapping = None;
    let mut exercise_toc = None;
    let mut has_container_code = false;

    if let Some(overlay_config_ref) = overlay_config.as_ref() {
        match extract_container_code_from_overlay(overlay_config_ref, &product_code) {
            Ok(container_code) => {
                has_container_code = true;
                let courses_base_path = match &book_source {
                    Some(BookSource::Local { path }) => {
                        let base = PathBuf::from(path);
                        crate::utils::local::ensure_path_not_in_project_temp(
                            &base,
                            "book_source.local.path",
                        )?;
                        base.join("courses")
                    }
                    _ => {
                        let cache_dir = app
                            .path()
                            .app_cache_dir()
                            .map_err(|e| format!("无法获取缓存目录: {}", e))?;
                        cache_dir.join("courses")
                    }
                };

                let ebook_con_path_target = courses_base_path.join(&container_code);
                let mut course_container_candidates = vec![ebook_con_path_target.clone()];
                course_container_candidates.extend(
                    bundle_courses_base_paths(&app)
                        .into_iter()
                        .map(|base| base.join(&container_code)),
                );
                let mut con_def_path = course_container_candidates.iter().find_map(|candidate| {
                    let p = candidate.join("meta").join("definition.json");
                    if file_exists_and_non_empty(&p) {
                        Some(p)
                    } else {
                        None
                    }
                });
                info!(
                    "练习容器路径检查: product_code={}, container_code={}, courses_base_path={}, container_target={}, container_definition_exists={}, candidate_count={}",
                    product_code,
                    container_code,
                    courses_base_path.display(),
                    ebook_con_path_target.display(),
                    con_def_path.is_some(),
                    course_container_candidates.len()
                );

                if con_def_path.is_none()
                    && let Some(BookSource::CloudflareGateway {}) = &book_source
                {
                    let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
                    let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;
                    // 统一下载到 single 结构
                    let target_con_path = ebook_con_path_target.clone();
                    let p = target_con_path.join("meta").join("definition.json");

                    let con_def_key = format!("courses/{}/meta/definition.json", container_code);

                    if let Ok(data) = crate::utils::r2::get_object(&client, &con_def_key).await {
                        write_bytes_to_path(&p, &data)?;
                        con_def_path = Some(p);
                    }
                }

                if let Some(path) = con_def_path {
                    info!(
                        "准备解析练习容器 definition: product_code={}, container_code={}, path={}",
                        product_code,
                        container_code,
                        path.display()
                    );

                    match MetadataService::parse_definition(&path) {
                        Ok(con_def) => {
                            let built_exercise_mapping =
                                MetadataService::build_exercise_mapping(&con_def);
                            let built_module_mapping =
                                MetadataService::build_module_mapping(&con_def);
                            let parsed_exercise_toc = MetadataService::parse_toc(&con_def, None);

                            let exercise_page_keys = built_exercise_mapping.len();
                            let total_exercises: usize = built_exercise_mapping
                                .values()
                                .map(std::vec::Vec::len)
                                .sum();
                            let module_keys = built_module_mapping.len();
                            let exercise_toc_nodes = parsed_exercise_toc.len();

                            info!(
                                "练习容器解析完成: product_code={}, container_code={}, root_items={}, exercise_page_keys={}, total_exercises={}, module_keys={}, exercise_toc_nodes={}",
                                product_code,
                                container_code,
                                con_def.items.default.len(),
                                exercise_page_keys,
                                total_exercises,
                                module_keys,
                                exercise_toc_nodes
                            );

                            exercise_mapping = Some(built_exercise_mapping);
                            module_mapping = Some(built_module_mapping);
                            exercise_toc = Some(parsed_exercise_toc);
                        }
                        Err(e) => {
                            warn!(
                                "解析练习容器 definition 失败: product_code={}, container_code={}, path={}, error={}",
                                product_code,
                                container_code,
                                path.display(),
                                e
                            );
                        }
                    }
                } else {
                    warn!(
                        "未找到练习容器 definition，当前书籍不会关联练习资源: product_code={}, container_code={}, expected_path={}",
                        product_code,
                        container_code,
                        ebook_con_path_target
                            .join("meta")
                            .join("definition.json")
                            .display()
                    );
                }
            }
            Err(e) => {
                warn!(
                    "叠加层缺少可用 courseId，跳过练习容器关联: product_code={}, error={}",
                    product_code, e
                );
            }
        }
    } else {
        warn!(
            "叠加层配置不可用，跳过练习容器关联: product_code={}",
            product_code
        );
    }

    if has_container_code {
        // 触发异步后台下载 deps 资源
        let app_handle = app.clone();
        let product_code_clone = product_code.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            if let Err(e) =
                trigger_deps_download(app_handle, product_code_clone, None, true, false).await
            {
                error!("后台同步 deps 失败: {}", e);
            }
        });
    }

    let page_labels: Vec<String> = book_json
        .pages
        .page
        .iter()
        .map(|p| p.pagelabel.clone())
        .collect();

    let toc = MetadataService::parse_toc(&definition, overlay_config.as_ref());
    info!(
        "开始构建页面索引: product_code={}, pages={}, has_overlay_config={}, has_exercise_mapping={}, has_module_mapping={}",
        product_code,
        page_labels.len(),
        overlay_config.is_some(),
        exercise_mapping.is_some(),
        module_mapping.is_some()
    );
    let pages = MetadataService::build_page_index(
        &definition,
        &book_json,
        exercise_mapping.as_ref(),
        module_mapping.as_ref(),
        overlay_config.as_ref(),
    );
    info!(
        "书籍元数据构建完成: product_code={}, toc_nodes={}, exercise_toc_nodes={}, pages={}",
        product_code,
        toc.len(),
        exercise_toc.as_ref().map(std::vec::Vec::len).unwrap_or(0),
        pages.len()
    );

    Ok(BookMetadataResponse {
        toc,
        exercise_toc,
        pages,
        page_labels,
        page_width: book_json.page_width,
        page_height: book_json.page_height,
    })
}
