use tauri::State;

#[tauri::command]
/// 按书籍、状态和分页查询学习安排；query 为列表筛选参数。
pub async fn get_study_arrangements(
    state: State<'_, crate::database::DbState>,
    query: crate::services::study_plan::StudyArrangementQuery,
) -> Result<crate::services::study_plan::StudyArrangementList, String> {
    let db = state.get().await?;
    crate::services::study_plan::get_study_arrangements(db.as_ref(), query).await
}

#[tauri::command]
/// 查询单元全部现存复习安排；plan_unit_id 为计划标识。
pub async fn get_study_arrangement_reviews(
    state: State<'_, crate::database::DbState>,
    plan_unit_id: i64,
) -> Result<Vec<crate::services::study_plan::StudyArrangementReview>, String> {
    let db = state.get().await?;
    crate::services::study_plan::get_study_arrangement_reviews(db.as_ref(), plan_unit_id).await
}

#[tauri::command]
/// 创建或更新当前单元的学习计划。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
/// - `unit_name`：学习单元显示名称。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn upsert_study_plan(
    state: State<'_, crate::database::DbState>,
    product_code: String,
    resource_id: String,
    unit_name: String,
    local_date: String,
) -> Result<crate::services::study_plan::StudyPlanUpsertResponse, String> {
    let db = state.get().await?;
    crate::services::study_plan::upsert_study_plan_on_date(
        db.as_ref(),
        &product_code,
        &resource_id,
        &unit_name,
        &local_date,
    )
    .await
}

#[tauri::command]
/// 放弃指定单元的学习计划。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
pub async fn abandon_study_plan(
    state: State<'_, crate::database::DbState>,
    product_code: String,
    resource_id: String,
) -> Result<crate::services::study_plan::StudyPlanActionResponse, String> {
    let db = state.get().await?;
    crate::services::study_plan::abandon_study_plan(db.as_ref(), &product_code, &resource_id).await
}

#[tauri::command]
/// 查询指定单元在本地日期下的学习计划状态。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_study_plan_status(
    state: State<'_, crate::database::DbState>,
    product_code: String,
    resource_id: String,
    local_date: String,
) -> Result<crate::services::study_plan::StudyPlanStatusResponse, String> {
    let db = state.get().await?;
    crate::services::study_plan::get_study_plan_status_on_date(
        db.as_ref(),
        &product_code,
        &resource_id,
        &local_date,
    )
    .await
}

#[tauri::command]
/// 批量查询一本书内已有学习计划的单元状态。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `product_code`：图书产品码。
pub async fn get_book_study_plan_statuses(
    state: State<'_, crate::database::DbState>,
    product_code: String,
) -> Result<Vec<crate::services::study_plan::BookStudyPlanStatusItem>, String> {
    let db = state.get().await?;
    crate::services::study_plan::get_book_study_plan_statuses(db.as_ref(), &product_code).await
}

#[tauri::command]
/// 查询日期范围内的学习任务汇总。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `range_start`：统计范围开始日期。
/// - `range_end`：统计范围结束日期。
/// - `view_mode`：学习任务汇总视图模式。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_study_tasks_summary(
    state: State<'_, crate::database::DbState>,
    range_start: String,
    range_end: String,
    view_mode: String,
    local_date: String,
) -> Result<crate::services::study_plan::StudyTaskSummaryResponse, String> {
    let db = state.get().await?;
    crate::services::study_plan::get_study_tasks_summary_on_date(
        db.as_ref(),
        &range_start,
        &range_end,
        &view_mode,
        &local_date,
    )
    .await
}

#[tauri::command]
/// 查询指定日期的待办和逾期学习任务。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `date`：需要查询的日期。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_tasks_by_date(
    state: State<'_, crate::database::DbState>,
    date: String,
    local_date: String,
) -> Result<Vec<crate::services::study_plan::StudyTaskItem>, String> {
    let db = state.get().await?;
    crate::services::study_plan::get_tasks_by_date_on_date(db.as_ref(), &date, &local_date).await
}

#[tauri::command]
/// 整体平移第 1 阶段尚未完成的学习计划。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `plan_unit_id`：学习计划数据库标识。
/// - `offset_days`：平移天数，负数表示提前，正数表示延期。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn shift_study_plan(
    state: State<'_, crate::database::DbState>,
    plan_unit_id: i64,
    offset_days: i32,
    local_date: String,
) -> Result<crate::services::study_plan::ShiftStudyPlanResponse, String> {
    let db = state.get().await?;
    crate::services::study_plan::shift_study_plan_on_date(
        db.as_ref(),
        plan_unit_id,
        offset_days,
        &local_date,
    )
    .await
}

#[tauri::command]
/// 完成学习任务并推进对应计划状态。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `task_id`：学习任务数据库标识。
/// - `local_date`：本次实际完成的本地日期。
/// - `rating`：第五次及以后任务的掌握评价。
/// - `finish_plan`：连续完成三次巩固后的结束选择。
/// - `expected_revision`：用户确认的预览版本，用于拒绝跨日或过期的安排。
pub async fn complete_study_task(
    state: State<'_, crate::database::DbState>,
    task_id: i64,
    local_date: String,
    rating: Option<String>,
    finish_plan: Option<bool>,
    expected_revision: Option<String>,
) -> Result<crate::services::study_plan::CompleteStudyTaskResponse, String> {
    let db = state.get().await?;
    crate::services::study_plan::complete_assessed_task_with_preview(
        db.as_ref(),
        task_id,
        &local_date,
        rating.as_deref(),
        finish_plan,
        expected_revision.as_deref(),
    )
    .await
}

#[tauri::command]
/// 获取四种掌握评价的安排预览；参数为共享状态、任务标识和本地日期。
pub async fn get_study_assessment_preview(
    state: State<'_, crate::database::DbState>,
    task_id: i64,
    local_date: String,
) -> Result<crate::services::study_plan::StudyAssessmentPreview, String> {
    let db = state.get().await?;
    crate::services::study_plan::get_assessment_preview(db.as_ref(), task_id, &local_date).await
}
