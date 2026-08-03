//! 覆盖层解析与页面索引构建。

use super::*;

impl MetadataService {
    /// 解析图书 book-overlays.json。
    ///
    /// # 参数
    /// - `path`：目标文件或目录路径。
    pub fn parse_overlays(
        path: &Path,
    ) -> anyhow::Result<crate::models::book_metadata::OverlayConfig> {
        let content = fs::read_to_string(path)?;
        let config: crate::models::book_metadata::OverlayConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 合并定义、页面、练习与 Overlay 数据构建页面索引。
    ///
    /// # 参数
    /// - `definition`：解析后的图书定义。
    /// - `book_json`：解析后的页面元数据。
    /// - `exercise_mapping`：可选的页面练习映射。
    /// - `module_mapping`：可选的模块练习映射。
    /// - `overlay_config`：可选的 Overlay 配置。
    pub fn build_page_index(
        definition: &BookDefinition,
        book_json: &BookJson,
        exercise_mapping: Option<&HashMap<String, Vec<ExerciseInfo>>>,
        module_mapping: Option<&HashMap<String, ExerciseInfo>>,
        overlay_config: Option<&crate::models::book_metadata::OverlayConfig>,
    ) -> HashMap<String, PageIndex> {
        let total_pages = book_json.pages.page.len();

        // 建立页面标签到资源标识的映射。
        let mut page_to_resource = HashMap::new();
        for (res_id, res) in &definition.resources.generic {
            if let Some(unit) = &res.imgbook_unit {
                page_to_resource.insert(unit.page_no.clone(), res_id.clone());
            }
        }

        if let Some(mapping) = exercise_mapping {
            let total_exercises: usize = mapping.values().map(std::vec::Vec::len).sum();
            let book_page_labels: HashSet<&str> = book_json
                .pages
                .page
                .iter()
                .map(|p| p.pagelabel.as_str())
                .collect();

            let matched_page_keys = mapping
                .keys()
                .filter(|label| book_page_labels.contains(label.as_str()))
                .count();
            let unmatched_page_keys = mapping.len().saturating_sub(matched_page_keys);
            let sample_unmatched_page_keys: Vec<String> = mapping
                .keys()
                .filter(|label| !book_page_labels.contains(label.as_str()))
                .take(8)
                .cloned()
                .collect();

            info!(
                "页码匹配统计: book_pages={}, exercise_page_keys={}, total_exercises={}, matched_page_keys={}, unmatched_page_keys={}, sample_unmatched_page_keys={:?}",
                total_pages,
                mapping.len(),
                total_exercises,
                matched_page_keys,
                unmatched_page_keys,
                sample_unmatched_page_keys
            );

            if !mapping.is_empty() && matched_page_keys == 0 {
                warn!("页码匹配失败: exercise_page_keys 与 book pagelabel 无交集");
            }
        } else {
            info!("页码匹配统计: exercise_mapping 缺失，跳过练习页码关联");
        }

        // 建立序号到 Overlay 的映射；sno 是 pages.page 中从 1 开始的索引。
        let mut sno_to_overlays = HashMap::new();
        let overlay_page_count = overlay_config
            .map(|config| config.pages.page.len())
            .unwrap_or(0);
        let mut overlay_count = 0usize;
        let mut learning_object_total = 0usize;
        let mut learning_object_resolved = 0usize;
        let mut learning_object_missing_mapping = 0usize;
        let mut learning_object_missing_payload = 0usize;
        let mut learning_object_module_not_found = 0usize;
        let mut learning_object_unresolved_samples: Vec<String> = Vec::new();

        if let Some(config) = overlay_config {
            for page in &config.pages.page {
                let mut overlays = page.overlays.clone();
                overlay_count += overlays.len();

                // 解析学习对象类型的 Overlay。
                for overlay in &mut overlays {
                    if overlay.overlay_type == "learning-object" {
                        learning_object_total += 1;
                        match (&overlay.learning_object, module_mapping) {
                            (Some(lo), Some(m_map)) => {
                                if let Some(ex_info) = m_map.get(&lo.module_id) {
                                    overlay.exercise = Some(ex_info.clone());
                                    learning_object_resolved += 1;
                                } else {
                                    learning_object_module_not_found += 1;
                                    if learning_object_unresolved_samples.len() < 8 {
                                        learning_object_unresolved_samples.push(format!(
                                            "sno={}, module_id={}, course_id={}",
                                            page.sno, lo.module_id, lo.course_id
                                        ));
                                    }
                                }
                            }
                            (Some(lo), None) => {
                                learning_object_missing_mapping += 1;
                                if learning_object_unresolved_samples.len() < 8 {
                                    learning_object_unresolved_samples.push(format!(
                                        "sno={}, module_id={}, course_id={}, reason=module_mapping_none",
                                        page.sno, lo.module_id, lo.course_id
                                    ));
                                }
                            }
                            (None, _) => {
                                learning_object_missing_payload += 1;
                                if learning_object_unresolved_samples.len() < 8 {
                                    learning_object_unresolved_samples.push(format!(
                                        "sno={}, reason=learning_object_missing",
                                        page.sno
                                    ));
                                }
                            }
                        }
                    }
                }

                sno_to_overlays.insert(page.sno, overlays);
            }

            info!(
                "overlay 关联统计: overlay_pages={}, total_overlays={}, learning_object_total={}, learning_object_resolved={}, learning_object_module_not_found={}, learning_object_missing_mapping={}, learning_object_missing_payload={}, unresolved_samples={:?}",
                overlay_page_count,
                overlay_count,
                learning_object_total,
                learning_object_resolved,
                learning_object_module_not_found,
                learning_object_missing_mapping,
                learning_object_missing_payload,
                learning_object_unresolved_samples
            );

            if learning_object_total > 0 && learning_object_resolved == 0 {
                warn!(
                    "learning-object 关联失败: 存在 learning-object 热区但没有任何一个成功绑定练习资源"
                );
            }
        } else {
            info!("overlay 关联统计: overlay_config 缺失，跳过 learning-object 解析");
        }

        let xlrg_folder = &book_json.paths.pagexl_lrg_img_folder;
        let mut matched_count = 0;
        let mut total_overlays = 0;
        let mut pages_with_resource_id = 0usize;
        let mut pages_with_exercises = 0usize;
        let mut pages_with_generated_overlays = 0usize;
        let mut generated_overlay_count = 0usize;
        let mut overlays_before_generation = 0usize;
        let mut overlays_with_exercise_links = 0usize;

        let pages = book_json
            .pages
            .page
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                let sno = (idx + 1) as i32;
                let resource_id = page_to_resource.get(&p.pagelabel).cloned();
                if resource_id.is_some() {
                    pages_with_resource_id += 1;
                }
                let exercises = exercise_mapping.and_then(|m| m.get(&p.pagelabel).cloned());
                if exercises.is_some() {
                    pages_with_exercises += 1;
                }
                let mut overlays = sno_to_overlays.get(&sno).cloned().unwrap_or_default();
                overlays_before_generation += overlays.len();
                overlays_with_exercise_links += overlays
                    .iter()
                    .filter(|overlay| overlay.exercise.is_some())
                    .count();

                // 将尚未出现在 Overlay 中的练习补充为默认热点。
                let mut page_generated_count = 0usize;
                if let Some(exs) = &exercises {
                    let mut current_y = 60.0; // Start position for generated exercise icons
                    for ex in exs {
                        // 避免重复添加已经关联到 Overlay 的练习。
                        let already_has_overlay = overlays.iter().any(|o| {
                            o.exercise
                                .as_ref()
                                .map(|oe| oe.resource_id == ex.resource_id)
                                .unwrap_or(false)
                        });

                        if !already_has_overlay {
                            overlays.push(crate::models::book_metadata::OverlayItem {
                                x: book_json.page_width - 45.0, // Fixed position on the right
                                y: current_y,
                                w: 36.0,
                                h: 36.0,
                                overlay_type: "exercise".to_string(),
                                audio: None,
                                page: None,
                                learning_object: None,
                                exercise: Some(ex.clone()),
                            });
                            current_y += 40.0; // Stack vertically
                            page_generated_count += 1;
                        }
                    }
                }
                if page_generated_count > 0 {
                    pages_with_generated_overlays += 1;
                    generated_overlay_count += page_generated_count;
                }

                if !overlays.is_empty() {
                    matched_count += 1;
                    total_overlays += overlays.len();
                }

                (
                    p.pagelabel.clone(),
                    PageIndex {
                        label: p.pagelabel.clone(),
                        image_path: format!("{}{}", xlrg_folder, p.bgimage),
                        resource_id,
                        exercises,
                        overlays: if overlays.is_empty() {
                            None
                        } else {
                            Some(overlays)
                        },
                    },
                )
            })
            .collect();

        info!(
            "页面索引构建完成: total_pages={}, pages_with_resource_id={}, pages_with_exercises={}, pages_with_any_overlays={}, total_overlays={}, overlays_before_generation={}, overlays_with_exercise_links={}, generated_overlay_pages={}, generated_overlay_count={}",
            total_pages,
            pages_with_resource_id,
            pages_with_exercises,
            matched_count,
            total_overlays,
            overlays_before_generation,
            overlays_with_exercise_links,
            pages_with_generated_overlays,
            generated_overlay_count
        );
        pages
    }
}
