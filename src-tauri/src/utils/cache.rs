use crate::models::BookGroup;

pub struct CacheKey;

impl CacheKey {
    pub const BOOK_LIST: &'static str = "book_list";

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
