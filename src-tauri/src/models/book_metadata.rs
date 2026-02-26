use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BookDefinition {
    pub meta: DefinitionMeta,
    pub items: DefinitionItems,
    pub resources: DefinitionResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DefinitionMeta {
    pub title: String,
    pub code: Option<String>,
    pub productcode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionItems {
    pub default: Vec<TocItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TocItem {
    pub name: String,
    pub item_type: String,
    pub resource: Option<String>,
    pub items: Option<Vec<TocItem>>,
    pub attribs: Option<TocItemAttribs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TocItemAttribs {
    pub page_no: Option<String>,
    pub start_page_no: Option<String>,
    pub end_page_no: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionResources {
    pub generic: HashMap<String, GenericResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GenericResource {
    pub sub_type: String,
    #[serde(rename = "imgbook_unit")]
    pub imgbook_unit: Option<ImgbookUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ImgbookUnit {
    pub page_no: String,
    pub start_page_no: String,
    pub end_page_no: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookJson {
    pub bookid: String,
    pub page_width: f64,
    pub page_height: f64,
    pub paths: BookPaths,
    pub pages: BookPages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookPaths {
    pub pagexl_lrg_img_folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookPages {
    pub page: Vec<PageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub bgimage: String,
    pub pagelabel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageIndex {
    pub label: String,
    pub image_path: String,
    pub resource_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exercises: Option<Vec<ExerciseInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlays: Option<Vec<OverlayItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub pages: OverlayPages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayPages {
    pub page: Vec<OverlayPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayPage {
    pub sno: i32,
    pub overlays: Vec<OverlayItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayItem {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(rename = "type")]
    pub overlay_type: String, // "audio" or "page"
    pub audio: Option<OverlayAudio>,
    pub page: Option<OverlayTargetPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayAudio {
    pub path: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayTargetPage {
    pub pagelabel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExerciseInfo {
    pub name: String,
    pub resource_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocNode {
    pub title: String,
    pub key: String,
    pub start_page: Option<String>,
    pub end_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_files: Option<Vec<OverlayAudio>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TocNode>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize_definition() {
        let data = json!({
            "meta": {
                "title": "Test Book",
                "code": "test"
            },
            "items": {
                "default": [
                    {
                        "name": "Unit 1",
                        "item-type": "item",
                        "resource": "RE_0001",
                        "attribs": {
                            "page-no": "12"
                        }
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
        assert_eq!(def.meta.title, "Test Book");
        assert_eq!(def.meta.code, Some("test".to_string()));
        assert_eq!(def.items.default[0].name, "Unit 1");
        assert_eq!(
            def.resources
                .generic
                .get("RE_0001")
                .unwrap()
                .imgbook_unit
                .as_ref()
                .unwrap()
                .page_no,
            "12"
        );
    }

    #[test]
    fn test_deserialize_book_json() {
        let data = json!({
            "bookid": "essgiuebk",
            "pageWidth": 555.84,
            "pageHeight": 748.08,
            "paths": {
                "pagexlLrgImgFolder": "images/xlrg/"
            },
            "pages": {
                "page": [
                    {
                        "bgimage": "9781107480551book-updated13.jpg",
                        "pagelabel": "12"
                    }
                ]
            }
        });

        let book: BookJson = serde_json::from_value(data).unwrap();
        assert_eq!(book.bookid, "essgiuebk");
        assert_eq!(book.pages.page[0].pagelabel, "12");
    }
}
