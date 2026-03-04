use tauri::State;

#[tauri::command]
pub async fn upsert_study_plan(
    state: State<'_, crate::database::DbState>,
    product_code: String,
    resource_id: String,
    unit_name: String,
) -> Result<crate::services::study_plan::StudyPlanUpsertResponse, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_plan::upsert_study_plan(
        db.as_ref(),
        &product_code,
        &resource_id,
        &unit_name,
    )
    .await
}

#[tauri::command]
pub async fn abandon_study_plan(
    state: State<'_, crate::database::DbState>,
    product_code: String,
    resource_id: String,
) -> Result<crate::services::study_plan::StudyPlanActionResponse, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_plan::abandon_study_plan(db.as_ref(), &product_code, &resource_id).await
}

#[tauri::command]
pub async fn get_study_plan_status(
    state: State<'_, crate::database::DbState>,
    product_code: String,
    resource_id: String,
) -> Result<crate::services::study_plan::StudyPlanStatusResponse, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_plan::get_study_plan_status(db.as_ref(), &product_code, &resource_id)
        .await
}

#[tauri::command]
pub async fn get_study_tasks_summary(
    state: State<'_, crate::database::DbState>,
    range_start: String,
    range_end: String,
    view_mode: String,
) -> Result<crate::services::study_plan::StudyTaskSummaryResponse, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_plan::get_study_tasks_summary(
        db.as_ref(),
        &range_start,
        &range_end,
        &view_mode,
    )
    .await
}

#[tauri::command]
pub async fn get_tasks_by_date(
    state: State<'_, crate::database::DbState>,
    date: String,
) -> Result<Vec<crate::services::study_plan::StudyTaskItem>, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_plan::get_tasks_by_date(db.as_ref(), &date).await
}

#[tauri::command]
pub async fn complete_study_task(
    state: State<'_, crate::database::DbState>,
    task_id: i64,
) -> Result<crate::services::study_plan::CompleteStudyTaskResponse, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::services::study_plan::complete_study_task(db.as_ref(), task_id).await
}
