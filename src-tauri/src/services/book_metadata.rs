use crate::models::book_metadata::{BookDefinition, BookJson, ExerciseInfo, PageIndex, TocNode};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct MetadataService;

impl MetadataService {
    pub fn parse_definition(path: &Path) -> anyhow::Result<BookDefinition> {
        let content = fs::read_to_string(path)?;
        let def: BookDefinition = serde_json::from_str(&content)?;
        Ok(def)
    }

    pub fn parse_book_json(path: &Path) -> anyhow::Result<BookJson> {
        let content = fs::read_to_string(path)?;
        let book: BookJson = serde_json::from_str(&content)?;
        Ok(book)
    }

    pub fn build_exercise_mapping(
        container_definition: &BookDefinition,
    ) -> HashMap<String, Vec<ExerciseInfo>> {
        let mut mapping: HashMap<String, Vec<ExerciseInfo>> = HashMap::new();
        Self::recursive_extract_exercises(&container_definition.items.default, &mut mapping);
        mapping
    }

    fn recursive_extract_exercises(
        items: &[crate::models::book_metadata::TocItem],
        mapping: &mut HashMap<String, Vec<ExerciseInfo>>,
    ) {
        for item in items {
            if let Some(res_id) = &item.resource {
                if let Some(label) = Self::extract_page_label_from_name(&item.name) {
                    mapping.entry(label).or_default().push(ExerciseInfo {
                        name: item.name.clone(),
                        resource_id: res_id.clone(),
                    });
                }
            }
            if let Some(sub_items) = &item.items {
                Self::recursive_extract_exercises(sub_items, mapping);
            }
        }
    }

    fn extract_page_label_from_name(name: &str) -> Option<String> {
        // Example: EGIU_PP_U001_P013_x01_Aks.zip -> 13
        // Also handle P002 -> 2
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| regex::Regex::new(r"P(\d{3})").unwrap());

        re.captures(name)
            .map(|cap| {
                let label = cap.get(1).unwrap().as_str();
                label.trim_start_matches('0').to_string()
            })
            .and_then(|s| {
                if s.is_empty() {
                    Some("0".to_string())
                } else {
                    Some(s)
                }
            })
    }

    pub fn parse_overlays(
        path: &Path,
    ) -> anyhow::Result<crate::models::book_metadata::OverlayConfig> {
        let content = fs::read_to_string(path)?;
        let config: crate::models::book_metadata::OverlayConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn build_page_index(
        definition: &BookDefinition,
        book_json: &BookJson,
        exercise_mapping: Option<&HashMap<String, Vec<ExerciseInfo>>>,
        overlay_config: Option<&crate::models::book_metadata::OverlayConfig>,
    ) -> HashMap<String, PageIndex> {
        // Build a mapping from page_label to resource_id
        let mut page_to_resource = HashMap::new();
        for (res_id, res) in &definition.resources.generic {
            if let Some(unit) = &res.imgbook_unit {
                page_to_resource.insert(unit.page_no.clone(), res_id.clone());
            }
        }

        // Build a mapping from sno to overlays (sno is 1-based index in pages.page)
        let mut sno_to_overlays = HashMap::new();
        if let Some(config) = overlay_config {
            for page in &config.pages.page {
                sno_to_overlays.insert(page.sno, page.overlays.clone());
            }
        }

        let xlrg_folder = &book_json.paths.pagexl_lrg_img_folder;
        let mut matched_count = 0;
        let mut total_overlays = 0;

        let pages = book_json
            .pages
            .page
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                let sno = (idx + 1) as i32;
                let resource_id = page_to_resource.get(&p.pagelabel).cloned();
                let exercises = exercise_mapping.and_then(|m| m.get(&p.pagelabel).cloned());
                let overlays = sno_to_overlays.get(&sno).cloned();

                if let Some(ovs) = &overlays {
                    matched_count += 1;
                    total_overlays += ovs.len();
                }

                (
                    p.pagelabel.clone(),
                    PageIndex {
                        label: p.pagelabel.clone(),
                        image_path: format!("{}{}", xlrg_folder, p.bgimage),
                        resource_id,
                        exercises,
                        overlays,
                    },
                )
            })
            .collect();

        log::info!(
            "页面索引构建完成: {} 页匹配到热区, 总计 {} 个热区",
            matched_count,
            total_overlays
        );
        pages
    }

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

                // If it has a resource, look up page range
                if let Some(res_id) = &item.resource {
                    if let Some(res) = definition.resources.generic.get(res_id) {
                        if let Some(unit) = &res.imgbook_unit {
                            node.start_page = Some(unit.start_page_no.clone());
                            node.end_page = Some(unit.end_page_no.clone());
                        }
                    }
                } else if let Some(attribs) = &item.attribs {
                    // Some items might have direct page info in attribs
                    node.start_page = attribs
                        .start_page_no
                        .clone()
                        .or_else(|| attribs.page_no.clone());
                    node.end_page = attribs
                        .end_page_no
                        .clone()
                        .or_else(|| attribs.page_no.clone());
                }

                // If we have a page range, collect audios
                if let (Some(start_str), Some(end_str)) = (&node.start_page, &node.end_page) {
                    if let (Ok(start), Ok(end)) = (start_str.parse::<i32>(), end_str.parse::<i32>())
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
                }

                // Recursive children
                if let Some(sub_items) = &item.items {
                    if !sub_items.is_empty() {
                        let children =
                            Self::recursive_parse_toc(sub_items, definition, page_to_audios);
                        if !children.is_empty() {
                            node.children = Some(children);
                        }
                    }
                }

                node
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_definition() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{
            "meta": {{ "title": "Test", "code": "test" }},
            "items": {{ "default": [] }},
            "resources": {{ "generic": {{}} }}
        }}"#
        )
        .unwrap();

        let res = MetadataService::parse_definition(file.path());
        assert!(res.is_ok());
        assert_eq!(res.unwrap().meta.title, "Test");
    }

    #[test]
    fn test_build_page_index() {
        let def = BookDefinition {
            meta: crate::models::book_metadata::DefinitionMeta {
                title: "Test".to_string(),
                code: Some("test".to_string()),
                productcode: None,
            },
            items: crate::models::book_metadata::DefinitionItems { default: vec![] },
            resources: crate::models::book_metadata::DefinitionResources {
                generic: {
                    let mut m = HashMap::new();
                    m.insert(
                        "RE_0001".to_string(),
                        crate::models::book_metadata::GenericResource {
                            sub_type: "imgbook_unit".to_string(),
                            imgbook_unit: Some(crate::models::book_metadata::ImgbookUnit {
                                page_no: "1".to_string(),
                                start_page_no: "1".to_string(),
                                end_page_no: "1".to_string(),
                            }),
                        },
                    );
                    m
                },
            },
        };

        let book_json = BookJson {
            bookid: "test".to_string(),
            page_width: 100.0,
            page_height: 200.0,
            paths: crate::models::book_metadata::BookPaths {
                pagexl_lrg_img_folder: "images/xlrg/".to_string(),
            },
            pages: crate::models::book_metadata::BookPages {
                page: vec![crate::models::book_metadata::PageInfo {
                    bgimage: "page1.jpg".to_string(),
                    pagelabel: "1".to_string(),
                }],
            },
        };

        let index = MetadataService::build_page_index(&def, &book_json, None, None);
        assert_eq!(index.len(), 1);
        let p1 = index.get("1").unwrap();
        assert_eq!(p1.image_path, "images/xlrg/page1.jpg");
        assert_eq!(p1.resource_id, Some("RE_0001".to_string()));
    }

    #[test]
    fn test_parse_toc() {
        let data = serde_json::json!({
            "meta": { "title": "Test", "code": "test" },
            "items": {
                "default": [
                    {
                        "name": "Folder",
                        "item-type": "folder",
                        "items": [
                            {
                                "name": "Unit 1",
                                "item-type": "item",
                                "resource": "RE_0001"
                            }
                        ]
                    }
                ]
            },
            "resources": {
                "generic": {
                    "RE_0001": {
                        "sub-type": "imgbook_unit",
                        "imgbook_unit": {
                            "page-no": "12",
                            "start-page-no": "12",
                            "end-page-no": "13"
                        }
                    }
                }
            }
        });

        let def: BookDefinition = serde_json::from_value(data).unwrap();
        let toc = MetadataService::parse_toc(&def, None);

        assert_eq!(toc.len(), 1);
        assert_eq!(toc[0].title, "Folder");
        let children = toc[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].title, "Unit 1");
        assert_eq!(children[0].start_page, Some("12".to_string()));
        assert_eq!(children[0].end_page, Some("13".to_string()));
    }

    #[test]
    fn test_parse_toc_with_audios() {
        let def_data = json!({
            "meta": { "title": "Test", "code": "test" },
            "items": {
                "default": [
                    {
                        "name": "Unit 1",
                        "item-type": "item",
                        "resource": "RE_0001"
                    }
                ]
            },
            "resources": {
                "generic": {
                    "RE_0001": {
                        "sub-type": "imgbook_unit",
                        "imgbook_unit": {
                            "page-no": "1",
                            "start-page-no": "1",
                            "end-page-no": "2"
                        }
                    }
                }
            }
        });

        let overlay_data = json!({
            "pages": {
                "page": [
                    {
                        "sno": 1,
                        "overlays": [
                            { "type": "audio", "x": 0, "y": 0, "w": 0, "h": 0, "audio": { "path": "a1.mp3", "title": "Audio 1" } }
                        ]
                    },
                    {
                        "sno": 2,
                        "overlays": [
                            { "type": "audio", "x": 0, "y": 0, "w": 0, "h": 0, "audio": { "path": "a2.mp3" } }
                        ]
                    }
                ]
            }
        });

        let def: BookDefinition = serde_json::from_value(def_data).unwrap();
        let overlay: crate::models::book_metadata::OverlayConfig =
            serde_json::from_value(overlay_data).unwrap();

        let toc = MetadataService::parse_toc(&def, Some(&overlay));

        assert_eq!(toc.len(), 1);
        let audios = toc[0]
            .audio_files
            .as_ref()
            .expect("Should have audio files");
        assert_eq!(audios.len(), 2);
        assert_eq!(audios[0].path, "a1.mp3");
        assert_eq!(audios[1].path, "a2.mp3");
    }

    #[test]
    fn test_parse_actual_files() {
        let def_path = Path::new("../test_data/books/essgiuebk/meta/definition.json");
        let res = MetadataService::parse_definition(def_path);
        if let Err(e) = &res {
            panic!("Error parsing definition: {:?}", e);
        }
        let def = res.unwrap();

        let book_path = Path::new("../test_data/books/essgiuebk/assets/imgbook-meta/book.json");
        let res_book = MetadataService::parse_book_json(book_path);
        if let Err(e) = &res_book {
            panic!("Error parsing book.json: {:?}", e);
        }
        let book = res_book.unwrap();

        // Load container definition
        let con_def_path = Path::new("../test_data/courses/essgiuebkcon/meta/definition.json");
        let res_con_def = MetadataService::parse_definition(con_def_path);
        let exercise_mapping = match res_con_def {
            Ok(con_def) => Some(MetadataService::build_exercise_mapping(&con_def)),
            Err(e) => {
                println!("Error loading container definition: {:?}", e);
                None
            }
        };

        // Load overlay config
        let overlay_path =
            Path::new("../test_data/books/essgiuebk/assets/imgbook-meta/book-overlays.json");
        let res_overlay = MetadataService::parse_overlays(overlay_path);
        let overlay_config = res_overlay.as_ref().ok();

        let index = MetadataService::build_page_index(
            &def,
            &book,
            exercise_mapping.as_ref(),
            overlay_config,
        );
        assert!(!index.is_empty());

        // Page 13 should have exercises and possibly overlays
        if let Some(p13) = index.get("13") {
            assert!(p13.exercises.is_some());
            let exercises = p13.exercises.as_ref().unwrap();
            assert!(!exercises.is_empty());
            println!("Page 13 exercise count: {}", exercises.len());

            if let Some(overlays) = &p13.overlays {
                println!("Page 13 overlay count: {}", overlays.len());
            }
        }

        let toc = MetadataService::parse_toc(&def, overlay_config);
        assert!(!toc.is_empty());

        // Check if any node has audio files
        fn find_node_with_audio(nodes: &[TocNode]) -> Option<&TocNode> {
            for node in nodes {
                if node.audio_files.is_some() {
                    return Some(node);
                }
                if let Some(children) = &node.children {
                    if let Some(found) = find_node_with_audio(children) {
                        return Some(found);
                    }
                }
            }
            None
        }

        if let Some(node) = find_node_with_audio(&toc) {
            println!(
                "Found node with audio: {} (audios: {})",
                node.title,
                node.audio_files.as_ref().unwrap().len()
            );
        } else {
            println!("No nodes with audio found in TOC");
        }
    }
}
