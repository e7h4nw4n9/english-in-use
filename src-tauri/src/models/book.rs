use super::book_group::BookGroup;
use serde::{Deserialize, Serialize};

/// 规范化可选书籍简称，空白内容统一视为未设置。
///
/// # 参数
/// - `value`：数据库或接口提供的原始简称。
pub fn normalize_short_title(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|short_title| !short_title.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: i32,
    pub book_group: BookGroup,
    pub product_code: String,
    pub title: String,
    pub short_title: Option<String>,
    pub author: Option<String>,
    pub product_type: String,
    pub cover: Option<String>,
    pub sort_num: i32,
}

impl Book {
    /// 将数据库 JSON 行转换为图书模型。
    ///
    /// # 参数
    /// - `value`：数据库返回的 JSON 值。
    pub fn from_json(value: serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;

        let id = obj.get("id")?.as_i64()? as i32;
        let book_group_val = obj.get("book_group")?.as_i64()? as i32;
        let product_code = obj.get("product_code")?.as_str()?.to_string();
        let title = obj.get("title")?.as_str()?.to_string();
        let short_title = normalize_short_title(obj.get("short_title").and_then(|v| v.as_str()));
        let author = obj
            .get("author")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let product_type = obj.get("product_type")?.as_str()?.to_string();
        let cover = obj
            .get("cover")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let sort_num = obj.get("sort_num")?.as_i64()? as i32;

        Some(Self {
            id,
            book_group: BookGroup::from(book_group_val),
            product_code,
            title,
            short_title,
            author,
            product_type,
            cover,
            sort_num,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_book_from_json() {
        let val = json!({
            "id": 1,
            "book_group": 2,
            "product_code": "test-code",
            "title": "Test Book",
            "short_title": "Test",
            "author": "Author Name",
            "product_type": "Student's Book",
            "cover": "cover.jpg",
            "sort_num": 10
        });

        let book = Book::from_json(val).unwrap();
        assert_eq!(book.id, 1);
        assert_eq!(book.book_group, BookGroup::Grammar);
        assert_eq!(book.product_code, "test-code");
        assert_eq!(book.short_title, Some("Test".to_string()));
        assert_eq!(book.author, Some("Author Name".to_string()));
    }

    #[test]
    fn test_book_from_json_minimal() {
        let val = json!({
            "id": 1,
            "book_group": 1,
            "product_code": "code",
            "title": "Title",
            "product_type": "Type",
            "sort_num": 0
        });

        let book = Book::from_json(val).unwrap();
        assert_eq!(book.short_title, None);
        assert_eq!(book.author, None);
        assert_eq!(book.cover, None);
    }

    #[test]
    fn test_book_from_json_normalizes_unicode_whitespace_short_title() {
        let val = json!({
            "id": 1,
            "book_group": 1,
            "product_code": "code",
            "title": "Title",
            "short_title": "\t　\n",
            "product_type": "Type",
            "sort_num": 0
        });

        let book = Book::from_json(val).unwrap();
        assert_eq!(book.short_title, None);
    }
}
