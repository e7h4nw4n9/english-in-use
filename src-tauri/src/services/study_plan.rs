use crate::database::Database;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

const REVIEW_DAY_OFFSETS: [i32; 7] = [1, 2, 4, 7, 15, 30, 60];
const TOTAL_STAGES: i32 = 7;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPlanUpsertResponse {
    pub plan_unit_id: i64,
    pub plan_status: i32,
    pub next_review_date: Option<String>,
    pub total_stages: i32,
    pub completed_stages: Vec<i32>,
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

fn json_i32(row: &Value, key: &str) -> Option<i32> {
    json_i64(row, key).and_then(|v| i32::try_from(v).ok())
}

fn json_string(row: &Value, key: &str) -> Option<String> {
    row.get(key).and_then(|v| v.as_str()).map(str::to_string)
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

async fn get_plan_row(
    db: &dyn Database,
    book_id: i64,
    resource_id: &str,
) -> Result<Option<Value>, String> {
    let resource_id = escape_sql_literal(resource_id);
    let sql = format!(
        "SELECT id, plan_status FROM study_plan_units WHERE book_id = {} AND resource_id = '{}' LIMIT 1",
        book_id, resource_id
    );
    let rows = db.query(sql).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().next())
}

async fn get_completed_stages(db: &dyn Database, plan_unit_id: i64) -> Result<Vec<i32>, String> {
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

async fn get_next_review_date(
    db: &dyn Database,
    plan_unit_id: i64,
) -> Result<Option<String>, String> {
    let sql = format!(
        "SELECT scheduled_date FROM study_tasks WHERE plan_unit_id = {} AND task_status = 0 ORDER BY scheduled_date ASC, review_stage ASC LIMIT 1",
        plan_unit_id
    );
    let rows = db.query(sql).await.map_err(|e| e.to_string())?;
    Ok(rows
        .first()
        .and_then(|row| json_string(row, "scheduled_date")))
}

async fn get_overdue_count(db: &dyn Database, plan_unit_id: i64) -> Result<i64, String> {
    let sql = format!(
        "SELECT id FROM study_tasks WHERE plan_unit_id = {} AND task_status = 0 AND scheduled_date < date('now', 'localtime')",
        plan_unit_id
    );
    let rows = db.query(sql).await.map_err(|e| e.to_string())?;
    Ok(rows.len() as i64)
}

pub async fn upsert_study_plan(
    db: &dyn Database,
    product_code: &str,
    resource_id: &str,
    unit_name: &str,
) -> Result<StudyPlanUpsertResponse, String> {
    let book_id = resolve_book_id(db, product_code).await?;
    let mut plan_row = get_plan_row(db, book_id, resource_id).await?;

    if plan_row.is_none() {
        let escaped_resource_id = escape_sql_literal(resource_id);
        let escaped_unit_name = escape_sql_literal(unit_name);
        let insert_plan_sql = format!(
            "INSERT INTO study_plan_units (book_id, resource_id, unit_name, plan_status, created_at, updated_at) VALUES ({}, '{}', '{}', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            book_id, escaped_resource_id, escaped_unit_name
        );
        if let Err(insert_err) = db.execute(insert_plan_sql).await {
            let existing_after_conflict = get_plan_row(db, book_id, resource_id).await?;
            if existing_after_conflict.is_none() {
                return Err(insert_err.to_string());
            }
        }

        plan_row = get_plan_row(db, book_id, resource_id).await?;
        let plan_unit_id = plan_row
            .as_ref()
            .and_then(|row| json_i64(row, "id"))
            .ok_or("DB_CONFLICT_RETRYABLE")?;

        for (index, offset) in REVIEW_DAY_OFFSETS.iter().enumerate() {
            let stage = index + 1;
            let insert_task_sql = format!(
                "INSERT OR IGNORE INTO study_tasks (plan_unit_id, scheduled_date, review_stage, task_status, created_at, updated_at) VALUES ({}, date('now', 'localtime', '+{} day'), {}, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                plan_unit_id, offset, stage
            );
            db.execute(insert_task_sql)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    let plan_row = plan_row.ok_or("PLAN_NOT_FOUND")?;
    let plan_unit_id = json_i64(&plan_row, "id").ok_or("PLAN_NOT_FOUND")?;
    let mut plan_status = json_i32(&plan_row, "plan_status").ok_or("PLAN_NOT_FOUND")?;

    if plan_status == 2 {
        let sql = format!(
            "UPDATE study_plan_units SET plan_status = 0, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
            plan_unit_id
        );
        db.execute(sql).await.map_err(|e| e.to_string())?;
        plan_status = 0;
    }

    let completed_stages = get_completed_stages(db, plan_unit_id).await?;
    let next_review_date = get_next_review_date(db, plan_unit_id).await?;

    Ok(StudyPlanUpsertResponse {
        plan_unit_id,
        plan_status,
        next_review_date,
        total_stages: TOTAL_STAGES,
        completed_stages,
    })
}

pub async fn abandon_study_plan(
    db: &dyn Database,
    product_code: &str,
    resource_id: &str,
) -> Result<StudyPlanActionResponse, String> {
    let book_id = resolve_book_id(db, product_code).await?;
    let plan_row = get_plan_row(db, book_id, resource_id)
        .await?
        .ok_or("PLAN_NOT_FOUND")?;

    let plan_unit_id = json_i64(&plan_row, "id").ok_or("PLAN_NOT_FOUND")?;
    let sql = format!(
        "UPDATE study_plan_units SET plan_status = 2, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
        plan_unit_id
    );
    db.execute(sql).await.map_err(|e| e.to_string())?;

    Ok(StudyPlanActionResponse { success: true })
}

pub async fn get_study_plan_status(
    db: &dyn Database,
    product_code: &str,
    resource_id: &str,
) -> Result<StudyPlanStatusResponse, String> {
    let book_id = resolve_book_id(db, product_code).await?;
    let plan_row = get_plan_row(db, book_id, resource_id).await?;

    let Some(plan_row) = plan_row else {
        return Ok(StudyPlanStatusResponse {
            in_plan: false,
            plan_status: None,
            plan_unit_id: None,
            completed_stages: Vec::new(),
            next_review_date: None,
            overdue_count: 0,
        });
    };

    let plan_unit_id = json_i64(&plan_row, "id").ok_or("PLAN_NOT_FOUND")?;
    let plan_status = json_i32(&plan_row, "plan_status").ok_or("PLAN_NOT_FOUND")?;

    let completed_stages = get_completed_stages(db, plan_unit_id).await?;
    let next_review_date = get_next_review_date(db, plan_unit_id).await?;
    let overdue_count = get_overdue_count(db, plan_unit_id).await?;

    Ok(StudyPlanStatusResponse {
        in_plan: true,
        plan_status: Some(plan_status),
        plan_unit_id: Some(plan_unit_id),
        completed_stages,
        next_review_date,
        overdue_count,
    })
}

pub async fn get_study_tasks_summary(
    db: &dyn Database,
    range_start: &str,
    range_end: &str,
    view_mode: &str,
) -> Result<StudyTaskSummaryResponse, String> {
    if !matches!(view_mode, "day" | "week" | "month") {
        return Err("INVALID_VIEW_MODE".to_string());
    }

    let range_start_raw = range_start.to_string();
    let range_end_raw = range_end.to_string();
    let range_start_sql = escape_sql_literal(range_start);
    let range_end_sql = escape_sql_literal(range_end);

    let sql = format!(
        "SELECT \
            t.scheduled_date AS scheduled_date, \
            t.task_status AS task_status \
        FROM study_tasks t \
        JOIN study_plan_units u ON t.plan_unit_id = u.id \
        WHERE u.plan_status = 0 \
          AND t.scheduled_date >= '{}' \
          AND t.scheduled_date <= '{}' \
        ORDER BY t.scheduled_date ASC",
        range_start_sql, range_end_sql
    );

    let rows = db.query(sql).await.map_err(|e| e.to_string())?;
    let today_rows = db
        .query("SELECT date('now', 'localtime') AS today".to_string())
        .await
        .map_err(|e| e.to_string())?;
    let today = today_rows
        .first()
        .and_then(|row| json_string(row, "today"))
        .unwrap_or_default();

    let mut day_map: BTreeMap<String, StudyTaskSummaryDay> = BTreeMap::new();
    for row in rows {
        let date = json_string(&row, "scheduled_date").unwrap_or_default();
        if date.is_empty() {
            continue;
        }
        let task_status = json_i32(&row, "task_status").unwrap_or(0);

        let entry = day_map.entry(date.clone()).or_insert(StudyTaskSummaryDay {
            date,
            total: 0,
            due: 0,
            overdue: 0,
            completed: 0,
        });
        entry.total += 1;
        if task_status == 0 {
            entry.due += 1;
            if !today.is_empty() && entry.date < today {
                entry.overdue += 1;
            }
        } else {
            entry.completed += 1;
        }
    }

    let days = day_map.into_values().collect();

    Ok(StudyTaskSummaryResponse {
        range_start: range_start_raw,
        range_end: range_end_raw,
        view_mode: view_mode.to_string(),
        days,
    })
}

pub async fn get_tasks_by_date(
    db: &dyn Database,
    date: &str,
) -> Result<Vec<StudyTaskItem>, String> {
    let date = escape_sql_literal(date);

    let sql = format!(
        "SELECT \
            t.id AS task_id, \
            t.plan_unit_id AS plan_unit_id, \
            b.product_code AS product_code, \
            u.resource_id AS resource_id, \
            u.unit_name AS unit_name, \
            t.review_stage AS review_stage, \
            t.scheduled_date AS scheduled_date, \
            t.task_status AS task_status, \
            t.completed_at AS completed_at \
        FROM study_tasks t \
        JOIN study_plan_units u ON t.plan_unit_id = u.id \
        JOIN books b ON u.book_id = b.id \
        WHERE u.plan_status = 0 \
          AND t.scheduled_date = '{}' \
        ORDER BY t.review_stage ASC, t.id ASC",
        date
    );

    let rows = db.query(sql).await.map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|row| StudyTaskItem {
            task_id: json_i64(row, "task_id").unwrap_or_default(),
            plan_unit_id: json_i64(row, "plan_unit_id").unwrap_or_default(),
            product_code: json_string(row, "product_code").unwrap_or_default(),
            resource_id: json_string(row, "resource_id").unwrap_or_default(),
            unit_name: json_string(row, "unit_name").unwrap_or_default(),
            review_stage: json_i32(row, "review_stage").unwrap_or_default(),
            scheduled_date: json_string(row, "scheduled_date").unwrap_or_default(),
            task_status: json_i32(row, "task_status").unwrap_or_default(),
            is_overdue: false,
            completed_at: json_string(row, "completed_at"),
        })
        .collect())
}

pub async fn complete_study_task(
    db: &dyn Database,
    task_id: i64,
) -> Result<CompleteStudyTaskResponse, String> {
    let task_sql = format!(
        "SELECT t.id AS task_id, t.plan_unit_id AS plan_unit_id, t.task_status AS task_status, u.plan_status AS plan_status \
         FROM study_tasks t \
         JOIN study_plan_units u ON t.plan_unit_id = u.id \
         WHERE t.id = {} LIMIT 1",
        task_id
    );

    let task_rows = db.query(task_sql).await.map_err(|e| e.to_string())?;
    let task_row = task_rows.first().ok_or("TASK_NOT_FOUND")?;

    let plan_unit_id = json_i64(task_row, "plan_unit_id").ok_or("TASK_NOT_FOUND")?;
    let mut task_status = json_i32(task_row, "task_status").ok_or("TASK_NOT_FOUND")?;
    let plan_status_before = json_i32(task_row, "plan_status").ok_or("TASK_NOT_FOUND")?;

    if plan_status_before != 0 {
        return Err(if plan_status_before == 1 {
            "PLAN_ALREADY_MASTERED".to_string()
        } else {
            "PLAN_NOT_ACTIVE".to_string()
        });
    }

    if task_status == 0 {
        let update_task_sql = format!(
            "UPDATE study_tasks SET task_status = 1, completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
            task_id
        );
        db.execute(update_task_sql)
            .await
            .map_err(|e| e.to_string())?;

        let update_plan_sql = format!(
            "UPDATE study_plan_units SET last_review_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
            plan_unit_id
        );
        db.execute(update_plan_sql)
            .await
            .map_err(|e| e.to_string())?;

        task_status = 1;
    }

    let completed_count_sql = format!(
        "SELECT id FROM study_tasks WHERE plan_unit_id = {} AND task_status = 1",
        plan_unit_id
    );
    let completed_count_rows = db
        .query(completed_count_sql)
        .await
        .map_err(|e| e.to_string())?;
    let completed_count = completed_count_rows.len() as i64;

    if completed_count >= i64::from(TOTAL_STAGES) {
        let update_mastered_sql = format!(
            "UPDATE study_plan_units SET plan_status = 1, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
            plan_unit_id
        );
        db.execute(update_mastered_sql)
            .await
            .map_err(|e| e.to_string())?;
    }

    let plan_status_sql = format!(
        "SELECT plan_status FROM study_plan_units WHERE id = {} LIMIT 1",
        plan_unit_id
    );
    let plan_status_rows = db.query(plan_status_sql).await.map_err(|e| e.to_string())?;
    let plan_status = plan_status_rows
        .first()
        .and_then(|row| json_i32(row, "plan_status"))
        .ok_or("PLAN_NOT_FOUND")?;

    let completed_stages = get_completed_stages(db, plan_unit_id).await?;

    Ok(CompleteStudyTaskResponse {
        task_id,
        task_status,
        plan_status,
        completed_stages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Database, SqliteDatabase, migrate_up};
    async fn create_db() -> SqliteDatabase {
        let path = std::env::temp_dir()
            .join(format!("study_plan_test_{}.db", uuid::Uuid::new_v4()))
            .to_str()
            .unwrap()
            .to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();
        migrate_up(&db, None).await.unwrap();
        db.execute("INSERT INTO books (id, book_group, product_code, title, author, product_type, sort_num) VALUES (2001, 1, 'studytestbook', 'Test', NULL, 'imgbook', 1)".to_string()).await.unwrap();
        db
    }

    #[tokio::test]
    async fn upsert_is_idempotent_for_same_unit() {
        let db = create_db().await;

        let first = upsert_study_plan(&db, "studytestbook", "RE_1001", "Unit 1")
            .await
            .unwrap();
        let second = upsert_study_plan(&db, "studytestbook", "RE_1001", "Unit 1")
            .await
            .unwrap();

        assert_eq!(first.plan_unit_id, second.plan_unit_id);

        let task_rows = db
            .query(format!(
                "SELECT id FROM study_tasks WHERE plan_unit_id = {}",
                first.plan_unit_id
            ))
            .await
            .unwrap();
        assert_eq!(task_rows.len(), 7);
    }

    #[tokio::test]
    async fn mastery_requires_all_stages() {
        let db = create_db().await;

        let plan = upsert_study_plan(&db, "studytestbook", "RE_2001", "Unit 2")
            .await
            .unwrap();

        let rows = db
            .query(format!(
                "SELECT id, review_stage FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage ASC",
                plan.plan_unit_id
            ))
            .await
            .unwrap();

        let stage7_id = rows
            .iter()
            .find(|row| json_i32(row, "review_stage") == Some(7))
            .and_then(|row| json_i64(row, "id"))
            .unwrap();

        let stage7_result = complete_study_task(&db, stage7_id).await.unwrap();
        assert_eq!(stage7_result.plan_status, 0);

        for row in rows {
            let stage = json_i32(&row, "review_stage").unwrap();
            if stage == 7 {
                continue;
            }
            let task_id = json_i64(&row, "id").unwrap();
            let _ = complete_study_task(&db, task_id).await.unwrap();
        }

        let status = get_study_plan_status(&db, "studytestbook", "RE_2001")
            .await
            .unwrap();
        assert_eq!(status.plan_status, Some(1));
        assert_eq!(status.completed_stages.len(), 7);
    }

    #[tokio::test]
    async fn get_tasks_by_date_returns_only_selected_day_and_no_overdue_flag() {
        let db = create_db().await;

        let plan = upsert_study_plan(&db, "studytestbook", "RE_3001", "Unit 3")
            .await
            .unwrap();

        let stage_rows = db
            .query(format!(
                "SELECT id, review_stage FROM study_tasks WHERE plan_unit_id = {}",
                plan.plan_unit_id
            ))
            .await
            .unwrap();

        let stage1_id = stage_rows
            .iter()
            .find(|row| json_i32(row, "review_stage") == Some(1))
            .and_then(|row| json_i64(row, "id"))
            .unwrap();
        let stage2_id = stage_rows
            .iter()
            .find(|row| json_i32(row, "review_stage") == Some(2))
            .and_then(|row| json_i64(row, "id"))
            .unwrap();

        db.execute(format!(
            "UPDATE study_tasks SET scheduled_date = date('now', 'localtime', '-1 day') WHERE id = {}",
            stage1_id
        ))
        .await
        .unwrap();
        db.execute(format!(
            "UPDATE study_tasks SET scheduled_date = date('now', 'localtime') WHERE id = {}",
            stage2_id
        ))
        .await
        .unwrap();

        let today_rows = db
            .query("SELECT date('now', 'localtime') AS today".to_string())
            .await
            .unwrap();
        let today = json_string(&today_rows[0], "today").unwrap();

        let tasks = get_tasks_by_date(&db, &today).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].review_stage, 2);
        assert!(!tasks[0].is_overdue);
    }
}
