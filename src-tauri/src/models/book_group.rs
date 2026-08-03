use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BookGroup {
    Vocabulary = 1,
    Grammar = 2,
}

impl From<i32> for BookGroup {
    fn from(v: i32) -> Self {
        match v {
            1 => BookGroup::Vocabulary,
            2 => BookGroup::Grammar,
            _ => BookGroup::Vocabulary,
        }
    }
}

impl From<BookGroup> for i32 {
    fn from(value: BookGroup) -> Self {
        value as i32
    }
}

// BookGroup 与数据库整数值之间的自定义序列化。
impl Serialize for BookGroup {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> Deserialize<'de> for BookGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = i32::deserialize(deserializer)?;
        Ok(BookGroup::from(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_group_conversions() {
        assert_eq!(BookGroup::from(1), BookGroup::Vocabulary);
        assert_eq!(BookGroup::from(2), BookGroup::Grammar);
        assert_eq!(BookGroup::from(99), BookGroup::Vocabulary); // Default case

        let v: i32 = BookGroup::Grammar.into();
        assert_eq!(v, 2);
    }

    #[test]
    fn test_book_group_serde() {
        let bg = BookGroup::Grammar;
        let json = serde_json::to_string(&bg).unwrap();
        assert_eq!(json, "2");

        let decoded: BookGroup = serde_json::from_str("1").unwrap();
        assert_eq!(decoded, BookGroup::Vocabulary);
    }
}
