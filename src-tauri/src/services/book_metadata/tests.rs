//! 图书元数据服务回归测试。

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

    let index = MetadataService::build_page_index(&def, &book_json, None, None, None);
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
    assert_eq!(children[0].unit_number, Some(1));
    assert_eq!(children[0].start_page, Some("12".to_string()));
    assert_eq!(children[0].end_page, Some("13".to_string()));
}

#[test]
fn test_parse_toc_unit_number_comes_from_resource_metadata() {
    let data = serde_json::json!({
        "meta": { "title": "Test", "code": "test" },
        "items": {
            "default": [
                {
                    "name": "Later unit",
                    "item-type": "resource",
                    "resource": "RE_00101"
                },
                {
                    "name": "Earlier unit",
                    "item-type": "resource",
                    "resource": "RE_0001"
                },
                {
                    "name": "Appendix 1 Reference",
                    "item-type": "resource",
                    "resource": "RE_00102"
                },
                {
                    "name": "Invalid resource",
                    "item-type": "resource",
                    "resource": "RESOURCE_3"
                }
            ]
        },
        "resources": { "generic": {} }
    });

    let definition: BookDefinition = serde_json::from_value(data).unwrap();
    let toc = MetadataService::parse_toc(&definition, None);

    assert_eq!(toc[0].unit_number, Some(101));
    assert_eq!(toc[1].unit_number, Some(1));
    assert_eq!(toc[2].unit_number, None);
    assert_eq!(toc[3].unit_number, None);
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
    let def_path = Path::new("tests/fixtures/books/essgiuebk/meta/definition.json");
    let res = MetadataService::parse_definition(def_path);
    if let Err(e) = &res {
        panic!("Error parsing definition: {:?}", e);
    }
    let def = res.unwrap();

    let book_path = Path::new("tests/fixtures/books/essgiuebk/assets/imgbook-meta/book.json");
    let res_book = MetadataService::parse_book_json(book_path);
    if let Err(e) = &res_book {
        panic!("Error parsing book.json: {:?}", e);
    }
    let book = res_book.unwrap();

    // 加载课程容器定义。
    let con_def_path = Path::new("tests/fixtures/courses/essgiuebkcon/meta/definition.json");
    let res_con_def = MetadataService::parse_definition(con_def_path);
    let (exercise_mapping, module_mapping) = match res_con_def {
        Ok(con_def) => (
            Some(MetadataService::build_exercise_mapping(&con_def)),
            Some(MetadataService::build_module_mapping(&con_def)),
        ),
        Err(e) => {
            println!("Error loading container definition: {:?}", e);
            (None, None)
        }
    };

    // 加载页面覆盖层配置。
    let overlay_path =
        Path::new("tests/fixtures/books/essgiuebk/assets/imgbook-meta/book-overlays.json");
    let res_overlay = MetadataService::parse_overlays(overlay_path);
    let overlay_config = res_overlay.as_ref().ok();

    let index = MetadataService::build_page_index(
        &def,
        &book,
        exercise_mapping.as_ref(),
        module_mapping.as_ref(),
        overlay_config,
    );
    assert!(!index.is_empty());

    // 第 13 页应包含练习，并可能包含覆盖层。
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

    // 检查目录树中是否存在带音频的节点。
    fn find_node_with_audio(nodes: &[TocNode]) -> Option<&TocNode> {
        for node in nodes {
            if node.audio_files.is_some() {
                return Some(node);
            }
            if let Some(children) = &node.children
                && let Some(found) = find_node_with_audio(children)
            {
                return Some(found);
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
