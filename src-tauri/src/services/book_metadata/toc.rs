//! 图书目录树与音频信息解析。

use super::*;

impl MetadataService {
    /// 将定义树和 Overlay 信息转换为阅读器目录树。
    ///
    /// # 参数
    /// - `definition`：解析后的图书定义。
    /// - `overlay_config`：可选的 Overlay 配置。
    pub fn parse_toc(
        definition: &BookDefinition,
        overlay_config: Option<&crate::models::book_metadata::OverlayConfig>,
    ) -> Vec<TocNode> {
        let mut page_to_audios: HashMap<i32, Vec<crate::models::book_metadata::OverlayAudio>> =
            HashMap::new();

        if let Some(config) = overlay_config {
            for page in &config.pages.page {
                let audios: Vec<_> = page
                    .overlays
                    .iter()
                    .filter_map(|o| o.audio.clone())
                    .collect();
                if !audios.is_empty() {
                    page_to_audios.insert(page.sno, audios);
                }
            }
        }

        Self::recursive_parse_toc(&definition.items.default, definition, &page_to_audios)
    }

    /// 递归转换定义树，并继承页面范围和 Overlay 音频信息。
    ///
    /// # 参数
    /// - `items`：当前层级的定义节点。
    /// - `definition`：用于解析资源页面范围的图书定义。
    /// - `page_to_audios`：按页面标签整理的 Overlay 音频映射。
    fn recursive_parse_toc(
        items: &[crate::models::book_metadata::TocItem],
        definition: &BookDefinition,
        page_to_audios: &HashMap<i32, Vec<crate::models::book_metadata::OverlayAudio>>,
    ) -> Vec<TocNode> {
        items
            .iter()
            .map(|item| {
                let mut node = TocNode {
                    title: item.name.clone(),
                    key: item
                        .resource
                        .clone()
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    start_page: None,
                    end_page: None,
                    audio_files: None,
                    children: None,
                };

                // 节点具有资源标识时优先查找对应页面范围。
                if let Some(res_id) = &item.resource
                    && let Some(res) = definition.resources.generic.get(res_id)
                    && let Some(unit) = &res.imgbook_unit
                {
                    node.start_page = Some(unit.start_page_no.clone());
                    node.end_page = Some(unit.end_page_no.clone());
                }

                if node.start_page.is_none()
                    && let Some(attribs) = &item.attribs
                {
                    // 部分节点会直接在属性中提供页面信息。
                    node.start_page = attribs
                        .start_page_no
                        .clone()
                        .or_else(|| attribs.page_no.clone());
                    node.end_page = attribs
                        .end_page_no
                        .clone()
                        .or_else(|| attribs.page_no.clone());
                }

                // 仍无页面信息时尝试从名称提取，练习节点通常采用这种形式。
                if node.start_page.is_none()
                    && let Some(label) = Self::extract_page_label_from_name(&item.name)
                {
                    node.start_page = Some(label.clone());
                    node.end_page = Some(label);
                }

                // 页面范围确定后收集范围内的音频。
                if let (Some(start_str), Some(end_str)) = (&node.start_page, &node.end_page)
                    && let (Ok(start), Ok(end)) = (start_str.parse::<i32>(), end_str.parse::<i32>())
                {
                    let mut audios = Vec::new();
                    for sno in start..=end {
                        if let Some(page_audios) = page_to_audios.get(&sno) {
                            audios.extend(page_audios.clone());
                        }
                    }
                    if !audios.is_empty() {
                        node.audio_files = Some(audios);
                    }
                }

                // 递归转换子节点。
                if let Some(sub_items) = &item.items
                    && !sub_items.is_empty()
                {
                    let children = Self::recursive_parse_toc(sub_items, definition, page_to_audios);
                    if !children.is_empty() {
                        node.children = Some(children);
                    }
                }

                node
            })
            .collect()
    }
}
