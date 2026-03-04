use tauri::State;

#[tauri::command]
pub async fn save_study_session(
    state: State<'_, crate::database::DbState>,
    payload: crate::services::study_session::SaveStudySessionPayload,
) -> Result<crate::services::study_session::SaveStudySessionResponse, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_session::save_study_session(db.as_ref(), payload).await
}

#[tauri::command]
pub async fn get_study_stats(
    state: State<'_, crate::database::DbState>,
    period_type: String,
    filters: Option<crate::services::study_session::StudyStatsFilters>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<crate::services::study_session::StudyStatsResponse, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_session::get_study_stats(
        db.as_ref(),
        &period_type,
        filters,
        page,
        page_size,
    )
    .await
}
