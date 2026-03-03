use log::warn;
use serde::Serialize;
use std::path::PathBuf;
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

fn emit_exercise_download_progress<R: Runtime>(
    app: &AppHandle<R>,
    payload: ExerciseDownloadProgressEvent,
) {
    if let Err(e) = app.emit("exercise-download-progress", payload) {
        warn!("发送练习下载进度事件失败: {}", e);
    }
}

pub(super) async fn download_r2_objects_concurrently<R: Runtime>(
    app: &AppHandle<R>,
    client: &aws_sdk_s3::Client,
    bucket_name: &str,
    keys: Vec<String>,
    cache_container_path: PathBuf,
    container_code: &str,
    product_code: &str,
    resource_id: Option<&str>,
    stage: &str,
    emit_progress: bool,
) -> DownloadBatchStats {
    let max_concurrent_downloads = if cfg!(target_os = "ios") { 3 } else { 8 };

    let mut pending_keys = Vec::new();
    let mut skipped_existing = 0usize;
    let total_keys = keys.len();

    for key in keys {
        let relative_part = key
            .strip_prefix(&format!("courses/{}/", container_code))
            .unwrap_or(&key);
        let target_path = cache_container_path.join(relative_part);
        if target_path.exists() {
            skipped_existing += 1;
        } else {
            pending_keys.push(key);
        }
    }

    let mut stats = DownloadBatchStats {
        total_files: total_keys,
        completed_files: skipped_existing,
        skipped_files: skipped_existing,
        ..DownloadBatchStats::default()
    };

    if stats.total_files == 0 {
        if emit_progress {
            emit_exercise_download_progress(
                app,
                ExerciseDownloadProgressEvent {
                    product_code: product_code.to_string(),
                    resource_id: resource_id.map(str::to_string),
                    stage: stage.to_string(),
                    total_files: 0,
                    completed_files: 0,
                    failed_files: 0,
                    percent: 100.0,
                    done: true,
                },
            );
        }
        return stats;
    }

    if pending_keys.is_empty() {
        if emit_progress {
            emit_exercise_download_progress(
                app,
                ExerciseDownloadProgressEvent {
                    product_code: product_code.to_string(),
                    resource_id: resource_id.map(str::to_string),
                    stage: stage.to_string(),
                    total_files: stats.total_files,
                    completed_files: stats.completed_files,
                    failed_files: 0,
                    percent: 100.0,
                    done: true,
                },
            );
        }
        return stats;
    }

    if emit_progress {
        let percent = (stats.completed_files as f64 / stats.total_files as f64) * 100.0;
        emit_exercise_download_progress(
            app,
            ExerciseDownloadProgressEvent {
                product_code: product_code.to_string(),
                resource_id: resource_id.map(str::to_string),
                stage: stage.to_string(),
                total_files: stats.total_files,
                completed_files: stats.completed_files,
                failed_files: 0,
                percent,
                done: false,
            },
        );
    }

    let semaphore = Arc::new(Semaphore::new(max_concurrent_downloads));
    let mut join_set = tokio::task::JoinSet::new();

    for key in pending_keys {
        let client = client.clone();
        let bucket_name = bucket_name.to_string();
        let cache_container_path = cache_container_path.clone();
        let container_code = container_code.to_string();
        let semaphore = semaphore.clone();

        join_set.spawn(async move {
            let _permit = semaphore.acquire_owned().await.map_err(|e| e.to_string())?;

            let relative_part = key
                .strip_prefix(&format!("courses/{}/", container_code))
                .unwrap_or(&key);
            let target_path = cache_container_path.join(relative_part);

            if target_path.exists() {
                return Ok::<DownloadTaskResult, String>(DownloadTaskResult::Skipped);
            }

            let data = crate::utils::r2::get_object(&client, &bucket_name, &key)
                .await
                .map_err(|e| format!("下载对象失败 (key: {}): {}", key, e))?;
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建下载目录失败 (path: {}): {}", parent.display(), e))?;
            }
            std::fs::write(&target_path, data).map_err(|e| {
                format!("写入下载文件失败 (path: {}): {}", target_path.display(), e)
            })?;

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

        if emit_progress {
            let percent = (stats.completed_files as f64 / stats.total_files as f64) * 100.0;
            emit_exercise_download_progress(
                app,
                ExerciseDownloadProgressEvent {
                    product_code: product_code.to_string(),
                    resource_id: resource_id.map(str::to_string),
                    stage: stage.to_string(),
                    total_files: stats.total_files,
                    completed_files: stats.completed_files,
                    failed_files: stats.failed_files,
                    percent,
                    done: stats.completed_files >= stats.total_files,
                },
            );
        }
    }

    stats
}
