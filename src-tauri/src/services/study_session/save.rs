//! 学习会话持久化流程。

use super::common::*;
use super::types::*;
use super::*;

/// 校验并持久化一次学习会话。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `payload`：需要保存的学习会话数据。
pub async fn save_study_session(
    db: &dyn Database,
    payload: SaveStudySessionPayload,
) -> Result<SaveStudySessionResponse, String> {
    ensure_non_empty(&payload.product_code, "BOOK_NOT_FOUND")?;
    ensure_non_empty(&payload.assigned_resource_id, "INVALID_RESOURCE_ID")?;
    ensure_non_empty(&payload.assigned_unit_name, "INVALID_UNIT_NAME")?;
    ensure_non_empty(&payload.entry_resource_id, "INVALID_ENTRY_RESOURCE_ID")?;
    ensure_non_empty(&payload.entry_unit_name, "INVALID_ENTRY_UNIT_NAME")?;
    ensure_non_empty(&payload.start_at, "INVALID_TIME_RANGE")?;
    ensure_non_empty(&payload.end_at, "INVALID_TIME_RANGE")?;

    if payload.duration <= 0 {
        return Err("INVALID_DURATION".to_string());
    }

    let start_at = time::OffsetDateTime::parse(
        &payload.start_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| "INVALID_TIME_RANGE".to_string())?;
    let end_at = time::OffsetDateTime::parse(
        &payload.end_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| "INVALID_TIME_RANGE".to_string())?;
    let elapsed_seconds = (end_at - start_at).whole_seconds();
    if elapsed_seconds < payload.duration || elapsed_seconds <= 0 {
        return Err("INVALID_TIME_RANGE".to_string());
    }
    if payload.timezone_offset_minutes < -840 || payload.timezone_offset_minutes > 840 {
        return Err("INVALID_TIMEZONE_OFFSET".to_string());
    }
    crate::services::study_plan::validate_local_date_for_payload(&payload.local_date)?;

    let visited_units_json =
        serde_json::to_string(&payload.visited_units).map_err(|e| e.to_string())?;
    let rows = db
        .query_write_statement(SqlStatement::new(
            "INSERT INTO study_sessions \
         (book_id, resource_id, unit_name, entry_resource_id, entry_unit_name, \
          visited_units_json, start_at, end_at, duration, local_date, \
          timezone_offset_minutes, created_at, updated_at) \
         SELECT id, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP \
         FROM books WHERE product_code = ? \
         RETURNING id",
            vec![
                SqlValue::Text(payload.assigned_resource_id),
                SqlValue::Text(payload.assigned_unit_name),
                SqlValue::Text(payload.entry_resource_id),
                SqlValue::Text(payload.entry_unit_name),
                SqlValue::Text(visited_units_json),
                SqlValue::Text(payload.start_at),
                SqlValue::Text(payload.end_at),
                SqlValue::Integer(payload.duration),
                SqlValue::Text(payload.local_date),
                SqlValue::Integer(i64::from(payload.timezone_offset_minutes)),
                SqlValue::Text(payload.product_code),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?;
    let session_id = rows
        .first()
        .and_then(|row| json_i64(row, "id"))
        .ok_or("DB_CONFLICT_RETRYABLE")?;

    Ok(SaveStudySessionResponse {
        id: session_id,
        success: true,
    })
}
