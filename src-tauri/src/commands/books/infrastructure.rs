use log::warn;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Semaphore;

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

#[derive(Clone, Copy)]
pub(super) struct DownloadProgress<'a> {
    product_code: &'a str,
    resource_id: Option<&'a str>,
    stage: &'a str,
}

impl<'a> DownloadProgress<'a> {
    /// 创建练习资源下载进度上下文。
    pub fn new(product_code: &'a str, resource_id: Option<&'a str>, stage: &'a str) -> Self {
        Self {
            product_code,
            resource_id,
            stage,
        }
    }

    /// 计算并发送一次下载进度事件。
    pub fn emit<R: Runtime>(
        self,
        app: &AppHandle<R>,
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
        emit_exercise_download_progress(
            app,
            ExerciseDownloadProgressEvent {
                product_code: self.product_code.to_string(),
                resource_id: self.resource_id.map(str::to_string),
                stage: self.stage.to_string(),
                total_files,
                completed_files,
                failed_files,
                percent,
                done,
            },
        );
    }
}

pub(super) struct DownloadBatchOptions<'a> {
    pub cache_container_path: PathBuf,
    pub container_code: &'a str,
    pub progress: Option<DownloadProgress<'a>>,
}

#[derive(Default)]
pub(super) struct DownloadBatchStats {
    pub total_files: usize,
    pub completed_files: usize,
    pub downloaded_files: usize,
    pub skipped_files: usize,
    pub failed_files: usize,
}

enum DownloadTaskResult {
    Downloaded,
    Skipped,
}

/// 将 R2 对象键转换为容器内的安全相对路径。
fn safe_relative_object_path(key: &str, container_code: &str) -> Result<PathBuf, String> {
    let prefix = format!("courses/{container_code}/");
    let relative = key
        .strip_prefix(&prefix)
        .ok_or_else(|| format!("对象键不属于预期容器: {key}"))?;

    if relative.is_empty() || relative.contains('\\') || relative.contains('\0') {
        return Err(format!("对象键包含非法路径: {key}"));
    }

    let relative_path = Path::new(relative);
    if relative_path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("对象键包含路径穿越片段: {key}"));
    }

    Ok(relative_path.to_path_buf())
}

fn emit_exercise_download_progress<R: Runtime>(
    app: &AppHandle<R>,
    payload: ExerciseDownloadProgressEvent,
) {
    if let Err(e) = app.emit("exercise-download-progress", payload) {
        warn!("发送练习下载进度事件失败: {}", e);
    }
}

/// 并发下载一批 R2 对象，并持续汇总下载结果和进度。
pub(super) async fn download_r2_objects_concurrently<R: Runtime>(
    app: &AppHandle<R>,
    client: &crate::utils::gateway::GatewayClient,
    keys: Vec<String>,
    options: DownloadBatchOptions<'_>,
) -> DownloadBatchStats {
    let max_concurrent_downloads = if cfg!(target_os = "ios") { 3 } else { 8 };

    let mut pending_downloads = Vec::new();
    let mut skipped_existing = 0usize;
    let mut rejected_keys = 0usize;
    let total_keys = keys.len();

    for key in keys {
        let relative_path = match safe_relative_object_path(&key, options.container_code) {
            Ok(path) => path,
            Err(err) => {
                rejected_keys += 1;
                warn!("拒绝不安全的 R2 对象键: {}", err);
                continue;
            }
        };
        let target_path = options.cache_container_path.join(relative_path);
        if crate::utils::cache::is_non_empty_file(&target_path).await {
            skipped_existing += 1;
        } else {
            pending_downloads.push((key, target_path));
        }
    }

    let mut stats = DownloadBatchStats {
        total_files: total_keys,
        completed_files: skipped_existing + rejected_keys,
        skipped_files: skipped_existing,
        failed_files: rejected_keys,
        ..DownloadBatchStats::default()
    };

    if stats.total_files == 0 {
        if let Some(progress) = options.progress {
            progress.emit(app, 0, 0, 0, true);
        }
        return stats;
    }

    if pending_downloads.is_empty() {
        if let Some(progress) = options.progress {
            progress.emit(
                app,
                stats.total_files,
                stats.completed_files,
                stats.failed_files,
                true,
            );
        }
        return stats;
    }

    if let Some(progress) = options.progress {
        progress.emit(
            app,
            stats.total_files,
            stats.completed_files,
            stats.failed_files,
            false,
        );
    }

    let semaphore = Arc::new(Semaphore::new(max_concurrent_downloads));
    let mut join_set = tokio::task::JoinSet::new();

    for (key, target_path) in pending_downloads {
        let client = client.clone();
        let semaphore = semaphore.clone();

        join_set.spawn(async move {
            let _permit = semaphore.acquire_owned().await.map_err(|e| e.to_string())?;

            if crate::utils::cache::is_non_empty_file(&target_path).await {
                return Ok::<DownloadTaskResult, String>(DownloadTaskResult::Skipped);
            }

            crate::utils::r2::download_object_to_path(&client, &key, &target_path)
                .await
                .map_err(|e| format!("下载对象失败 (key: {}): {}", key, e))?;

            Ok::<DownloadTaskResult, String>(DownloadTaskResult::Downloaded)
        });
    }

    while let Some(joined) = join_set.join_next().await {
        stats.completed_files += 1;

        match joined {
            Ok(Ok(DownloadTaskResult::Downloaded)) => {
                stats.downloaded_files += 1;
            }
            Ok(Ok(DownloadTaskResult::Skipped)) => {
                stats.skipped_files += 1;
            }
            Ok(Err(err)) => {
                stats.failed_files += 1;
                warn!("并发下载任务失败: {}", err);
            }
            Err(err) => {
                stats.failed_files += 1;
                warn!("并发下载任务 Join 失败: {}", err);
            }
        }

        if let Some(progress) = options.progress {
            progress.emit(
                app,
                stats.total_files,
                stats.completed_files,
                stats.failed_files,
                stats.completed_files >= stats.total_files,
            );
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::safe_relative_object_path;
    use std::path::PathBuf;

    #[test]
    fn accepts_object_key_inside_expected_container() {
        assert_eq!(
            safe_relative_object_path("courses/container/books/book.json", "container").unwrap(),
            PathBuf::from("books/book.json")
        );
    }

    #[test]
    fn rejects_path_traversal_and_foreign_prefixes() {
        for key in [
            "courses/container/../config.toml",
            "courses/container/books\\..\\config.toml",
            "/absolute/path",
            "courses/other/book.json",
        ] {
            assert!(
                safe_relative_object_path(key, "container").is_err(),
                "{key}"
            );
        }
    }

    #[tokio::test]
    async fn empty_file_is_not_treated_as_valid_cache() {
        let directory = tempfile::tempdir().unwrap();
        let empty_file = directory.path().join("empty.js");
        let complete_file = directory.path().join("complete.js");
        tokio::fs::write(&empty_file, []).await.unwrap();
        tokio::fs::write(&complete_file, b"content").await.unwrap();

        assert!(!crate::utils::cache::is_non_empty_file(&empty_file).await);
        assert!(crate::utils::cache::is_non_empty_file(&complete_file).await);
    }
}
