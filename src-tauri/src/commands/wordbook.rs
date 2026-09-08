use tauri::State;

#[tauri::command]
/// 按添加时间倒序读取单词本。
pub async fn list_wordbook_entries(
    state: State<'_, crate::database::DbState>,
) -> Result<Vec<crate::services::wordbook::WordbookEntry>, String> {
    let db = state.get().await?;
    crate::services::wordbook::list(db.as_ref()).await
}

#[tauri::command]
/// 新增或更新一条单词本记录。
pub async fn add_wordbook_entry(
    state: State<'_, crate::database::DbState>,
    entry: crate::services::wordbook::WordbookEntry,
) -> Result<(), String> {
    let db = state.get().await?;
    crate::services::wordbook::add(db.as_ref(), entry).await
}

#[tauri::command]
/// 按词条标识移除单词本记录。
pub async fn remove_wordbook_entry(
    state: State<'_, crate::database::DbState>,
    word_id: String,
) -> Result<(), String> {
    let db = state.get().await?;
    crate::services::wordbook::remove(db.as_ref(), &word_id).await
}
