use crate::models::BookGroup;
use std::path::Path;

/// 判断缓存路径是否为非空普通文件。
///
/// # 参数
/// - `path`：待检查的缓存路径。
pub async fn is_non_empty_file(path: &Path) -> bool {
    tokio::fs::symlink_metadata(path)
        .await
        .map(|metadata| metadata.file_type().is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

pub struct CacheKey;

impl CacheKey {
    pub const BOOK_LIST: &'static str = "book_list";

    /// 生成带可选图书分组的书籍列表缓存键。
    ///
    /// # 参数
    /// - `group`：可选的图书分组筛选。
    pub fn book_list(group: Option<BookGroup>) -> String {
        match group {
            Some(g) => format!("{}:{:?}", Self::BOOK_LIST, g),
            None => format!("{}:all", Self::BOOK_LIST),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_list_cache_key() {
        assert_eq!(CacheKey::book_list(None), "book_list:all");
        assert_eq!(
            CacheKey::book_list(Some(BookGroup::Vocabulary)),
            "book_list:Vocabulary"
        );
        assert_eq!(
            CacheKey::book_list(Some(BookGroup::Grammar)),
            "book_list:Grammar"
        );
    }
}
