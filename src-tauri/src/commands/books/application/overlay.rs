//! 图书 Overlay 配置的加载、结构校验和课程容器解析。

use crate::models::BookSource;
use crate::models::book_metadata::OverlayConfig;
use crate::services::book_metadata::MetadataService;
use log::{info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime, State};

use super::paths::{bundle_books_base_paths, write_bytes_to_path};
use super::{
    ERR_OVERLAY_INVALID_STRUCTURE, ERR_OVERLAY_NO_COURSE_ID, ERR_RESOURCE_NOT_FOUND, coded_error,
};

/// Overlay 校验产生的统计信息和非致命警告。
#[derive(Debug, Default)]
struct OverlayValidationReport {
    overlay_total: usize,
    learning_object_total: usize,
    invalid_learning_object_count: usize,
    primary_course_id: Option<String>,
    course_id_stats: Vec<(String, usize)>,
    warnings: Vec<String>,
}

/// 校验 Overlay 中学习对象的关键字段，并选出出现次数最多的主课程容器。
///
/// # 参数
/// - `overlay_config`：解析后的 Overlay 配置。
/// - `product_code`：用于日志和错误定位的图书产品码。
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

/// 从已校验的 Overlay 配置中提取主课程容器代码。
///
/// # 参数
/// - `overlay_config`：解析后的 Overlay 配置。
/// - `product_code`：用于日志和错误定位的图书产品码。
pub(super) fn extract_container_code_from_overlay(
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

/// 从本地、应用包或 R2 加载图书 Overlay 配置。
///
/// # 参数
/// - `app`：用于访问缓存、资源目录和 R2 状态的应用句柄。
/// - `config_state`：当前应用配置状态。
/// - `book_source`：已解析的图书来源配置。
/// - `books_base_path`：首选图书目录或缓存目录。
/// - `product_code`：目标图书产品码。
pub(super) async fn load_book_overlay_config<R: Runtime>(
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
        if let Some(BookSource::CloudflareGateway {}) = book_source {
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

            let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
            let client = crate::utils::r2::get_client(config_state, &r2_state).await?;
            let data = crate::utils::r2::get_optional_object(&client, &key)
                .await
                .map_err(|e| {
                    coded_error(
                        ERR_RESOURCE_NOT_FOUND,
                        format!("从 R2 下载 book-overlays.json 失败 (key: {}): {}", key, e),
                    )
                })?
                .ok_or_else(|| {
                    coded_error(
                        ERR_RESOURCE_NOT_FOUND,
                        format!("R2 中不存在 book-overlays.json (key: {})", key),
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

#[cfg(test)]
mod tests {
    use super::{ERR_OVERLAY_NO_COURSE_ID, validate_overlay_config};
    use crate::models::book_metadata::{
        ExerciseInfo, LearningObject, OverlayAudio, OverlayConfig, OverlayItem, OverlayPage,
        OverlayPages, OverlayTargetPage,
    };

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
}
