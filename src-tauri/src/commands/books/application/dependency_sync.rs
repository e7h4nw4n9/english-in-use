//! 练习运行依赖的下载进度、并发去重和完整性标记管理。

use crate::models::BookSource;
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};

use super::overlay::{extract_container_code_from_overlay, load_book_overlay_config};
use super::{DownloadBatchOptions, DownloadProgress, download_r2_objects_concurrently};

static DEPS_DOWNLOAD_LOCKS: OnceLock<crate::utils::keyed_lock::KeyedAsyncLock<String>> =
    OnceLock::new();

/// 获取进程内共享的依赖同步锁集合。
fn deps_download_locks() -> &'static crate::utils::keyed_lock::KeyedAsyncLock<String> {
    DEPS_DOWNLOAD_LOCKS.get_or_init(Default::default)
}

/// 检查本地是否同时具备可运行的 Engine 与 Design Pack 依赖。
///
/// # 参数
/// - `assets_dir`：课程容器的 assets 目录。
pub(super) fn local_deps_ready(assets_dir: &Path) -> bool {
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

/// 计算课程依赖同步完成标记的存储路径。
///
/// # 参数
/// - `app`：用于解析应用数据目录的应用句柄。
/// - `container_code`：课程容器代码。
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

/// 判断指定课程容器是否已有依赖同步完成标记。
///
/// # 参数
/// - `app`：用于解析标记路径的应用句柄。
/// - `container_code`：课程容器代码。
fn has_deps_download_marker<R: Runtime>(app: &AppHandle<R>, container_code: &str) -> bool {
    deps_download_marker_path(app, container_code)
        .map(|path| path.exists())
        .unwrap_or(false)
}

/// 写入指定课程容器的依赖同步完成标记。
///
/// # 参数
/// - `app`：用于解析标记路径的应用句柄。
/// - `container_code`：课程容器代码。
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

/// 删除已经失效的依赖同步完成标记。
///
/// # 参数
/// - `app`：用于解析标记路径的应用句柄。
/// - `container_code`：课程容器代码。
fn remove_deps_download_marker<R: Runtime>(app: &AppHandle<R>, container_code: &str) {
    if let Ok(path) = deps_download_marker_path(app, container_code)
        && path.exists()
        && let Err(e) = std::fs::remove_file(&path)
    {
        warn!(
            "删除过期 deps 下载标记失败 (path: {}): {}",
            path.display(),
            e
        );
    }
}

/// 同步练习运行依赖，并通过完成标记和按键异步锁避免重复下载。
///
/// # 参数
/// - `app`：用于访问配置、缓存、R2 状态和事件通道的应用句柄。
/// - `product_code`：当前图书产品码。
/// - `resource_id`：可选的练习资源标识。
/// - `emit_progress`：是否向前端发送下载进度。
/// - `require_available`：同步结束后是否强制要求依赖完整可用。
pub(super) async fn trigger_deps_download<R: Runtime>(
    app: AppHandle<R>,
    product_code: String,
    resource_id: Option<String>,
    emit_progress: bool,
    require_available: bool,
) -> Result<(), String> {
    let config_state = app.state::<crate::services::config::ConfigState>();
    let (book_source, uses_gateway, books_base_path) = {
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
        let uses_gateway = matches!(source, Some(BookSource::CloudflareGateway { .. }));
        Ok::<(Option<BookSource>, bool, PathBuf), String>((source, uses_gateway, books_path))
    }?;

    if !uses_gateway {
        return Ok(());
    }

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
    let progress = DownloadProgress::new(&product_code, resource_id.as_deref(), "deps");
    let marker_exists = has_deps_download_marker(&app, &container_code);

    if marker_exists && local_deps_ready(&cache_assets_dir) {
        info!(
            "deps 下载标记命中，跳过同步 (container: {}, marker=true)",
            container_code
        );
        if emit_progress {
            progress.emit(&app, 1, 1, 0, true);
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

    let lock_key = format!("deps:{container_code}");
    let sync_lock = deps_download_locks().get(lock_key.clone())?;
    let _sync_guard = match sync_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            info!("deps 同步任务已在进行中，等待当前任务完成: {lock_key}");
            if emit_progress {
                progress.emit(&app, 1, 0, 0, false);
            }
            tokio::time::timeout(Duration::from_secs(45), sync_lock.lock())
                .await
                .map_err(|_| format!("等待 deps 同步超时: {lock_key}"))?
        }
    };

    if has_deps_download_marker(&app, &container_code) && local_deps_ready(&cache_assets_dir) {
        if emit_progress {
            progress.emit(&app, 1, 1, 0, true);
        }
        return Ok(());
    }

    async {
        let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
        let client = crate::utils::r2::get_client(&config_state, &r2_state).await?;

        let prefix = format!("courses/{}/assets/deps/", container_code);
        let objects = crate::utils::r2::list_objects(&client, Some(&prefix)).await?;

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
            objects,
            DownloadBatchOptions {
                cache_container_path: cache_container_path.clone(),
                container_code: &container_code,
                progress: emit_progress.then_some(progress),
            },
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

        if stats.failed_files == 0 && local_deps_ready(&cache_assets_dir) {
            if let Err(e) = write_deps_download_marker(&app, &container_code) {
                warn!("写入 deps 下载标记失败: {}", e);
            } else {
                info!(
                    "deps 下载标记写入成功 (container: {}, marker_dir=app_data/state/deps_sync_markers)",
                    container_code
                );
            }
        } else {
            remove_deps_download_marker(&app, &container_code);
        }

        if stats.failed_files > 0 && require_available {
            return Err(format!(
                "deps 同步有 {} 个文件失败 (container: {})",
                stats.failed_files, container_code
            ));
        }
        if !local_deps_ready(&cache_assets_dir) && require_available {
            return Err(format!(
                "deps 同步完成后依赖仍未就绪 (container: {}, assets={})",
                container_code,
                cache_assets_dir.display()
            ));
        }

        Ok(())
    }
    .await
}
