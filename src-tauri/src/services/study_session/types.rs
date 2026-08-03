//! 学习会话保存与统计接口的数据类型。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySessionUnitRef {
    pub resource_id: String,
    pub unit_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStudySessionPayload {
    pub product_code: String,
    pub entry_resource_id: String,
    pub entry_unit_name: String,
    pub assigned_resource_id: String,
    pub assigned_unit_name: String,
    #[serde(default)]
    pub visited_units: Vec<StudySessionUnitRef>,
    pub start_at: String,
    pub end_at: String,
    pub duration: i64,
    pub local_date: String,
    pub timezone_offset_minutes: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStudySessionResponse {
    pub id: i64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StudyStatsFilters {
    pub book_id: Option<i64>,
    pub book_group: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyStatsTrendItem {
    pub date: String,
    pub duration: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyStatsBookBreakdownItem {
    pub book_id: i64,
    pub product_code: String,
    pub book_title: String,
    pub duration: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyStatsSeriesBreakdownItem {
    pub book_group: i32,
    pub series_key: String,
    pub duration: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySessionListItem {
    pub id: i64,
    pub book_id: i64,
    pub book_group: i32,
    pub product_code: String,
    pub book_title: String,
    pub resource_id: String,
    pub unit_name: String,
    pub entry_resource_id: String,
    pub entry_unit_name: String,
    pub visited_units: Vec<StudySessionUnitRef>,
    pub start_at: String,
    pub end_at: String,
    pub duration: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyStatsResponse {
    pub period_type: String,
    pub range_start: String,
    pub range_end: String,
    pub trend: Vec<StudyStatsTrendItem>,
    pub book_breakdown: Vec<StudyStatsBookBreakdownItem>,
    pub series_breakdown: Vec<StudyStatsSeriesBreakdownItem>,
    pub recent_sessions: Vec<StudySessionListItem>,
    pub page: i64,
    pub page_size: i64,
    pub total_recent: i64,
}
