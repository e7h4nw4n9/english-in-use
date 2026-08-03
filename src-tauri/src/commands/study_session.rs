use tauri::State;

#[tauri::command]
/// 保存一次学习会话。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `payload`：需要保存的学习会话数据。
pub async fn save_study_session(
    state: State<'_, crate::database::DbState>,
    payload: crate::services::study_session::SaveStudySessionPayload,
) -> Result<crate::services::study_session::SaveStudySessionResponse, String> {
    let db = state.get().await?;
    crate::services::study_session::save_study_session(db.as_ref(), payload).await
}

#[tauri::command]
/// 按周期、筛选条件和分页参数查询学习统计。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `period_type`：统计周期类型。
/// - `filters`：可选的统计筛选条件。
/// - `page`：可选的分页页码。
/// - `page_size`：可选的每页记录数。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_study_stats(
    state: State<'_, crate::database::DbState>,
    period_type: String,
    filters: Option<crate::services::study_session::StudyStatsFilters>,
    page: Option<i64>,
    page_size: Option<i64>,
    local_date: String,
) -> Result<crate::services::study_session::StudyStatsResponse, String> {
    let db = state.get().await?;
    crate::services::study_session::get_study_stats_on_date(
        db.as_ref(),
        &period_type,
        filters,
        page,
        page_size,
        &local_date,
    )
    .await
}

#[tauri::command]
/// 查询指定日期的学习会话明细。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `date`：需要查询的日期。
/// - `filters`：可选的统计筛选条件。
pub async fn get_study_sessions_by_date(
    state: State<'_, crate::database::DbState>,
    date: String,
    filters: Option<crate::services::study_session::StudyStatsFilters>,
) -> Result<Vec<crate::services::study_session::StudySessionListItem>, String> {
    let db = state.get().await?;
    crate::services::study_session::get_study_sessions_by_date(db.as_ref(), &date, filters).await
}
