//! 学习计划共用的数据库行转换、日期校验和查询辅助函数。

use super::*;

pub(super) fn value_to_i64(value: &Value) -> Option<i64> {
    if let Some(v) = value.as_i64() {
        return Some(v);
    }
    if let Some(v) = value.as_u64() {
        return i64::try_from(v).ok();
    }
    if let Some(v) = value.as_str() {
        return v.parse::<i64>().ok();
    }
    if let Some(v) = value.as_bool() {
        return Some(if v { 1 } else { 0 });
    }
    None
}

pub(super) fn json_i64(row: &Value, key: &str) -> Option<i64> {
    if let Some(value) = row.get(key) {
        return value_to_i64(value);
    }

    // SQLite 表达式列名在某些场景下会退回为原始表达式（如 COUNT(*)）。
    row.as_object()
        .and_then(|obj| {
            if obj.len() == 1 {
                obj.values().next()
            } else {
                None
            }
        })
        .and_then(value_to_i64)
}

pub(super) fn json_i32(row: &Value, key: &str) -> Option<i32> {
    json_i64(row, key).and_then(|v| i32::try_from(v).ok())
}

pub(super) fn json_string(row: &Value, key: &str) -> Option<String> {
    row.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

/// 解析 SQL JSON 聚合返回的已完成阶段列表。
pub(super) fn json_completed_stages(row: &Value) -> Vec<i32> {
    row.get("completed_stages_json")
        .and_then(Value::as_str)
        .and_then(|value| serde_json::from_str::<Vec<i32>>(value).ok())
        .unwrap_or_default()
}

/// 解析计划已有的全部复习阶段列表。
pub(super) fn json_all_stages(row: &Value) -> Vec<i32> {
    row.get("all_stages_json")
        .and_then(Value::as_str)
        .and_then(|value| serde_json::from_str::<Vec<i32>>(value).ok())
        .unwrap_or_default()
}

/// 使用一次聚合查询读取图书、计划及其任务状态。
pub(super) async fn query_plan_status_row(
    db: &dyn Database,
    product_code: &str,
    resource_id: &str,
    local_date: &str,
) -> Result<Option<Value>, String> {
    let rows = db
        .query_statement(SqlStatement::new(
            "SELECT b.id AS book_id, u.id AS plan_unit_id, u.plan_status AS plan_status, \
                    COALESCE((SELECT json_group_array(review_stage) FROM (\
                        SELECT review_stage FROM study_tasks \
                        WHERE plan_unit_id = u.id AND task_status = 1 ORDER BY review_stage\
                    )), '[]') AS completed_stages_json, \
                    COALESCE((SELECT json_group_array(review_stage) FROM (\
                        SELECT review_stage FROM study_tasks \
                        WHERE plan_unit_id = u.id ORDER BY review_stage\
                    )), '[]') AS all_stages_json, \
                    (SELECT scheduled_date FROM study_tasks \
                     WHERE plan_unit_id = u.id AND task_status = 0 \
                     ORDER BY scheduled_date, review_stage LIMIT 1) AS next_review_date, \
                    (SELECT COUNT(*) FROM study_tasks \
                     WHERE plan_unit_id = u.id AND task_status = 0 AND scheduled_date < ?) AS overdue_count \
             FROM books b \
             LEFT JOIN study_plan_units u ON u.book_id = b.id AND u.resource_id = ? \
             WHERE b.product_code = ? LIMIT 1",
            vec![
                SqlValue::Text(local_date.to_string()),
                SqlValue::Text(resource_id.to_string()),
                SqlValue::Text(product_code.to_string()),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().next())
}

/// 读取计划单元已经完成的复习阶段。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `plan_unit_id`：计划单元数据库标识。
pub(super) async fn get_completed_stages(
    db: &dyn Database,
    plan_unit_id: i64,
) -> Result<Vec<i32>, String> {
    let sql = format!(
        "SELECT review_stage FROM study_tasks WHERE plan_unit_id = {} AND task_status = 1 ORDER BY review_stage ASC",
        plan_unit_id
    );
    let rows = db.query(sql).await.map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .filter_map(|row| json_i32(row, "review_stage"))
        .collect())
}

/// 校验 YYYY-MM-DD 文本和实际日历日期。
///
/// # 参数
/// - `local_date`：本地日期，格式为 YYYY-MM-DD。
pub(super) fn validate_local_date(local_date: &str) -> Result<(), String> {
    let bytes = local_date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7) && !byte.is_ascii_digit())
    {
        return Err("INVALID_LOCAL_DATE".to_string());
    }

    let year = local_date[0..4]
        .parse::<i32>()
        .map_err(|_| "INVALID_LOCAL_DATE".to_string())?;
    let month = local_date[5..7]
        .parse::<u8>()
        .ok()
        .and_then(|value| time::Month::try_from(value).ok())
        .ok_or_else(|| "INVALID_LOCAL_DATE".to_string())?;
    let day = local_date[8..10]
        .parse::<u8>()
        .map_err(|_| "INVALID_LOCAL_DATE".to_string())?;
    time::Date::from_calendar_date(year, month, day)
        .map_err(|_| "INVALID_LOCAL_DATE".to_string())?;

    Ok(())
}

/// 校验前端传入的本地日期格式和日历有效性。
pub(crate) fn validate_local_date_for_payload(local_date: &str) -> Result<(), String> {
    validate_local_date(local_date)
}
