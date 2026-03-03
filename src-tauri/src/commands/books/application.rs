use crate::models::BookSource;
use crate::models::book_metadata::{OverlayConfig, PageIndex, TocNode};
use crate::services::book_metadata::MetadataService;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::infrastructure::download_r2_objects_concurrently;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExerciseDownloadProgressEvent {
    product_code: String,
    resource_id: Option<String>,
    stage: String,
    total_files: usize,
    completed_files: usize,
    failed_files: usize,
    percent: f64,
    done: bool,
}

fn emit_exercise_download_progress<R: Runtime>(
    app: &AppHandle<R>,
    product_code: &str,
    resource_id: Option<&str>,
    stage: &str,
    total_files: usize,
    completed_files: usize,
    failed_files: usize,
    done: bool,
) {
    let percent = if total_files == 0 {
        if done { 100.0 } else { 0.0 }
    } else {
        (completed_files as f64 / total_files as f64) * 100.0
    };

    if let Err(e) = app.emit(
        "exercise-download-progress",
        ExerciseDownloadProgressEvent {
            product_code: product_code.to_string(),
            resource_id: resource_id.map(str::to_string),
            stage: stage.to_string(),
            total_files,
            completed_files,
            failed_files,
            percent,
            done,
        },
    ) {
        warn!("发送练习下载进度事件失败: {}", e);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMetadataResponse {
    pub toc: Vec<TocNode>,
    pub exercise_toc: Option<Vec<TocNode>>,
    pub pages: HashMap<String, PageIndex>,
    pub page_labels: Vec<String>,
    pub page_width: f64,
    pub page_height: f64,
}

static DEPS_DOWNLOAD_INFLIGHT: OnceLock<tokio::sync::Mutex<HashSet<String>>> = OnceLock::new();
const ERR_OVERLAY_NO_COURSE_ID: &str = "ERR_OVERLAY_NO_COURSE_ID";
const ERR_OVERLAY_INVALID_STRUCTURE: &str = "ERR_OVERLAY_INVALID_STRUCTURE";
const ERR_PATH_OUTSIDE_BASE: &str = "ERR_PATH_OUTSIDE_BASE";
const ERR_PATH_INVALID_RELATIVE: &str = "ERR_PATH_INVALID_RELATIVE";
const ERR_RESOURCE_NOT_FOUND: &str = "ERR_RESOURCE_NOT_FOUND";

fn deps_download_inflight() -> &'static tokio::sync::Mutex<HashSet<String>> {
    DEPS_DOWNLOAD_INFLIGHT.get_or_init(|| tokio::sync::Mutex::new(HashSet::new()))
}

fn coded_error(code: &str, message: impl Into<String>) -> String {
    format!("[{}] {}", code, message.into())
}

fn normalize_safe_relative_path(input: &str, field_name: &str) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(coded_error(
            ERR_PATH_INVALID_RELATIVE,
            format!("{} is empty", field_name),
        ));
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(coded_error(
            ERR_PATH_INVALID_RELATIVE,
            format!("{} must be relative: {}", field_name, trimmed),
        ));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                return Err(coded_error(
                    ERR_PATH_OUTSIDE_BASE,
                    format!(
                        "{} contains parent traversal segment and is rejected: {}",
                        field_name, trimmed
                    ),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(coded_error(
                    ERR_PATH_INVALID_RELATIVE,
                    format!("{} has invalid path component: {}", field_name, trimmed),
                ));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(coded_error(
            ERR_PATH_INVALID_RELATIVE,
            format!(
                "{} becomes empty after normalization: {}",
                field_name, trimmed
            ),
        ));
    }

    Ok(normalized)
}

fn path_to_slash_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn bundle_resource_roots<R: Runtime>(app: &AppHandle<R>) -> Vec<PathBuf> {
    let Ok(resource_dir) = app.path().resource_dir() else {
        return Vec::new();
    };
    let mut roots = vec![resource_dir.clone(), resource_dir.join("assets")];
    roots.sort();
    roots.dedup();
    roots
}

fn bundle_books_base_paths<R: Runtime>(app: &AppHandle<R>) -> Vec<PathBuf> {
    bundle_resource_roots(app)
        .into_iter()
        .map(|root| root.join("books"))
        .collect()
}

fn bundle_courses_base_paths<R: Runtime>(app: &AppHandle<R>) -> Vec<PathBuf> {
    bundle_resource_roots(app)
        .into_iter()
        .map(|root| root.join("courses"))
        .collect()
}

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if paths.iter().any(|path| path == &candidate) {
        return;
    }
    paths.push(candidate);
}

fn file_exists_and_non_empty(path: &Path) -> bool {
    path.exists()
        && std::fs::metadata(path)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
}

fn write_bytes_to_path(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败 (path: {}): {}", parent.display(), e))?;
    }
    std::fs::write(path, data)
        .map_err(|e| format!("写入文件失败 (path: {}): {}", path.display(), e))
}

fn local_deps_ready(assets_dir: &Path) -> bool {
    let engine_dir = assets_dir.join("deps").join("engine");
    let dp_dir = assets_dir.join("deps").join("dp");

    let has_engine_entry = std::fs::read_dir(&engine_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .any(|path| {
            path.is_dir()
                && path.join("launcher-connector-channel.bundle.js").is_file()
                && path.join("player.js").is_file()
        });

    let has_dp_entry = std::fs::read_dir(&dp_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .any(|path| {
            if !path.is_dir() {
                return false;
            }
            path.join("css").join("style.css").is_file()
                || path.join("js.js").is_file()
                || path.join("templates.js").is_file()
        });

    has_engine_entry && has_dp_entry
}

fn deps_download_marker_path<R: Runtime>(
    app: &AppHandle<R>,
    container_code: &str,
) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .or_else(|_| app.path().app_cache_dir())
        .map_err(|e| format!("无法获取应用目录用于记录 deps 状态: {}", e))?;
    Ok(base_dir
        .join("state")
        .join("deps_sync_markers")
        .join(format!("{}.done", container_code)))
}

fn has_deps_download_marker<R: Runtime>(app: &AppHandle<R>, container_code: &str) -> bool {
    deps_download_marker_path(app, container_code)
        .map(|path| path.exists())
        .unwrap_or(false)
}

fn write_deps_download_marker<R: Runtime>(
    app: &AppHandle<R>,
    container_code: &str,
) -> Result<(), String> {
    let path = deps_download_marker_path(app, container_code)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 deps 状态目录失败 (path: {}): {}", parent.display(), e))?;
    }
    std::fs::write(&path, b"downloaded=true\n")
        .map_err(|e| format!("写入 deps 下载标记失败 (path: {}): {}", path.display(), e))
}

fn remove_deps_download_marker<R: Runtime>(app: &AppHandle<R>, container_code: &str) {
    if let Ok(path) = deps_download_marker_path(app, container_code) {
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                warn!(
                    "删除过期 deps 下载标记失败 (path: {}): {}",
                    path.display(),
                    e
                );
            }
        }
    }
}

async fn wait_for_inflight_deps_clear(inflight_key: &str, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    loop {
        {
            let inflight = deps_download_inflight().lock().await;
            if !inflight.contains(inflight_key) {
                return Ok(());
            }
        }

        if start.elapsed() >= timeout {
            return Err(format!("等待 deps 同步超时: {}", inflight_key));
        }

        tokio::time::sleep(Duration::from_millis(120)).await;
    }
}

#[derive(Debug, Default)]
struct OverlayValidationReport {
    overlay_total: usize,
    learning_object_total: usize,
    invalid_learning_object_count: usize,
    primary_course_id: Option<String>,
    course_id_stats: Vec<(String, usize)>,
    warnings: Vec<String>,
}

fn validate_overlay_config(
    overlay_config: &OverlayConfig,
    product_code: &str,
) -> Result<OverlayValidationReport, String> {
    let mut report = OverlayValidationReport::default();
    let mut course_id_counts: HashMap<String, usize> = HashMap::new();

    for page in &overlay_config.pages.page {
        for overlay in &page.overlays {
            report.overlay_total += 1;
            if overlay.overlay_type != "learning-object" {
                continue;
            }

            report.learning_object_total += 1;

            match &overlay.learning_object {
                Some(lo) => {
                    let course_id = lo.course_id.trim();
                    let module_id = lo.module_id.trim();
                    if course_id.is_empty() {
                        report.invalid_learning_object_count += 1;
                        if report.warnings.len() < 8 {
                            report.warnings.push(format!(
                                "[{}] product_code={} page={} learning-object courseId is empty",
                                ERR_OVERLAY_INVALID_STRUCTURE, product_code, page.sno
                            ));
                        }
                    } else {
                        *course_id_counts.entry(course_id.to_string()).or_insert(0) += 1;
                    }

                    if module_id.is_empty() {
                        report.invalid_learning_object_count += 1;
                        if report.warnings.len() < 8 {
                            report.warnings.push(format!(
                                "[{}] product_code={} page={} learning-object moduleId is empty",
                                ERR_OVERLAY_INVALID_STRUCTURE, product_code, page.sno
                            ));
                        }
                    }
                }
                None => {
                    report.invalid_learning_object_count += 1;
                    if report.warnings.len() < 8 {
                        report.warnings.push(format!(
                            "[{}] product_code={} page={} learning-object payload is missing",
                            ERR_OVERLAY_INVALID_STRUCTURE, product_code, page.sno
                        ));
                    }
                }
            }
        }
    }

    let mut course_id_stats: Vec<(String, usize)> = course_id_counts
        .iter()
        .map(|(course_id, count)| (course_id.clone(), *count))
        .collect();
    course_id_stats.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    report.primary_course_id = course_id_stats.first().map(|item| item.0.clone());
    report.course_id_stats = course_id_stats.clone();

    info!(
        "overlay 校验统计: product_code={}, overlay_total={}, learning_object_total={}, invalid_learning_object_count={}, distinct_course_ids={}, course_id_stats={:?}",
        product_code,
        report.overlay_total,
        report.learning_object_total,
        report.invalid_learning_object_count,
        course_id_counts.len(),
        course_id_stats
    );

    if report.primary_course_id.is_none() {
        return Err(coded_error(
            ERR_OVERLAY_NO_COURSE_ID,
            format!(
                "book-overlays.json 缺少有效 learning-object.courseId (product_code: {})",
                product_code
            ),
        ));
    }

    if course_id_counts.len() > 1 {
        report.warnings.push(format!(
            "[{}] product_code={} has multiple courseId values, use primary={}, all={:?}",
            ERR_OVERLAY_INVALID_STRUCTURE,
            product_code,
            report.primary_course_id.as_deref().unwrap_or_default(),
            course_id_stats
        ));
    }

    for warning in &report.warnings {
        warn!("{}", warning);
    }

    Ok(report)
}

fn extract_container_code_from_overlay(
    overlay_config: &OverlayConfig,
    product_code: &str,
) -> Result<String, String> {
    let report = validate_overlay_config(overlay_config, product_code)?;
    let container_code = report.primary_course_id.ok_or_else(|| {
        coded_error(
            ERR_OVERLAY_NO_COURSE_ID,
            format!(
                "book-overlays.json courseId 解析异常 (product_code: {})",
                product_code
            ),
        )
    })?;

    info!(
        "overlay courseId 解析成功: product_code={}, container_code={}, distinct_course_ids={}",
        product_code,
        container_code,
        report.course_id_stats.len()
    );
    Ok(container_code)
}

async fn load_book_overlay_config<R: Runtime>(
    app: &AppHandle<R>,
    config_state: &State<'_, crate::services::config::ConfigState>,
    book_source: &Option<BookSource>,
    books_base_path: &Path,
    product_code: &str,
) -> Result<OverlayConfig, String> {
    let overlay_relative = PathBuf::from(product_code)
        .join("assets")
        .join("imgbook-meta")
        .join("book-overlays.json");
    let overlay_target_path = books_base_path.join(&overlay_relative);
    let mut overlay_path = overlay_target_path.clone();
    if !overlay_path.exists() {
        for base in bundle_books_base_paths(app) {
            let candidate = base.join(&overlay_relative);
            if candidate.exists() {
                overlay_path = candidate;
                break;
            }
        }
    }

    if !overlay_path.exists() {
        if let Some(BookSource::CloudflareR2 { bucket_name, .. }) = book_source {
            let key = format!(
                "books/{}/assets/imgbook-meta/book-overlays.json",
                product_code
            );
            info!(
                "book-overlays.json 缺失，尝试从 R2 下载: product_code={}, key={}, target={}",
                product_code,
                key,
                overlay_target_path.display()
            );

            let r2_state = app.state::<crate::utils::r2::R2ClientState>();
            let client = crate::utils::r2::get_client(config_state, &r2_state).await?;
            let data = crate::utils::r2::get_object(&client, bucket_name, &key)
                .await
                .map_err(|e| {
                    coded_error(
                        ERR_RESOURCE_NOT_FOUND,
                        format!("从 R2 下载 book-overlays.json 失败 (key: {}): {}", key, e),
                    )
                })?;
            write_bytes_to_path(&overlay_target_path, &data)?;
            overlay_path = overlay_target_path;
        } else {
            return Err(coded_error(
                ERR_RESOURCE_NOT_FOUND,
                format!(
                    "找不到 book-overlays.json (product_code: {}, path: {})",
                    product_code,
                    overlay_path.display()
                ),
            ));
        }
    }

    MetadataService::parse_overlays(&overlay_path).map_err(|e| {
        coded_error(
            ERR_OVERLAY_INVALID_STRUCTURE,
            format!(
                "解析 book-overlays.json 失败 (product_code: {}, path: {}): {}",
                product_code,
                overlay_path.display(),
                e
            ),
        )
    })
}

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
        if candidate_def.exists() && candidate_book_json.exists() {
            Some(candidate.clone())
        } else {
            None
        }
    });

    if ebook_path.is_none() {
        if let Some(BookSource::CloudflareR2 { bucket_name, .. }) = &book_source {
            info!("元数据缺失，尝试从 R2 下载...");
            let r2_state = app.state::<crate::utils::r2::R2ClientState>();
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

            if let Ok(data) = crate::utils::r2::get_object(&client, bucket_name, &def_key).await {
                write_bytes_to_path(&def_path, &data)?;

                // 下载 book.json
                let bj_key = format!("books/{}/assets/imgbook-meta/book.json", product_code);
                if let Ok(data) = crate::utils::r2::get_object(&client, bucket_name, &bj_key).await
                {
                    write_bytes_to_path(&bj_path, &data)?;
                }

                // 尝试下载可选的 overlays
                let ov_key = format!(
                    "books/{}/assets/imgbook-meta/book-overlays.json",
                    product_code
                );
                if let Ok(data) = crate::utils::r2::get_object(&client, bucket_name, &ov_key).await
                {
                    write_bytes_to_path(&ov_path, &data)?;
                }
            } else {
                return Err(coded_error(
                    ERR_RESOURCE_NOT_FOUND,
                    format!("从 R2 下载书籍元数据失败。Key: {}", def_key),
                ));
            }

            if def_path.exists() && bj_path.exists() {
                ebook_path = Some(target_path);
            }
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
                    if p.exists() { Some(p) } else { None }
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

                if con_def_path.is_none() {
                    if let Some(BookSource::CloudflareR2 { bucket_name, .. }) = &book_source {
                        let r2_state = app.state::<crate::utils::r2::R2ClientState>();
                        let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;
                        // 统一下载到 single 结构
                        let target_con_path = ebook_con_path_target.clone();
                        let p = target_con_path.join("meta").join("definition.json");

                        let con_def_key =
                            format!("courses/{}/meta/definition.json", container_code);

                        if let Ok(data) =
                            crate::utils::r2::get_object(&client, bucket_name, &con_def_key).await
                        {
                            write_bytes_to_path(&p, &data)?;
                            con_def_path = Some(p);
                        }
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
                .exists()
        })
        .cloned()
        .unwrap_or_else(|| writable_ebook_path.clone());
    let book_json_path = ebook_path
        .join("assets")
        .join("imgbook-meta")
        .join("book.json");

    if matches!(book_source, Some(BookSource::CloudflareR2 { .. }))
        && (!ebook_path.exists() || !book_json_path.exists())
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

    if !book_json_path.exists() {
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
    let resolved_image_path = if image_path.exists() {
        image_path
    } else if cache_image_path.exists() {
        cache_image_path.clone()
    } else {
        if let Some(BookSource::CloudflareR2 { bucket_name, .. }) = book_source {
            info!(
                "资源文件缺失，尝试从 R2 下载: product_code={}, key_path={}, read_path={}, cache_path={}",
                product_code,
                image_rel_path_str,
                image_path.display(),
                cache_image_path.display()
            );
            let r2_state = app.state::<crate::utils::r2::R2ClientState>();
            let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

            let key = format!("books/{}/{}", product_code, image_rel_path_str);

            if let Ok(data) = crate::utils::r2::get_object(&client, &bucket_name, &key).await {
                write_bytes_to_path(&cache_image_path, &data)?;
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
        }
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
        if path.exists() {
            asset_path = Some(path.clone());
            break;
        }
    }

    if asset_path.is_none() {
        if let Some(BookSource::CloudflareR2 { bucket_name, .. }) = book_source {
            info!("资源文件缺失，尝试从 R2 下载: {}", safe_rel_path_str);
            let r2_state = app.state::<crate::utils::r2::R2ClientState>();
            let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

            // 尝试下载两个可能的 Key：直接路径和 assets/ 下的路径
            let keys = [
                format!("books/{}/assets/{}", product_code, safe_rel_path_str),
                format!("books/{}/{}", product_code, safe_rel_path_str),
            ];

            let mut img_data = None;
            let mut final_path: Option<PathBuf> = None;

            for (i, key) in keys.iter().enumerate() {
                if let Ok(data) = crate::utils::r2::get_object(&client, &bucket_name, key).await {
                    img_data = Some(data);
                    // 如果是用 assets/ 开头的 Key 下载成功的，保存到 assets/ 子目录
                    final_path = Some(if i == 0 {
                        ebook_path.join("assets").join(&safe_rel_path)
                    } else {
                        ebook_path.join(&safe_rel_path)
                    });
                    break;
                }
            }

            if let (Some(data), Some(path)) = (img_data, final_path) {
                write_bytes_to_path(&path, &data)?;
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

pub async fn resolve_exercise_resource<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    resource_id: String,
) -> Result<String, String> {
    resolve_exercise_resource_internal(app, config_state, product_code, resource_id, false).await
}

async fn resolve_exercise_resource_internal<R: Runtime>(
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
        // Try to download definition from R2 if missing
        if let Some(BookSource::CloudflareR2 { bucket_name, .. }) = &book_source {
            let r2_state = app.state::<crate::utils::r2::R2ClientState>();
            let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;
            let target_path = cache_container_path.join("meta").join("definition.json");
            let key = format!("courses/{}/meta/definition.json", container_code);

            if let Ok(data) = crate::utils::r2::get_object(&client, bucket_name, &key).await {
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

    // Locations to search for index.html based on the URL path from definition.json
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

    // If not found locally, and we have R2 source, try to download from R2
    if let Some(BookSource::CloudflareR2 { bucket_name, .. }) = &book_source {
        info!(
            "本地缺失练习资源 (url: {})，尝试从 R2 下载",
            safe_url_rel_str
        );
        let r2_state = app.state::<crate::utils::r2::R2ClientState>();
        let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

        // 构造 R2 中的前缀: courses/{container_code}/assets/{url}
        // 不加末尾斜杠，增加匹配成功率
        let prefix = format!("courses/{}/assets/{}", container_code, safe_url_rel_str);

        // 获取该前缀下的所有文件
        let objects = crate::utils::r2::list_objects(&client, bucket_name, Some(&prefix))
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
            bucket_name,
            objects,
            cache_container_path.clone(),
            &container_code,
            &product_code,
            Some(&resource_id),
            "resource",
            emit_progress,
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

        if cache_index_path.exists() {
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseHtmlResponse {
    pub html: String,
    pub url: String,
}

pub async fn get_exercise_html<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    resource_id: String,
) -> Result<ExerciseHtmlResponse, String> {
    info!("正在准备内联练习 HTML (resource_id: {})", resource_id);

    // 1. 先确保 deps 下载完成（优先级高于练习包）
    info!(
        "练习加载前检查 deps 就绪状态: product_code={}, resource_id={}",
        product_code, resource_id
    );
    trigger_deps_download(
        app.clone(),
        product_code.clone(),
        Some(resource_id.clone()),
        true,
        true,
    )
    .await?;
    info!(
        "练习加载前 deps 同步完成: product_code={}, resource_id={}",
        product_code, resource_id
    );

    // 2. deps 完成后，再解析/下载练习包并获取 index.html
    let index_path_str = resolve_exercise_resource_internal(
        app.clone(),
        config_state.clone(),
        product_code.clone(),
        resource_id.clone(),
        true,
    )
    .await?;

    let index_path = PathBuf::from(&index_path_str);
    let index_path = index_path.canonicalize().unwrap_or(index_path);
    let index_dir = index_path.parent().ok_or("无法获取练习目录")?;
    let assets_dir = index_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("无法获取 assets 目录")?;

    if !local_deps_ready(assets_dir) {
        return Err(format!(
            "练习依赖尚未就绪，请稍后重试 (assets: {})",
            assets_dir.display()
        ));
    }

    // 2. 读取 HTML 和 data.js 内容
    let mut html =
        std::fs::read_to_string(&index_path).map_err(|e| format!("读取 index.html 失败: {}", e))?;

    let data_js_path = index_dir.join("data.js");
    let data_js_content_raw = if data_js_path.exists() {
        std::fs::read_to_string(&data_js_path).ok()
    } else {
        None
    };

    // 3. 构造资产路径转换工具
    let to_exercise_asset_url = |p: &std::path::Path| {
        #[cfg(target_os = "windows")]
        {
            format!(
                "eiuasset://localhost/{}",
                p.to_string_lossy()
                    .replace("\\", "/")
                    .replace(" ", "%20")
                    .replace(":", "%3A")
            )
        }
        #[cfg(not(target_os = "windows"))]
        {
            format!(
                "eiuasset://localhost{}",
                p.to_string_lossy().replace(" ", "%20")
            )
        }
    };

    let engine_dir = assets_dir.join("deps").join("engine");
    let dp_dir = assets_dir.join("deps").join("dp");
    let media_dir = index_dir.join("media");
    let engine_base_url = to_exercise_asset_url(&engine_dir);
    let dp_base_url = to_exercise_asset_url(&dp_dir);
    let index_dir_url = format!(
        "{}/",
        to_exercise_asset_url(index_dir).trim_end_matches('/')
    );
    let index_url = format!("{}index.html", index_dir_url);
    let media_base_url = to_exercise_asset_url(&media_dir);

    // srcdoc 在打包环境下通常处于 tauri:// 协议，`//host/path` 会被错误解析为 tauri://host/path。
    // 这里统一升级为 https://，避免练习资源在 release 包中空白。
    html = normalize_protocol_relative_urls(&html);
    let data_js_content = data_js_content_raw.as_ref().map(|raw| {
        let sanitized = sanitize_data_js_media_content(raw, &media_base_url);
        normalize_protocol_relative_urls(&sanitized)
    });

    // 4. 内联 data.js
    if let Some(content) = data_js_content {
        let re_data_script =
            Regex::new(r#"<script\s+[^>]*src=['\"]data\.js['\"][^>]*></script>"#).unwrap();
        let inline_script = format!("<script type=\"text/javascript\">\n{}\n</script>", content);
        html = re_data_script
            .replace(&html, inline_script.as_str())
            .to_string();
    }

    // 5. 若本地 deps 已就绪，则替换为本地路径；否则保留原始远程配置并后台预热
    let normalize_engine_version = |v: &str| v.split_whitespace().collect::<String>();
    let mut resolved_engine_version: Option<String> = None;

    if engine_dir.exists() {
        let mut available_engine_versions: Vec<String> = std::fs::read_dir(&engine_dir)
            .map_err(|e| format!("读取引擎目录失败: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        available_engine_versions.sort();

        if available_engine_versions.is_empty() {
            warn!(
                "本地引擎目录为空，暂不替换为本地 deps: {}",
                engine_dir.display()
            );
        } else {
            let re_engine = Regex::new(
                r#"A5\.ENGINE_ROOT\s*=\s*['\"](?:(?:https?:)?//)[^/]+/(buckminster-[^/'\"]+)/?['\"]"#,
            )
            .map_err(|e| e.to_string())?;
            let preferred_engine_version_raw = re_engine
                .captures(&html)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()));

            let mut selected = preferred_engine_version_raw
                .as_ref()
                .and_then(|raw| {
                    available_engine_versions
                        .iter()
                        .find(|v| *v == raw)
                        .cloned()
                        .or_else(|| {
                            let normalized_raw = normalize_engine_version(raw);
                            available_engine_versions
                                .iter()
                                .find(|v| normalize_engine_version(v) == normalized_raw)
                                .cloned()
                        })
                })
                .or_else(|| available_engine_versions.last().cloned());

            let has_player_js =
                |version: &str| -> bool { engine_dir.join(version).join("player.js").exists() };
            if let Some(version) = selected.as_ref() {
                if !has_player_js(version) {
                    selected = available_engine_versions
                        .iter()
                        .rev()
                        .find(|candidate| has_player_js(candidate))
                        .cloned();
                }
            }

            if let Some(version) = selected {
                let resolved_engine_root = format!("{}/{}/", engine_base_url, version);
                if re_engine.is_match(&html) {
                    html = re_engine
                        .replace_all(
                            &html,
                            format!("A5.ENGINE_ROOT = '{}'", resolved_engine_root),
                        )
                        .to_string();
                } else {
                    let inject = format!(
                        "<script>A5=A5||{{}};A5.ENGINE_ROOT='{}';</script>",
                        resolved_engine_root
                    );
                    if let Some(pos) = html.find("</head>") {
                        html.insert_str(pos, &inject);
                    } else {
                        html.push_str(&inject);
                    }
                }

                let re_geogebra =
                    Regex::new(r#"A5\.GEOGEBRA\s*=\s*['\"]https?://[^/]+/geogebra/(.*)['\"]"#)
                        .map_err(|e| e.to_string())?;
                html = re_geogebra
                    .replace_all(&html, |caps: &regex::Captures| {
                        format!("A5.GEOGEBRA = '{}/geogebra/{}/'", engine_base_url, &caps[1])
                    })
                    .to_string();

                let re_mathjax =
                    Regex::new(r#"A5\.MATHJAX\s*=\s*['\"]https?://[^/]+/mathjax/(.*)['\"]"#)
                        .map_err(|e| e.to_string())?;
                html = re_mathjax
                    .replace_all(&html, |caps: &regex::Captures| {
                        format!("A5.MATHJAX = '{}/mathjax/{}/'", engine_base_url, &caps[1])
                    })
                    .to_string();

                resolved_engine_version = Some(version);
            } else {
                warn!(
                    "未找到可用本地引擎版本，保持远程引擎配置: {}",
                    engine_dir.display()
                );
            }
        }
    } else {
        warn!(
            "本地引擎目录不存在，保持远程引擎配置: {}",
            engine_dir.display()
        );
    }

    if dp_dir.exists() {
        let mut available_dp_versions: Vec<String> = std::fs::read_dir(&dp_dir)
            .map_err(|e| format!("读取皮肤目录失败: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        available_dp_versions.sort();

        if available_dp_versions.is_empty() {
            warn!("本地皮肤目录为空，保持远程皮肤配置: {}", dp_dir.display());
        } else {
            let has_dp_entry = |version: &str| -> bool {
                let root = dp_dir.join(version);
                root.join("css").join("style.css").exists()
                    || root.join("js.js").exists()
                    || root.join("templates.js").exists()
            };

            let resolved_dp_version = available_dp_versions
                .iter()
                .rev()
                .find(|version| has_dp_entry(version))
                .cloned()
                .or_else(|| available_dp_versions.last().cloned());

            if let Some(dp_version) = resolved_dp_version {
                let resolved_skin_root = format!("{}/{}/", dp_base_url, dp_version);
                let re_skin = Regex::new(r#"A5\.SKIN_URL\s*=\s*['\"][^'\"]+['\"]"#)
                    .map_err(|e| e.to_string())?;
                if re_skin.is_match(&html) {
                    html = re_skin
                        .replace_all(&html, format!("A5.SKIN_URL = '{}'", resolved_skin_root))
                        .to_string();
                } else {
                    let inject = format!(
                        "<script>A5=A5||{{}};A5.SKIN_URL='{}';</script>",
                        resolved_skin_root
                    );
                    if let Some(pos) = html.find("</head>") {
                        html.insert_str(pos, &inject);
                    } else {
                        html.push_str(&inject);
                    }
                }
            } else {
                warn!(
                    "未找到可用本地皮肤版本，保持远程皮肤配置: {}",
                    dp_dir.display()
                );
            }
        }
    } else {
        warn!("本地皮肤目录不存在，保持远程皮肤配置: {}", dp_dir.display());
    }

    let re_asset_cdn =
        Regex::new(r#"LOInfo\.assetCDNURL\s*=\s*['\"][^'\"]*['\"]"#).map_err(|e| e.to_string())?;
    html = re_asset_cdn
        .replace_all(&html, format!("LOInfo.assetCDNURL = '{}'", media_base_url))
        .to_string();

    // 6. 将启动脚本改为本地 deps，确保不依赖外网
    let launcher_file_name = "launcher-connector-channel.bundle.js";

    let resolve_local_launcher = || -> Option<PathBuf> {
        let preferred = engine_dir
            .join(resolved_engine_version.as_ref()?)
            .join(launcher_file_name);
        if preferred.exists() {
            return Some(preferred);
        }

        let mut candidates: Vec<PathBuf> = std::fs::read_dir(&engine_dir)
            .ok()?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() && path.join(launcher_file_name).exists())
            .collect();
        candidates.sort();
        candidates.pop().map(|dir| dir.join(launcher_file_name))
    };

    if let Some(local_launcher_path) = resolve_local_launcher() {
        let local_launcher_url = to_exercise_asset_url(&local_launcher_path);
        let re_launcher_script = Regex::new(
            r#"(?is)<script\b[^>]*\bsrc=['\"][^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"][^>]*>\s*</script>"#,
        )
        .map_err(|e| e.to_string())?;
        let replaced_script = re_launcher_script.is_match(&html);
        html = re_launcher_script
            .replace_all(
                &html,
                format!(
                    "<script type=\"text/javascript\" src=\"{}\"></script>",
                    local_launcher_url
                ),
            )
            .to_string();

        // Also rewrite launcher URLs that may appear in inline JS/data blocks.
        // Only touch quoted remote/protocol-relative urls to avoid corrupting eiuasset:// urls.
        let re_launcher_url_quoted = Regex::new(
            r#"(?:'(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?'|\"(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?\")"#,
        )
        .map_err(|e| e.to_string())?;
        html = re_launcher_url_quoted
            .replace_all(&html, |caps: &regex::Captures| {
                let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("'");
                let quote = if matched.starts_with('"') { "\"" } else { "'" };
                format!("{quote}{local_launcher_url}{quote}")
            })
            .to_string();

        let re_launcher_tag =
            Regex::new(r#"LOInfo\.launcherTag\s*=\s*['\"][^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"]"#)
                .map_err(|e| e.to_string())?;
        html = re_launcher_tag
            .replace_all(
                &html,
                format!("LOInfo.launcherTag = '{}'", local_launcher_url),
            )
            .to_string();

        if !replaced_script {
            warn!(
                "未在 HTML 中命中 launcher script 标签，已尝试全局 URL 重写: resource_id={}, local_launcher={}",
                resource_id,
                local_launcher_path.display()
            );
        }

        let re_remote_launcher_remaining = Regex::new(
            r#"(?is)<script\b[^>]*\bsrc=['\"](?:(?:https?:)?//)[^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"]|LOInfo\.launcherTag\s*=\s*['\"](?:(?:https?:)?//)[^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"]|'(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?'|\"(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?\""#,
        )
        .map_err(|e| e.to_string())?;
        if re_remote_launcher_remaining.is_match(&html) {
            warn!(
                "练习 HTML 仍包含远程 launcher 地址，已保留兜底: resource_id={}",
                resource_id
            );
        }
    } else if resolved_engine_version.is_some() {
        error!(
            "未找到本地 launcher 脚本: {:?}",
            engine_dir.join("*/launcher-connector-channel.bundle.js")
        );
    }

    // 7. 同源 URL 环境下不再需要注入 base 标签，移除相关逻辑以避免干扰路径解析

    // 8. 补全 script 标签的 crossorigin，避免生产环境 WebKit 报 opaques script error 并阻止执行
    let re_script_crossorigin =
        Regex::new(r#"(?is)<script\b([^>]*\bsrc=['\"][^'\"]*['\"])([^>]*)>"#).unwrap();
    html = re_script_crossorigin
        .replace_all(&html, |caps: &regex::Captures| {
            let before_src = &caps[1];
            let after_src = &caps[2];
            if before_src.contains("crossorigin=") || after_src.contains("crossorigin=") {
                format!("<script{before_src}{after_src}>")
            } else {
                format!("<script{before_src} crossorigin=\"anonymous\"{after_src}>")
            }
        })
        .to_string();

    // 9. 注入同源布局修复及 URI 容错脚本 (Runtime Guard)
    // 必须在后端注入，确保 eiuasset:// 加载时包含此逻辑
    let runtime_guard = r#"
<script data-eiu-runtime-guard="1">
;(function () {
  var LOG_TYPE = 'eiu-exercise-runtime-log';
  function postRuntimeLog(level, payload) {
    try {
      if (!window.parent || typeof window.parent.postMessage !== 'function') return;
      window.parent.postMessage(JSON.stringify({ type: LOG_TYPE, level: level, payload: payload }), '*');
    } catch (e) {}
  }

  // 1. 代理 Console 并捕获错误
  (function ProxyConsoleAndErrors() {
    var levels = ['log', 'info', 'warn', 'error'];
    levels.forEach(function(level) {
      var original = console[level];
      console[level] = function() {
        var args = Array.prototype.slice.call(arguments);
        postRuntimeLog(level === 'log' ? 'info' : level, { message: 'iframe-console', args: args.map(function(a) { 
          try { return (typeof a === 'object') ? JSON.stringify(a).slice(0, 200) : String(a); } catch(e) { return '[unserializable]'; }
        })});
        if (typeof original === 'function') original.apply(console, arguments);
      };
    });
    window.onerror = function(msg, url, line, col, error) {
      postRuntimeLog('error', { message: 'iframe-window-error', details: msg, line: line, col: col, stack: error ? error.stack : '' });
    };
    window.onunhandledrejection = function(event) {
      postRuntimeLog('error', { message: 'iframe-unhandled-rejection', reason: String(event.reason) });
    };
  })();

  // 2. URI 容错补丁 (解决 URIError: URI error)
  (function PatchURI() {
    var fnNames = ['decodeURIComponent', 'decodeURI'];
    fnNames.forEach(function(fnName) {
      var originalFn = window[fnName];
      if (typeof originalFn !== 'function') return;
      window[fnName] = function(value) {
        try { return originalFn.call(window, value); } 
        catch (e) { 
          postRuntimeLog('warn', { message: 'uri-decode-fallback', fn: fnName, value: String(value).slice(0, 100) });
          return String(value); 
        }
      };
    });
  })();

  // 3. 布局修复
  function ensureLayout() {
    try {
      var html = document.documentElement;
      var body = document.body;
      if (!html || !body) return;
      html.style.height = '100%'; html.style.minHeight = '100%';
      body.style.height = '100%'; body.style.minHeight = '100%';
      body.style.margin = '0'; body.style.padding = '0';
      body.style.overflow = 'hidden'; body.style.backgroundColor = 'transparent';
      
      window.dispatchEvent(new Event('resize'));
      
      postRuntimeLog('info', { message: 'layout-patched', bodyHeight: body.clientHeight, bodyWidth: body.clientWidth });
    } catch (e) { postRuntimeLog('error', { message: 'layout-patch-failed', error: String(e) }); }
  }

  window.__eiuRuntimeGuardStarted = true;
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', ensureLayout);
  } else { ensureLayout(); }
  window.addEventListener('load', function() {
    ensureLayout();
    setTimeout(ensureLayout, 500);
    setTimeout(ensureLayout, 2000); 
  });
})();
</script>"#;

    if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, runtime_guard);
    } else {
        html.push_str(runtime_guard);
    }

    crate::utils::exercise_asset::cache_processed_html(index_url.clone(), html.clone());

    Ok(ExerciseHtmlResponse {
        html,
        url: index_url,
    })
}

fn sanitize_data_js_media_content(content: &str, media_base_url: &str) -> String {
    let re_encoded =
        Regex::new(r#"\?filename\\u003d[^"\\<>\s]+"#).expect("valid encoded filename regex");
    let tmp = re_encoded.replace_all(content, "");

    let re_plain = Regex::new(r#"\?filename=[^"'<>\s]+"#).expect("valid plain filename regex");
    let tmp = re_plain.replace_all(&tmp, "").into_owned();

    let re_asset_cdn_encoded =
        Regex::new(r#"\\u003cassetCDNURL\\u003emedia\\u003c/assetCDNURL\\u003e"#)
            .expect("valid encoded assetCDNURL regex");
    let encoded_replacement = format!(
        "\\u003cassetCDNURL\\u003e{}\\u003c/assetCDNURL\\u003e",
        media_base_url
    );

    let content = re_asset_cdn_encoded
        .replace_all(&tmp, encoded_replacement.as_str())
        .into_owned();

    // LearningObjectInfo.xml is often embedded in data.js as unicode-escaped XML.
    // In tauri:// srcdoc context, protocol-relative URLs (`//host/...`) can resolve
    // to tauri://host/... and fail. Normalize escaped and plain XML forms.
    let re_protocol_relative_escaped =
        Regex::new(r#"\\u003e//"#).expect("valid escaped xml protocol-relative regex");
    let content = re_protocol_relative_escaped
        .replace_all(&content, "\\u003ehttps://")
        .into_owned();

    let re_protocol_relative_plain =
        Regex::new(r#">//"#).expect("valid plain xml protocol-relative regex");
    re_protocol_relative_plain
        .replace_all(&content, ">https://")
        .into_owned()
}

fn normalize_protocol_relative_urls(content: &str) -> String {
    let single_quoted =
        Regex::new(r#"'//([^']+)'"#).expect("valid single-quoted protocol-relative regex");
    let content = single_quoted
        .replace_all(content, "'https://$1'")
        .into_owned();

    let double_quoted =
        Regex::new(r#""//([^"]+)""#).expect("valid double-quoted protocol-relative regex");
    double_quoted
        .replace_all(&content, "\"https://$1\"")
        .into_owned()
}

async fn trigger_deps_download<R: Runtime>(
    app: AppHandle<R>,
    product_code: String,
    resource_id: Option<String>,
    emit_progress: bool,
    require_available: bool,
) -> Result<(), String> {
    let config_state = app.state::<crate::services::config::ConfigState>();
    let (book_source, bucket_name, books_base_path) = {
        let config = config_state.0.read().map_err(|e| e.to_string())?;
        let source = config.book_source.clone();
        let books_path = match &source {
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
        let bucket_name = match &source {
            Some(BookSource::CloudflareR2 { bucket_name, .. }) => Some(bucket_name.clone()),
            _ => None,
        };
        Ok::<(Option<BookSource>, Option<String>, PathBuf), String>((
            source,
            bucket_name,
            books_path,
        ))
    }?;

    let bucket_name = match bucket_name {
        Some(b) => b,
        None => return Ok(()), // 没有配置 R2，跳过
    };

    let overlay_config = load_book_overlay_config(
        &app,
        &config_state,
        &book_source,
        &books_base_path,
        &product_code,
    )
    .await?;
    let container_code = extract_container_code_from_overlay(&overlay_config, &product_code)?;
    info!(
        "开始后台同步 deps 资源 (product_code: {}, container: {})",
        product_code, container_code
    );

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("无法获取缓存目录: {}", e))?;
    let cache_container_path = cache_dir.join("courses").join(&container_code);
    let cache_assets_dir = cache_container_path.join("assets");
    let marker_exists = has_deps_download_marker(&app, &container_code);

    if marker_exists && local_deps_ready(&cache_assets_dir) {
        info!(
            "deps 下载标记命中，跳过同步 (container: {}, marker=true)",
            container_code
        );
        if emit_progress {
            emit_exercise_download_progress(
                &app,
                &product_code,
                resource_id.as_deref(),
                "deps",
                1,
                1,
                0,
                true,
            );
        }
        return Ok(());
    }

    if !marker_exists && local_deps_ready(&cache_assets_dir) {
        info!(
            "本地 deps 已就绪，补写下载标记并跳过同步 (container: {})",
            container_code
        );
        if let Err(e) = write_deps_download_marker(&app, &container_code) {
            warn!("写入 deps 下载标记失败: {}", e);
        }
        if emit_progress {
            emit_exercise_download_progress(
                &app,
                &product_code,
                resource_id.as_deref(),
                "deps",
                1,
                1,
                0,
                true,
            );
        }
        return Ok(());
    }

    if marker_exists && !local_deps_ready(&cache_assets_dir) {
        warn!(
            "deps 下载标记存在但本地依赖不完整，清理标记后重新同步 (container: {}, assets={})",
            container_code,
            cache_assets_dir.display()
        );
        remove_deps_download_marker(&app, &container_code);
    }

    let inflight_key = format!("deps:{}", container_code);
    {
        let mut inflight = deps_download_inflight().lock().await;
        if !inflight.insert(inflight_key.clone()) {
            info!(
                "deps 同步任务已在进行中，等待当前任务完成: {}",
                inflight_key
            );
            drop(inflight);
            if emit_progress {
                // The current task is waiting for an in-flight deps sync started elsewhere.
                // Emit a visible "waiting" progress state so UI won't stay at stale 0/0.
                emit_exercise_download_progress(
                    &app,
                    &product_code,
                    resource_id.as_deref(),
                    "deps",
                    1,
                    0,
                    0,
                    false,
                );
            }
            wait_for_inflight_deps_clear(&inflight_key, Duration::from_secs(45)).await?;
            if !local_deps_ready(&cache_assets_dir) && require_available {
                return Err(format!(
                    "等待 deps 同步完成后依赖仍未就绪 (container: {}, assets={})",
                    container_code,
                    cache_assets_dir.display()
                ));
            }
            if local_deps_ready(&cache_assets_dir)
                && !has_deps_download_marker(&app, &container_code)
            {
                if let Err(e) = write_deps_download_marker(&app, &container_code) {
                    warn!("写入 deps 下载标记失败: {}", e);
                }
            }
            if emit_progress {
                emit_exercise_download_progress(
                    &app,
                    &product_code,
                    resource_id.as_deref(),
                    "deps",
                    1,
                    1,
                    0,
                    true,
                );
            }
            return Ok(());
        }
    }

    let result = async {
        let r2_state = app.state::<crate::utils::r2::R2ClientState>();
        let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

        let prefix = format!("courses/{}/assets/deps/", container_code);
        let objects = crate::utils::r2::list_objects(&client, &bucket_name, Some(&prefix)).await?;

        if objects.is_empty() {
            if require_available {
                return Err(format!("R2 上未找到 deps 资源 (prefix: {})", prefix));
            } else {
                info!("R2 上未找到 deps 资源 (prefix: {})", prefix);
                return Ok(());
            }
        }

        info!("同步 deps 中: 发现 {} 个文件需要同步", objects.len());
        let stats = download_r2_objects_concurrently(
            &app,
            &client,
            &bucket_name,
            objects,
            cache_container_path.clone(),
            &container_code,
            &product_code,
            resource_id.as_deref(),
            "deps",
            emit_progress,
        )
        .await;

        info!(
            "后台同步 deps 资源完成 (container: {}, total={}, downloaded={}, skipped={}, failed={})",
            container_code,
            stats.total_files,
            stats.downloaded_files,
            stats.skipped_files,
            stats.failed_files
        );

        if local_deps_ready(&cache_assets_dir) {
            if let Err(e) = write_deps_download_marker(&app, &container_code) {
                warn!("写入 deps 下载标记失败: {}", e);
            } else {
                info!(
                    "deps 下载标记写入成功 (container: {}, marker_dir=app_data/state/deps_sync_markers)",
                    container_code
                );
            }
        } else if require_available {
            return Err(format!(
                "deps 同步完成后依赖仍未就绪 (container: {}, assets={})",
                container_code,
                cache_assets_dir.display()
            ));
        }

        Ok(())
    }
    .await;

    {
        let mut inflight = deps_download_inflight().lock().await;
        inflight.remove(&inflight_key);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{
        ERR_OVERLAY_NO_COURSE_ID, ERR_PATH_OUTSIDE_BASE, normalize_protocol_relative_urls,
        normalize_safe_relative_path, sanitize_data_js_media_content, validate_overlay_config,
    };
    use crate::models::book_metadata::{
        ExerciseInfo, LearningObject, OverlayAudio, OverlayConfig, OverlayItem, OverlayPage,
        OverlayPages, OverlayTargetPage,
    };

    #[test]
    fn test_normalize_protocol_relative_urls_keeps_absolute_urls() {
        let input = r#"
            A5.ENGINE_ROOT = "https://author-engine-production.avallain.net/buckminster-34.0.0/";
            A5.SKIN_URL = '//author-assets-runtime-prod-cup.avallain.net/designpack/RDP_Base/abc/';
            LOInfo.toolBeltURL = "//cup-toolbelt-prod.avallain.net";
        "#;

        let output = normalize_protocol_relative_urls(input);

        assert!(
            output.contains("https://author-engine-production.avallain.net/buckminster-34.0.0/")
        );
        assert!(output.contains("A5.SKIN_URL = 'https://author-assets-runtime-prod-cup.avallain.net/designpack/RDP_Base/abc/'"));
        assert!(output.contains("LOInfo.toolBeltURL = \"https://cup-toolbelt-prod.avallain.net\""));
    }

    #[test]
    fn test_sanitize_data_js_media_content_rewrites_asset_cdn() {
        let input = r#"{"xml":"\u003cassetCDNURL\u003emedia\u003c/assetCDNURL\u003e\u003ctoolbeltServer\u003e//cup-toolbelt-prod.avallain.net\u003c/toolbeltServer\u003e"}"#;
        let media_base_url = "eiuasset://localhost/tmp/media";

        let output = sanitize_data_js_media_content(input, media_base_url);

        assert!(output.contains(
            "\\u003cassetCDNURL\\u003eeiuasset://localhost/tmp/media\\u003c/assetCDNURL\\u003e"
        ));
        assert!(output.contains(
            "\\u003ctoolbeltServer\\u003ehttps://cup-toolbelt-prod.avallain.net\\u003c/toolbeltServer\\u003e"
        ));
    }

    #[test]
    fn test_validate_overlay_config_allows_multiple_course_ids_with_primary() {
        let overlay = OverlayConfig {
            pages: OverlayPages {
                page: vec![
                    OverlayPage {
                        sno: 1,
                        overlays: vec![
                            OverlayItem {
                                x: 0.0,
                                y: 0.0,
                                w: 1.0,
                                h: 1.0,
                                overlay_type: "learning-object".to_string(),
                                audio: None,
                                page: None,
                                learning_object: Some(LearningObject {
                                    course_id: "course-A".to_string(),
                                    module_id: "M1".to_string(),
                                }),
                                exercise: None,
                            },
                            OverlayItem {
                                x: 0.0,
                                y: 0.0,
                                w: 1.0,
                                h: 1.0,
                                overlay_type: "learning-object".to_string(),
                                audio: None,
                                page: None,
                                learning_object: Some(LearningObject {
                                    course_id: "course-B".to_string(),
                                    module_id: "M2".to_string(),
                                }),
                                exercise: None,
                            },
                            OverlayItem {
                                x: 0.0,
                                y: 0.0,
                                w: 1.0,
                                h: 1.0,
                                overlay_type: "learning-object".to_string(),
                                audio: None,
                                page: None,
                                learning_object: Some(LearningObject {
                                    course_id: "course-A".to_string(),
                                    module_id: "M3".to_string(),
                                }),
                                exercise: None,
                            },
                        ],
                    },
                    OverlayPage {
                        sno: 2,
                        overlays: vec![OverlayItem {
                            x: 1.0,
                            y: 1.0,
                            w: 1.0,
                            h: 1.0,
                            overlay_type: "audio".to_string(),
                            audio: Some(OverlayAudio {
                                path: "a.mp3".to_string(),
                                title: Some("A".to_string()),
                            }),
                            page: Some(OverlayTargetPage {
                                pagelabel: "1".to_string(),
                            }),
                            learning_object: None,
                            exercise: Some(ExerciseInfo {
                                name: "ex".to_string(),
                                resource_id: "RE_1".to_string(),
                            }),
                        }],
                    },
                ],
            },
        };

        let report = validate_overlay_config(&overlay, "pcode").expect("should be valid");
        assert_eq!(report.primary_course_id.as_deref(), Some("course-A"));
        assert_eq!(report.course_id_stats.len(), 2);
    }

    #[test]
    fn test_validate_overlay_config_rejects_missing_course_id() {
        let overlay = OverlayConfig {
            pages: OverlayPages {
                page: vec![OverlayPage {
                    sno: 1,
                    overlays: vec![OverlayItem {
                        x: 0.0,
                        y: 0.0,
                        w: 1.0,
                        h: 1.0,
                        overlay_type: "learning-object".to_string(),
                        audio: None,
                        page: None,
                        learning_object: Some(LearningObject {
                            course_id: "".to_string(),
                            module_id: "".to_string(),
                        }),
                        exercise: None,
                    }],
                }],
            },
        };

        let err = validate_overlay_config(&overlay, "pcode").unwrap_err();
        assert!(err.contains(ERR_OVERLAY_NO_COURSE_ID));
    }

    #[test]
    fn test_normalize_safe_relative_path_rejects_parent_dir() {
        let err = normalize_safe_relative_path("../secret/file", "test.path").unwrap_err();
        assert!(err.contains(ERR_PATH_OUTSIDE_BASE));
    }
}
