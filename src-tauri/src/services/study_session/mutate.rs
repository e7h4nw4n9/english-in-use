//! 学习会话修改与删除流程。

use super::common::*;
use super::types::*;
use super::*;
use time::format_description::well_known::Rfc3339;

/// 修改学习会话的有效时长和归属单元。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `payload`：会话标识、新时长和目标归属单元。
pub async fn update_study_session(
    db: &dyn Database,
    payload: UpdateStudySessionPayload,
) -> Result<StudySessionActionResponse, String> {
    if payload.duration <= 0 {
        return Err("INVALID_DURATION".to_string());
    }
    ensure_non_empty(&payload.assigned_resource_id, "INVALID_RESOURCE_ID")?;

    let rows = db
        .query_statement(SqlStatement::new(
            "SELECT id AS session_id, resource_id, unit_name, visited_units_json, \
                    start_at, timezone_offset_minutes \
             FROM study_sessions WHERE id = ? LIMIT 1",
            vec![SqlValue::Integer(payload.session_id)],
        ))
        .await
        .map_err(|e| e.to_string())?;
    let row = rows.first().ok_or("SESSION_NOT_FOUND")?;

    let current_resource_id = json_string(row, "resource_id").ok_or("SESSION_NOT_FOUND")?;
    let current_unit_name = json_string(row, "unit_name").ok_or("SESSION_NOT_FOUND")?;
    let mut unit_options = row
        .get("visited_units_json")
        .and_then(Value::as_str)
        .and_then(|raw| serde_json::from_str::<Vec<StudySessionUnitRef>>(raw).ok())
        .unwrap_or_default();
    if !unit_options
        .iter()
        .any(|unit| unit.resource_id == current_resource_id)
    {
        unit_options.push(StudySessionUnitRef {
            resource_id: current_resource_id,
            unit_name: current_unit_name,
        });
    }
    let assigned_unit = unit_options
        .iter()
        .find(|unit| unit.resource_id == payload.assigned_resource_id)
        .ok_or("INVALID_ASSIGNED_UNIT")?;

    let start_at_raw = json_string(row, "start_at").ok_or("INVALID_TIME_RANGE")?;
    let start_at = time::OffsetDateTime::parse(&start_at_raw, &Rfc3339)
        .map_err(|_| "INVALID_TIME_RANGE".to_string())?;
    let end_at = start_at
        .checked_add(time::Duration::seconds(payload.duration))
        .ok_or("INVALID_DURATION")?;
    let end_at_raw = end_at
        .format(&Rfc3339)
        .map_err(|_| "INVALID_TIME_RANGE".to_string())?;

    let timezone_offset_minutes = json_i32(row, "timezone_offset_minutes").unwrap_or(0);
    let timezone_offset = time::UtcOffset::from_whole_seconds(
        timezone_offset_minutes
            .checked_mul(60)
            .ok_or("INVALID_TIMEZONE_OFFSET")?,
    )
    .map_err(|_| "INVALID_TIMEZONE_OFFSET".to_string())?;
    let local_date = end_at.to_offset(timezone_offset).date().to_string();

    let updated_rows = db
        .query_write_statement(SqlStatement::new(
            "UPDATE study_sessions SET resource_id = ?, unit_name = ?, duration = ?, \
                    end_at = ?, local_date = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? RETURNING id AS session_id",
            vec![
                SqlValue::Text(assigned_unit.resource_id.clone()),
                SqlValue::Text(assigned_unit.unit_name.clone()),
                SqlValue::Integer(payload.duration),
                SqlValue::Text(end_at_raw),
                SqlValue::Text(local_date),
                SqlValue::Integer(payload.session_id),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?;
    if updated_rows.is_empty() {
        return Err("SESSION_NOT_FOUND".to_string());
    }

    Ok(StudySessionActionResponse {
        success: true,
        session_id: payload.session_id,
    })
}

/// 永久删除一条学习会话。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `session_id`：学习会话数据库标识。
pub async fn delete_study_session(
    db: &dyn Database,
    session_id: i64,
) -> Result<StudySessionActionResponse, String> {
    let rows = db
        .query_write_statement(SqlStatement::new(
            "DELETE FROM study_sessions WHERE id = ? RETURNING id AS session_id",
            vec![SqlValue::Integer(session_id)],
        ))
        .await
        .map_err(|e| e.to_string())?;
    if rows.is_empty() {
        return Err("SESSION_NOT_FOUND".to_string());
    }

    Ok(StudySessionActionResponse {
        success: true,
        session_id,
    })
}
