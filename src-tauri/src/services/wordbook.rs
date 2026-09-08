use crate::database::{Database, SqlStatement, SqlValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordbookEntry {
    #[serde(rename = "wordId", alias = "word_id")]
    pub word_id: String,
    pub word: String,
    #[serde(default, rename = "definitionEng", alias = "definition_eng")]
    pub definition_eng: String,
    #[serde(default, rename = "definitionZh", alias = "definition_zh")]
    pub definition_zh: String,
    #[serde(default, rename = "createdAt", alias = "created_at")]
    pub created_at: String,
}

/// 读取单词本中的全部词条。
pub async fn list(db: &dyn Database) -> Result<Vec<WordbookEntry>, String> {
    let rows = db
        .query("SELECT word_id, word, definition_eng, definition_zh, created_at FROM wordbook_entries ORDER BY created_at DESC".to_string())
        .await
        .map_err(|error| error.to_string())?;
    rows.into_iter()
        .map(|row| serde_json::from_value(row).map_err(|error| error.to_string()))
        .collect()
}

/// 新增或更新单词本词条。
pub async fn add(db: &dyn Database, entry: WordbookEntry) -> Result<(), String> {
    if entry.word_id.trim().is_empty() || entry.word.trim().is_empty() {
        return Err("单词标识和词头不能为空".to_string());
    }
    db.execute_statement(SqlStatement::new(
        "INSERT INTO wordbook_entries (word_id, word, definition_eng, definition_zh) VALUES (?, ?, ?, ?) ON CONFLICT(word_id) DO UPDATE SET word=excluded.word, definition_eng=excluded.definition_eng, definition_zh=excluded.definition_zh",
        vec![
            SqlValue::Text(entry.word_id),
            SqlValue::Text(entry.word),
            SqlValue::Text(entry.definition_eng),
            SqlValue::Text(entry.definition_zh),
        ],
    ))
    .await
    .map_err(|error| error.to_string())
}

/// 按词条标识移除单词本词条。
pub async fn remove(db: &dyn Database, word_id: &str) -> Result<(), String> {
    db.execute_statement(SqlStatement::new(
        "DELETE FROM wordbook_entries WHERE word_id = ?",
        vec![SqlValue::Text(word_id.to_string())],
    ))
    .await
    .map_err(|error| error.to_string())
}
