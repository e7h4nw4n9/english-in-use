//! 学习会话共用的行转换、校验和筛选构造。

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

/// 校验必填文本并返回去除首尾空白后的值。
///
/// # 参数
/// - `value`：待校验或转换的文本。
/// - `err_code`：字段为空时返回的稳定错误码。
pub(super) fn ensure_non_empty(value: &str, err_code: &'static str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(err_code.to_string());
    }
    Ok(())
}

/// 将统计周期限制为支持的日、周、月枚举值。
///
/// # 参数
/// - `period_type`：统计周期类型。
pub(super) fn normalize_period_type(period_type: &str) -> Result<&'static str, String> {
    match period_type {
        "week" => Ok("week"),
        "month" => Ok("month"),
        "year" => Ok("year"),
        _ => Err("INVALID_PERIOD_TYPE".to_string()),
    }
}

/// 返回统计周期对应的 SQLite 起止日期表达式。
///
/// # 参数
/// - `period_type`：统计周期类型。
/// - `local_date`：本地日期，格式为 YYYY-MM-DD。
pub(super) fn period_range_expr(
    period_type: &str,
    local_date: &str,
) -> Result<(String, String), String> {
    crate::services::study_plan::validate_local_date_for_payload(local_date)?;
    match period_type {
        "week" => Ok((
            format!("date('{local_date}', '-6 day')"),
            format!("date('{local_date}')"),
        )),
        "month" => Ok((
            format!("date('{local_date}', 'start of month')"),
            format!("date('{local_date}', 'start of month', '+1 month', '-1 day')"),
        )),
        "year" => Ok((
            format!("date('{local_date}', 'start of year')"),
            format!("date('{local_date}', 'start of year', '+1 year', '-1 day')"),
        )),
        _ => Err("INVALID_PERIOD_TYPE".to_string()),
    }
}

/// 将时间分组值转换为前端统计序列键。
///
/// # 参数
/// - `book_group`：会话关联图书的可选分组。
pub(super) fn series_key_from_group(book_group: i32) -> String {
    match book_group {
        1 => "vocabulary".to_string(),
        2 => "grammar".to_string(),
        _ => "other".to_string(),
    }
}

/// 将数据库 JSON 行转换为学习会话列表项。
///
/// # 参数
/// - `row`：数据库返回的 JSON 行。
pub(super) fn row_to_session(row: &Value) -> Option<StudySessionListItem> {
    let visited_units = row
        .get("visited_units_json")
        .and_then(|value| value.as_str())
        .and_then(|raw| serde_json::from_str::<Vec<StudySessionUnitRef>>(raw).ok())
        .unwrap_or_default();

    Some(StudySessionListItem {
        id: json_i64(row, "session_id")?,
        book_id: json_i64(row, "book_id")?,
        book_group: json_i32(row, "book_group")?,
        product_code: json_string(row, "product_code")?,
        book_title: json_string(row, "book_title")?,
        resource_id: json_string(row, "resource_id")?,
        unit_name: json_string(row, "unit_name")?,
        entry_resource_id: json_string(row, "entry_resource_id")?,
        entry_unit_name: json_string(row, "entry_unit_name")?,
        visited_units,
        start_at: json_string(row, "start_at")?,
        end_at: json_string(row, "end_at")?,
        duration: json_i64(row, "duration")?,
    })
}

/// 根据筛选条件构造复用的参数化 WHERE 子句。
///
/// # 参数
/// - `range_start_expr`：统计范围开始日期的 SQL 表达式。
/// - `range_end_expr`：统计范围结束日期的 SQL 表达式。
/// - `filters`：可选的统计筛选条件。
pub(super) fn build_common_where_clause(
    range_start_expr: &str,
    range_end_expr: &str,
    filters: Option<&StudyStatsFilters>,
) -> String {
    let mut clauses = vec![format!(
        "s.local_date BETWEEN {} AND {}",
        range_start_expr, range_end_expr
    )];

    if let Some(filter) = filters {
        if let Some(book_id) = filter.book_id {
            clauses.push(format!("s.book_id = {}", book_id));
        }
        if let Some(book_group) = filter.book_group {
            clauses.push(format!("b.book_group = {}", book_group));
        }
    }

    format!("WHERE {}", clauses.join(" AND "))
}
