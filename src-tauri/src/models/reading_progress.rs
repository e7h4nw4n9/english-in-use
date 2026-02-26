use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingProgress {
    pub id: i32,
    pub book_id: i32,
    pub resource_id: Option<String>,
    pub page_label: Option<String>,
    pub scale: f64,
    pub offset_x: i32,
    pub offset_y: i32,
    pub updated_at: String,
}

impl ReadingProgress {
    pub fn from_json(value: serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;

        let id = obj.get("id")?.as_i64()? as i32;
        let book_id = obj.get("book_id")?.as_i64()? as i32;
        let resource_id = obj
            .get("resource_id")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let page_label = obj
            .get("page_label")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let scale = obj.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let offset_x = obj.get("offset_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let offset_y = obj.get("offset_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let updated_at = obj.get("updated_at")?.as_str()?.to_string();

        Some(Self {
            id,
            book_id,
            resource_id,
            page_label,
            scale,
            offset_x,
            offset_y,
            updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_reading_progress_from_json() {
        let val = json!({
            "id": 1,
            "book_id": 101,
            "resource_id": "RE_001",
            "page_label": "1",
            "scale": 1.5,
            "offset_x": 100,
            "offset_y": 200,
            "updated_at": "2023-10-27 10:00:00"
        });

        let progress = ReadingProgress::from_json(val).unwrap();
        assert_eq!(progress.id, 1);
        assert_eq!(progress.book_id, 101);
        assert_eq!(progress.resource_id, Some("RE_001".to_string()));
        assert_eq!(progress.scale, 1.5);
        assert_eq!(progress.offset_x, 100);
        assert_eq!(progress.offset_y, 200);
    }
}
