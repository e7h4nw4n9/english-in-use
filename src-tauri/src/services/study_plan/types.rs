//! 学习计划服务的响应类型和复习常量。

use serde::Serialize;

pub(super) const REVIEW_DAY_OFFSETS: [i32; 7] = [1, 2, 4, 7, 15, 30, 60];
pub(super) const TOTAL_STAGES: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StudyPlanUpsertOutcome {
    Created,
    Reactivated,
    Restarted,
    AlreadyActive,
    AlreadyMastered,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPlanUpsertResponse {
    pub plan_unit_id: i64,
    pub plan_status: i32,
    pub next_review_date: Option<String>,
    pub total_stages: i32,
    pub completed_stages: Vec<i32>,
    pub outcome: StudyPlanUpsertOutcome,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPlanStatusResponse {
    pub in_plan: bool,
    pub plan_status: Option<i32>,
    pub plan_unit_id: Option<i64>,
    pub completed_stages: Vec<i32>,
    pub next_review_date: Option<String>,
    pub overdue_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPlanActionResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTaskSummaryDay {
    pub date: String,
    pub total: i64,
    pub due: i64,
    pub overdue: i64,
    pub completed: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTaskSummaryResponse {
    pub range_start: String,
    pub range_end: String,
    pub view_mode: String,
    pub days: Vec<StudyTaskSummaryDay>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTaskItem {
    pub task_id: i64,
    pub plan_unit_id: i64,
    pub product_code: String,
    pub resource_id: String,
    pub unit_name: String,
    pub review_stage: i32,
    pub scheduled_date: String,
    pub is_overdue: bool,
    pub task_status: i32,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteStudyTaskResponse {
    pub task_id: i64,
    pub task_status: i32,
    pub plan_status: i32,
    pub completed_stages: Vec<i32>,
}
