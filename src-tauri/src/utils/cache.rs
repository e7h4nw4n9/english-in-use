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
