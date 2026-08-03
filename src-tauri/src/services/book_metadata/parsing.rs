//! definition.json 与 book.json 的基础反序列化。

use super::*;

impl MetadataService {
    /// 解析图书 definition.json。
    ///
    /// # 参数
    /// - `path`：目标文件或目录路径。
    pub fn parse_definition(path: &Path) -> anyhow::Result<BookDefinition> {
        let content = fs::read_to_string(path)?;
        let def: BookDefinition = serde_json::from_str(&content)?;
        Ok(def)
    }

    /// 解析图书 book.json。
    ///
    /// # 参数
    /// - `path`：目标文件或目录路径。
    pub fn parse_book_json(path: &Path) -> anyhow::Result<BookJson> {
        let content = fs::read_to_string(path)?;
        let book: BookJson = serde_json::from_str(&content)?;
        Ok(book)
    }
}
