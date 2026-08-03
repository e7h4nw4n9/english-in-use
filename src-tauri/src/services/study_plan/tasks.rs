//! 学习任务汇总、日期查询与完成流程。

use super::common::*;
use super::types::*;
use super::*;

/// 按显式本地日期汇总范围内的学习任务。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `range_start`：统计范围开始日期。
/// - `range_end`：统计范围结束日期。
/// - `view_mode`：学习任务汇总视图模式。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_study_tasks_summary_on_date(
    db: &dyn Database,
    range_start: &str,
    range_end: &str,
    view_mode: &str,
    local_date: &str,
) -> Result<StudyTaskSummaryResponse, String> {
    if !matches!(view_mode, "day" | "week" | "month") {
        return Err("INVALID_VIEW_MODE".to_string());
    }
    validate_local_date(local_date)?;

    let range_start_raw = range_start.to_string();
    let range_end_raw = range_end.to_string();

    let sql = "SELECT \
            t.scheduled_date AS scheduled_date, \
            t.task_status AS task_status \
        FROM study_tasks t \
        JOIN study_plan_units u ON t.plan_unit_id = u.id \
        WHERE u.plan_status = 0 \
          AND t.scheduled_date >= ? \
          AND t.scheduled_date <= ? \
        ORDER BY t.scheduled_date ASC"
        .to_string();
    let rows = db
        .query_statement(SqlStatement::new(
            sql,
            vec![
                SqlValue::Text(range_start.to_string()),
                SqlValue::Text(range_end.to_string()),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?;

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
            if entry.date.as_str() < local_date {
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

/// 按显式本地日期查询指定日期及逾期任务。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `date`：需要查询的日期。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_tasks_by_date_on_date(
    db: &dyn Database,
    date: &str,
    local_date: &str,
) -> Result<Vec<StudyTaskItem>, String> {
    validate_local_date(date)?;
    validate_local_date(local_date)?;
    let date_raw = date.to_string();
    let include_overdue = date_raw == local_date;

    let (date_condition, mut params) = if include_overdue {
        (
            "(t.scheduled_date = ? OR (t.scheduled_date < ? AND t.task_status = 0))",
            vec![
                SqlValue::Text(date_raw.clone()),
                SqlValue::Text(date_raw.clone()),
            ],
        )
    } else {
        (
            "t.scheduled_date = ?",
            vec![SqlValue::Text(date_raw.clone())],
        )
    };

    let order_by_clause = if include_overdue {
        params.push(SqlValue::Text(date_raw.clone()));
        "CASE WHEN t.scheduled_date < ? THEN 0 ELSE 1 END ASC, t.scheduled_date ASC, t.review_stage ASC, t.id ASC"
            .to_string()
    } else {
        "t.review_stage ASC, t.id ASC".to_string()
    };

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
          AND {} \
        ORDER BY {}",
        date_condition, order_by_clause
    );

    let rows = db
        .query_statement(SqlStatement::new(sql, params))
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|row| {
            let scheduled_date = json_string(row, "scheduled_date").unwrap_or_default();
            let task_status = json_i32(row, "task_status").unwrap_or_default();
            let is_overdue = task_status == 0
                && !scheduled_date.is_empty()
                && scheduled_date.as_str() < date_raw.as_str();

            StudyTaskItem {
                task_id: json_i64(row, "task_id").unwrap_or_default(),
                plan_unit_id: json_i64(row, "plan_unit_id").unwrap_or_default(),
                product_code: json_string(row, "product_code").unwrap_or_default(),
                resource_id: json_string(row, "resource_id").unwrap_or_default(),
                unit_name: json_string(row, "unit_name").unwrap_or_default(),
                review_stage: json_i32(row, "review_stage").unwrap_or_default(),
                scheduled_date,
                task_status,
                is_overdue,
                completed_at: json_string(row, "completed_at"),
            }
        })
        .collect())
}

/// 完成指定任务并创建下一复习阶段或结束计划。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `task_id`：学习任务数据库标识。
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

    if task_status == 1 {
        let completed_stages = get_completed_stages(db, plan_unit_id).await?;
        return Ok(CompleteStudyTaskResponse {
            task_id,
            task_status,
            plan_status: plan_status_before,
            completed_stages,
        });
    }

    if plan_status_before != 0 {
        return Err(if plan_status_before == 1 {
            "PLAN_ALREADY_MASTERED".to_string()
        } else {
            "PLAN_NOT_ACTIVE".to_string()
        });
    }

    db.execute_batch(vec![
        SqlStatement::new(
            "UPDATE study_tasks SET task_status = 1, completed_at = CURRENT_TIMESTAMP, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ? AND task_status = 0",
            vec![SqlValue::Integer(task_id)],
        ),
        SqlStatement::new(
            "UPDATE study_plan_units SET last_review_at = CURRENT_TIMESTAMP, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ? AND plan_status = 0",
            vec![SqlValue::Integer(plan_unit_id)],
        ),
        SqlStatement::new(
            "UPDATE study_plan_units SET plan_status = 1, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND plan_status = 0 AND NOT EXISTS (\
               SELECT 1 FROM study_tasks WHERE plan_unit_id = ? AND task_status = 0\
             )",
            vec![
                SqlValue::Integer(plan_unit_id),
                SqlValue::Integer(plan_unit_id),
            ],
        ),
    ])
    .await
    .map_err(|e| e.to_string())?;
    task_status = 1;

    let result_rows = db
        .query_statement(SqlStatement::new(
            "SELECT u.plan_status AS plan_status, \
                    COALESCE((SELECT json_group_array(review_stage) FROM (\
                        SELECT review_stage FROM study_tasks \
                        WHERE plan_unit_id = u.id AND task_status = 1 ORDER BY review_stage\
                    )), '[]') AS completed_stages_json \
             FROM study_plan_units u WHERE u.id = ? LIMIT 1",
            vec![SqlValue::Integer(plan_unit_id)],
        ))
        .await
        .map_err(|e| e.to_string())?;
    let result_row = result_rows.first().ok_or("PLAN_NOT_FOUND")?;
    let plan_status = json_i32(result_row, "plan_status").ok_or("PLAN_NOT_FOUND")?;
    let completed_stages = json_completed_stages(result_row);

    Ok(CompleteStudyTaskResponse {
        task_id,
        task_status,
        plan_status,
        completed_stages,
    })
}
