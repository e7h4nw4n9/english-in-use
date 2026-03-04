use crate::database::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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

fn escape_sql_literal(input: &str) -> String {
    input.replace('\'', "''")
}

fn value_to_i64(value: &Value) -> Option<i64> {
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

fn json_i64(row: &Value, key: &str) -> Option<i64> {
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

fn json_i32(row: &Value, key: &str) -> Option<i32> {
    json_i64(row, key).and_then(|v| i32::try_from(v).ok())
}

fn json_string(row: &Value, key: &str) -> Option<String> {
    row.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn ensure_non_empty(value: &str, err_code: &'static str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(err_code.to_string());
    }
    Ok(())
}

async fn resolve_book_id(db: &dyn Database, product_code: &str) -> Result<i64, String> {
    let product_code = escape_sql_literal(product_code);
    let sql = format!(
        "SELECT id FROM books WHERE product_code = '{}' LIMIT 1",
        product_code
    );
    let rows = db.query(sql).await.map_err(|e| e.to_string())?;
    let row = rows.first().ok_or("BOOK_NOT_FOUND")?;
    json_i64(row, "id").ok_or("BOOK_NOT_FOUND".to_string())
}

fn normalize_period_type(period_type: &str) -> Result<&'static str, String> {
    match period_type {
        "week" => Ok("week"),
        "month" => Ok("month"),
        "year" => Ok("year"),
        _ => Err("INVALID_PERIOD_TYPE".to_string()),
    }
}

fn period_range_expr(period_type: &str) -> Result<(&'static str, &'static str), String> {
    match period_type {
        "week" => Ok((
            "date('now', 'localtime', '-6 day')",
            "date('now', 'localtime')",
        )),
        "month" => Ok((
            "date('now', 'localtime', 'start of month')",
            "date('now', 'localtime', 'start of month', '+1 month', '-1 day')",
        )),
        "year" => Ok((
            "date('now', 'localtime', 'start of year')",
            "date('now', 'localtime', 'start of year', '+1 year', '-1 day')",
        )),
        _ => Err("INVALID_PERIOD_TYPE".to_string()),
    }
}

fn series_key_from_group(book_group: i32) -> String {
    match book_group {
        1 => "vocabulary".to_string(),
        2 => "grammar".to_string(),
        _ => "other".to_string(),
    }
}

fn build_common_where_clause(
    range_start_expr: &str,
    range_end_expr: &str,
    filters: Option<&StudyStatsFilters>,
) -> String {
    let mut clauses = vec![format!(
        "date(s.start_at, 'localtime') BETWEEN {} AND {}",
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

    if payload.end_at < payload.start_at {
        return Err("INVALID_TIME_RANGE".to_string());
    }

    let book_id = resolve_book_id(db, &payload.product_code).await?;

    let escaped_resource_id = escape_sql_literal(&payload.assigned_resource_id);
    let escaped_unit_name = escape_sql_literal(&payload.assigned_unit_name);
    let escaped_entry_resource_id = escape_sql_literal(&payload.entry_resource_id);
    let escaped_entry_unit_name = escape_sql_literal(&payload.entry_unit_name);
    let escaped_start_at = escape_sql_literal(&payload.start_at);
    let escaped_end_at = escape_sql_literal(&payload.end_at);
    let visited_units_json =
        serde_json::to_string(&payload.visited_units).map_err(|e| e.to_string())?;
    let escaped_visited_units_json = escape_sql_literal(&visited_units_json);

    let insert_sql = format!(
        "INSERT INTO study_sessions \
         (book_id, resource_id, unit_name, entry_resource_id, entry_unit_name, visited_units_json, start_at, end_at, duration, created_at, updated_at) \
         VALUES ({}, '{}', '{}', '{}', '{}', '{}', '{}', '{}', {}, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        book_id,
        escaped_resource_id,
        escaped_unit_name,
        escaped_entry_resource_id,
        escaped_entry_unit_name,
        escaped_visited_units_json,
        escaped_start_at,
        escaped_end_at,
        payload.duration
    );

    db.execute(insert_sql).await.map_err(|e| e.to_string())?;

    let fetch_id_sql = format!(
        "SELECT id FROM study_sessions WHERE book_id = {} AND start_at = '{}' AND end_at = '{}' ORDER BY id DESC LIMIT 1",
        book_id, escaped_start_at, escaped_end_at
    );
    let rows = db.query(fetch_id_sql).await.map_err(|e| e.to_string())?;
    let session_id = rows
        .first()
        .and_then(|row| json_i64(row, "id"))
        .ok_or("DB_CONFLICT_RETRYABLE")?;

    Ok(SaveStudySessionResponse {
        id: session_id,
        success: true,
    })
}

pub async fn get_study_stats(
    db: &dyn Database,
    period_type: &str,
    filters: Option<StudyStatsFilters>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<StudyStatsResponse, String> {
    let period_type = normalize_period_type(period_type)?;
    let (range_start_expr, range_end_expr) = period_range_expr(period_type)?;
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * page_size;

    let where_sql = build_common_where_clause(range_start_expr, range_end_expr, filters.as_ref());

    let range_sql = format!(
        "SELECT {} AS range_start, {} AS range_end",
        range_start_expr, range_end_expr
    );
    let range_rows = db.query(range_sql).await.map_err(|e| e.to_string())?;
    let range_start = range_rows
        .first()
        .and_then(|row| json_string(row, "range_start"))
        .unwrap_or_default();
    let range_end = range_rows
        .first()
        .and_then(|row| json_string(row, "range_end"))
        .unwrap_or_default();

    let all_sessions_sql = format!(
        "SELECT s.id AS session_id, s.book_id AS book_id, b.book_group AS book_group, b.product_code AS product_code, b.title AS book_title, \
                s.resource_id AS resource_id, s.unit_name AS unit_name, s.entry_resource_id AS entry_resource_id, s.entry_unit_name AS entry_unit_name, \
                s.visited_units_json AS visited_units_json, s.start_at AS start_at, s.end_at AS end_at, s.duration AS duration, \
                date(s.start_at, 'localtime') AS stat_date \
         FROM study_sessions s \
         JOIN books b ON s.book_id = b.id \
         {} \
         ORDER BY s.start_at DESC, s.id DESC",
        where_sql
    );
    let all_rows = db
        .query(all_sessions_sql)
        .await
        .map_err(|e| e.to_string())?;

    let mut trend_map: BTreeMap<String, i64> = BTreeMap::new();
    let mut book_map: BTreeMap<i64, StudyStatsBookBreakdownItem> = BTreeMap::new();
    let mut series_map: BTreeMap<i32, i64> = BTreeMap::new();

    let all_sessions = all_rows
        .iter()
        .filter_map(|row| {
            let visited_units = row
                .get("visited_units_json")
                .and_then(|v| v.as_str())
                .and_then(|raw| serde_json::from_str::<Vec<StudySessionUnitRef>>(raw).ok())
                .unwrap_or_default();

            let session_id = json_i64(row, "session_id")?;
            let book_id = json_i64(row, "book_id")?;
            let book_group = json_i32(row, "book_group")?;
            let product_code = json_string(row, "product_code")?;
            let book_title = json_string(row, "book_title")?;
            let resource_id = json_string(row, "resource_id")?;
            let unit_name = json_string(row, "unit_name")?;
            let entry_resource_id = json_string(row, "entry_resource_id")?;
            let entry_unit_name = json_string(row, "entry_unit_name")?;
            let start_at = json_string(row, "start_at")?;
            let end_at = json_string(row, "end_at")?;
            let duration = json_i64(row, "duration")?;
            let stat_date = json_string(row, "stat_date")?;

            trend_map
                .entry(stat_date)
                .and_modify(|sum| *sum += duration)
                .or_insert(duration);

            book_map
                .entry(book_id)
                .and_modify(|item| item.duration += duration)
                .or_insert(StudyStatsBookBreakdownItem {
                    book_id,
                    product_code: product_code.clone(),
                    book_title: book_title.clone(),
                    duration,
                });

            series_map
                .entry(book_group)
                .and_modify(|sum| *sum += duration)
                .or_insert(duration);

            Some(StudySessionListItem {
                id: session_id,
                book_id,
                book_group,
                product_code,
                book_title,
                resource_id,
                unit_name,
                entry_resource_id,
                entry_unit_name,
                visited_units,
                start_at,
                end_at,
                duration,
            })
        })
        .collect::<Vec<_>>();

    let trend = trend_map
        .into_iter()
        .map(|(date, duration)| StudyStatsTrendItem { date, duration })
        .collect::<Vec<_>>();

    let mut book_breakdown = book_map.into_values().collect::<Vec<_>>();
    book_breakdown.sort_by(|a, b| {
        b.duration
            .cmp(&a.duration)
            .then_with(|| a.book_title.cmp(&b.book_title))
    });

    let mut series_breakdown = series_map
        .into_iter()
        .map(|(book_group, duration)| StudyStatsSeriesBreakdownItem {
            book_group,
            series_key: series_key_from_group(book_group),
            duration,
        })
        .collect::<Vec<_>>();
    series_breakdown.sort_by(|a, b| {
        b.duration
            .cmp(&a.duration)
            .then_with(|| a.book_group.cmp(&b.book_group))
    });

    let total_recent = i64::try_from(all_sessions.len()).unwrap_or(0);
    let page_start = usize::try_from(offset).unwrap_or(usize::MAX);
    let page_limit = usize::try_from(page_size).unwrap_or(0);
    let recent_sessions = if page_start >= all_sessions.len() {
        Vec::new()
    } else {
        let page_end = page_start
            .saturating_add(page_limit)
            .min(all_sessions.len());
        all_sessions[page_start..page_end].to_vec()
    };

    Ok(StudyStatsResponse {
        period_type: period_type.to_string(),
        range_start,
        range_end,
        trend,
        book_breakdown,
        series_breakdown,
        recent_sessions,
        page,
        page_size,
        total_recent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::MIGRATIONS;
    use crate::database::{Database, SqliteDatabase};

    async fn setup_test_db() -> SqliteDatabase {
        let db_path =
            std::env::temp_dir().join(format!("study_session_test_{}.db", uuid::Uuid::new_v4()));
        let db = SqliteDatabase::new(db_path.to_str().unwrap())
            .await
            .unwrap();

        for migration in MIGRATIONS {
            db.execute(migration.up.to_string()).await.unwrap();
            db.set_version(migration.version).await.unwrap();
        }

        db.execute("INSERT INTO books (id, book_group, product_code, title, author, product_type, sort_num) VALUES (3001, 1, 'timerbooka', 'Timer Book A', NULL, 'imgbook', 1)".to_string()).await.unwrap();
        db.execute("INSERT INTO books (id, book_group, product_code, title, author, product_type, sort_num) VALUES (3002, 2, 'timerbookb', 'Timer Book B', NULL, 'imgbook', 2)".to_string()).await.unwrap();

        db
    }

    async fn current_db_datetime(db: &dyn Database) -> String {
        let rows = db
            .query("SELECT datetime('now') AS now_at".to_string())
            .await
            .unwrap();
        rows.first()
            .and_then(|row| row.get("now_at"))
            .and_then(|value| value.as_str())
            .unwrap_or("2026-03-01 10:00:00")
            .to_string()
    }

    #[tokio::test]
    async fn saves_session_and_returns_week_stats() {
        let db = setup_test_db().await;
        let now_at = current_db_datetime(&db).await;

        let payload = SaveStudySessionPayload {
            product_code: "timerbooka".to_string(),
            entry_resource_id: "RE_U1".to_string(),
            entry_unit_name: "Unit 1".to_string(),
            assigned_resource_id: "RE_U1".to_string(),
            assigned_unit_name: "Unit 1".to_string(),
            visited_units: vec![StudySessionUnitRef {
                resource_id: "RE_U1".to_string(),
                unit_name: "Unit 1".to_string(),
            }],
            start_at: now_at.clone(),
            end_at: now_at.clone(),
            duration: 1200,
        };

        let save_result = save_study_session(&db, payload).await.unwrap();
        assert!(save_result.success);
        assert!(save_result.id > 0);

        let stats = get_study_stats(&db, "week", None, Some(1), Some(20))
            .await
            .unwrap();

        assert_eq!(stats.period_type, "week");
        assert!(!stats.book_breakdown.is_empty());
        assert!(!stats.recent_sessions.is_empty());
        assert_eq!(stats.recent_sessions[0].resource_id, "RE_U1");
        assert_eq!(stats.recent_sessions[0].duration, 1200);
    }

    #[tokio::test]
    async fn filters_stats_by_book_group() {
        let db = setup_test_db().await;
        let now_at = current_db_datetime(&db).await;

        let payload_a = SaveStudySessionPayload {
            product_code: "timerbooka".to_string(),
            entry_resource_id: "RE_A".to_string(),
            entry_unit_name: "Unit A".to_string(),
            assigned_resource_id: "RE_A".to_string(),
            assigned_unit_name: "Unit A".to_string(),
            visited_units: vec![],
            start_at: now_at.clone(),
            end_at: now_at.clone(),
            duration: 600,
        };

        let payload_b = SaveStudySessionPayload {
            product_code: "timerbookb".to_string(),
            entry_resource_id: "RE_B".to_string(),
            entry_unit_name: "Unit B".to_string(),
            assigned_resource_id: "RE_B".to_string(),
            assigned_unit_name: "Unit B".to_string(),
            visited_units: vec![],
            start_at: now_at.clone(),
            end_at: now_at.clone(),
            duration: 1200,
        };

        save_study_session(&db, payload_a).await.unwrap();
        save_study_session(&db, payload_b).await.unwrap();

        let filtered = get_study_stats(
            &db,
            "week",
            Some(StudyStatsFilters {
                book_id: None,
                book_group: Some(2),
            }),
            Some(1),
            Some(20),
        )
        .await
        .unwrap();

        assert_eq!(filtered.book_breakdown.len(), 1);
        assert_eq!(filtered.book_breakdown[0].product_code, "timerbookb");
        assert_eq!(filtered.series_breakdown.len(), 1);
        assert_eq!(filtered.series_breakdown[0].book_group, 2);
    }
}
