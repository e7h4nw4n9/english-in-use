//! 练习资源与学习模块映射构建。

use super::*;

#[derive(Default)]
struct ExerciseExtractStats {
    visited_items: usize,
    resource_items: usize,
    matched_resource_items: usize,
    unmatched_resource_samples: Vec<String>,
}

#[derive(Default)]
struct ModuleExtractStats {
    visited_items: usize,
    resource_items: usize,
    mapped_items: usize,
    missing_item_code_items: usize,
    invalid_module_id_items: usize,
    missing_item_code_samples: Vec<String>,
    invalid_item_code_samples: Vec<String>,
}

impl MetadataService {
    /// 按页面资源标识构建练习映射。
    ///
    /// # 参数
    /// - `container_definition`：解析后的课程定义。
    pub fn build_exercise_mapping(
        container_definition: &BookDefinition,
    ) -> HashMap<String, Vec<ExerciseInfo>> {
        let mut mapping: HashMap<String, Vec<ExerciseInfo>> = HashMap::new();
        let mut stats = ExerciseExtractStats::default();
        Self::recursive_extract_exercises(
            &container_definition.items.default,
            &mut mapping,
            &mut stats,
        );

        let total_exercises: usize = mapping.values().map(std::vec::Vec::len).sum();
        let mut sample_page_labels: Vec<String> = mapping.keys().cloned().collect();
        sample_page_labels.sort();
        sample_page_labels.truncate(8);

        let unmatched_resource_items = stats
            .resource_items
            .saturating_sub(stats.matched_resource_items);
        info!(
            "练习映射构建完成: page_keys={}, total_exercises={}, visited_items={}, resource_items={}, matched_resource_items={}, unmatched_resource_items={}, sample_page_keys={:?}, sample_unmatched_names={:?}",
            mapping.len(),
            total_exercises,
            stats.visited_items,
            stats.resource_items,
            stats.matched_resource_items,
            unmatched_resource_items,
            sample_page_labels,
            stats.unmatched_resource_samples
        );

        if mapping.is_empty() && stats.resource_items > 0 {
            warn!(
                "练习映射为空: 存在带 resource 的条目但未提取到页码，请检查条目命名是否符合 P### 规则"
            );
        }

        mapping
    }

    /// 按模块标识构建练习映射。
    ///
    /// # 参数
    /// - `container_definition`：解析后的课程定义。
    pub fn build_module_mapping(
        container_definition: &BookDefinition,
    ) -> HashMap<String, ExerciseInfo> {
        let mut mapping = HashMap::new();
        let mut stats = ModuleExtractStats::default();
        Self::recursive_extract_module_info(
            &container_definition.items.default,
            &mut mapping,
            &mut stats,
        );

        let mut sample_module_keys: Vec<String> = mapping.keys().cloned().collect();
        sample_module_keys.sort();
        sample_module_keys.truncate(8);

        info!(
            "模块映射构建完成: module_keys={}, visited_items={}, resource_items={}, mapped_items={}, missing_item_code_items={}, invalid_module_id_items={}, sample_module_keys={:?}, sample_missing_item_code_names={:?}, sample_invalid_item_codes={:?}",
            mapping.len(),
            stats.visited_items,
            stats.resource_items,
            stats.mapped_items,
            stats.missing_item_code_items,
            stats.invalid_module_id_items,
            sample_module_keys,
            stats.missing_item_code_samples,
            stats.invalid_item_code_samples
        );

        if mapping.is_empty() && stats.resource_items > 0 {
            warn!("模块映射为空: 存在带 resource 的条目但未提取到可用 module_id");
        }

        mapping
    }

    /// 递归遍历定义树并收集模块到练习信息的映射。
    ///
    /// # 参数
    /// - `items`：当前层级的定义节点。
    /// - `mapping`：需要累积更新的映射。
    /// - `stats`：递归过程累积的解析统计。
    fn recursive_extract_module_info(
        items: &[crate::models::book_metadata::TocItem],
        mapping: &mut HashMap<String, ExerciseInfo>,
        stats: &mut ModuleExtractStats,
    ) {
        for item in items {
            stats.visited_items += 1;
            if let (Some(res_id), Some(item_code)) = (&item.resource, &item.item_code) {
                stats.resource_items += 1;
                // moduleId 通常对应 item-code 的最后一段。
                if let Some(module_id) = item_code
                    .split('/')
                    .next_back()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    mapping.insert(
                        module_id.to_string(),
                        ExerciseInfo {
                            name: item.name.clone(),
                            resource_id: res_id.clone(),
                        },
                    );
                    stats.mapped_items += 1;
                } else {
                    stats.invalid_module_id_items += 1;
                    if stats.invalid_item_code_samples.len() < 8 {
                        stats.invalid_item_code_samples.push(item_code.clone());
                    }
                }
            } else if item.resource.is_some() {
                stats.resource_items += 1;
                stats.missing_item_code_items += 1;
                if stats.missing_item_code_samples.len() < 8 {
                    stats.missing_item_code_samples.push(item.name.clone());
                }
            }
            if let Some(sub_items) = &item.items {
                Self::recursive_extract_module_info(sub_items, mapping, stats);
            }
        }
    }

    /// 递归遍历定义树并收集页面范围内的练习。
    ///
    /// # 参数
    /// - `items`：当前层级的定义节点。
    /// - `mapping`：需要累积更新的映射。
    /// - `stats`：递归过程累积的解析统计。
    fn recursive_extract_exercises(
        items: &[crate::models::book_metadata::TocItem],
        mapping: &mut HashMap<String, Vec<ExerciseInfo>>,
        stats: &mut ExerciseExtractStats,
    ) {
        for item in items {
            stats.visited_items += 1;
            if let Some(res_id) = &item.resource {
                stats.resource_items += 1;
                if let Some(label) = Self::extract_page_label_from_name(&item.name) {
                    mapping.entry(label).or_default().push(ExerciseInfo {
                        name: item.name.clone(),
                        resource_id: res_id.clone(),
                    });
                    stats.matched_resource_items += 1;
                } else if stats.unmatched_resource_samples.len() < 8 {
                    stats.unmatched_resource_samples.push(item.name.clone());
                }
            }
            if let Some(sub_items) = &item.items {
                Self::recursive_extract_exercises(sub_items, mapping, stats);
            }
        }
    }

    /// 从练习资源名称中的页码片段提取页面标签。
    ///
    /// # 参数
    /// - `name`：可能包含页码的资源名称。
    pub(super) fn extract_page_label_from_name(name: &str) -> Option<String> {
        // 例如 EGIU_PP_U001_P013_x01_Aks.zip 对应页面 13，同时兼容 P002 形式。
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| regex::Regex::new(r"P(\d{3})").unwrap());

        re.captures(name)
            .map(|cap| {
                let label = cap.get(1).unwrap().as_str();
                label.trim_start_matches('0').to_string()
            })
            .map(|s| if s.is_empty() { "0".to_string() } else { s })
    }
}
