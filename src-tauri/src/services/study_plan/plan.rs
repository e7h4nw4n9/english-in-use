//! 学习计划的创建、放弃和状态查询。

use super::common::*;
use super::types::*;
use super::*;

/// 将计划聚合查询结果转换为 upsert 响应。
///
/// # 参数
/// - `plan_row`：包含计划及任务状态的聚合行。
/// - `outcome`：本次 upsert 的实际处理结果。
fn build_upsert_response(
    plan_row: &Value,
    outcome: StudyPlanUpsertOutcome,
) -> Result<StudyPlanUpsertResponse, String> {
    let plan_unit_id = json_i64(plan_row, "plan_unit_id").ok_or("PLAN_NOT_FOUND")?;
    let plan_status = json_i32(plan_row, "plan_status").ok_or("PLAN_NOT_FOUND")?;
    let completed_stages = json_completed_stages(plan_row);
    let next_review_date = json_string(plan_row, "next_review_date");

    Ok(StudyPlanUpsertResponse {
        plan_unit_id,
        plan_status,
        next_review_date,
        total_stages: TOTAL_STAGES,
        completed_stages,
        outcome,
    })
}

/// 按显式本地日期创建或更新学习计划。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
/// - `unit_name`：学习单元显示名称。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn upsert_study_plan_on_date(
    db: &dyn Database,
    product_code: &str,
    resource_id: &str,
    unit_name: &str,
    local_date: &str,
) -> Result<StudyPlanUpsertResponse, String> {
    validate_local_date(local_date)?;
    let existing_row = query_plan_status_row(db, product_code, resource_id, local_date)
        .await?
        .ok_or("BOOK_NOT_FOUND")?;
    let existing_status = json_i32(&existing_row, "plan_status");

    if existing_status == Some(0) {
        return build_upsert_response(&existing_row, StudyPlanUpsertOutcome::AlreadyActive);
    }
    if existing_status == Some(1) {
        return build_upsert_response(&existing_row, StudyPlanUpsertOutcome::AlreadyMastered);
    }
    if !matches!(existing_status, None | Some(2)) {
        return Err("INVALID_PLAN_STATUS".to_string());
    }

    let completed_stages = json_completed_stages(&existing_row);
    let all_stages = json_all_stages(&existing_row);
    let restart_all = existing_status == Some(2) && completed_stages.len() == TOTAL_STAGES as usize;
    let outcome = if existing_status.is_none() {
        StudyPlanUpsertOutcome::Created
    } else if restart_all {
        StudyPlanUpsertOutcome::Restarted
    } else {
        StudyPlanUpsertOutcome::Reactivated
    };

    // updated_at 在事务内部临时充当本次状态转换标识，批次末尾会恢复为合法时间。
    let operation_token = format!("operation:{}", uuid::Uuid::new_v4());
    let reset_last_review = restart_all || completed_stages.is_empty();
    let mut statements = vec![SqlStatement::new(
        "INSERT INTO study_plan_units \
         (book_id, resource_id, unit_name, plan_status, created_at, updated_at) \
         SELECT id, ?, ?, 0, CURRENT_TIMESTAMP, ? FROM books WHERE product_code = ? \
         ON CONFLICT(book_id, resource_id) DO UPDATE SET \
           plan_status = 0, \
           last_review_at = CASE WHEN ? THEN NULL ELSE study_plan_units.last_review_at END, \
           updated_at = excluded.updated_at \
         WHERE study_plan_units.plan_status = 2 \
         RETURNING id AS plan_unit_id",
        vec![
            SqlValue::Text(resource_id.to_string()),
            SqlValue::Text(unit_name.to_string()),
            SqlValue::Text(operation_token.clone()),
            SqlValue::Text(product_code.to_string()),
            SqlValue::Boolean(reset_last_review),
        ],
    )];

    let pending_stages: Vec<i32> = if restart_all || existing_status.is_none() {
        (1..=TOTAL_STAGES).collect()
    } else {
        (1..=TOTAL_STAGES)
            .filter(|stage| !completed_stages.contains(stage))
            .collect()
    };

    for stage in 1..=TOTAL_STAGES {
        if all_stages.contains(&stage) {
            continue;
        }
        let pending_index = pending_stages
            .iter()
            .position(|pending_stage| *pending_stage == stage)
            .unwrap_or((stage - 1) as usize);
        let offset = REVIEW_DAY_OFFSETS[pending_index];
        statements.push(SqlStatement::new(
            format!(
                "INSERT INTO study_tasks \
                 (plan_unit_id, scheduled_date, review_stage, task_status, created_at, updated_at) \
                 SELECT u.id, date(?, '+{offset} day'), ?, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP \
                 FROM study_plan_units u \
                 JOIN books b ON b.id = u.book_id \
                 WHERE b.product_code = ? AND u.resource_id = ? AND u.updated_at = ? \
                 ON CONFLICT(plan_unit_id, review_stage) DO NOTHING"
            ),
            vec![
                SqlValue::Text(local_date.to_string()),
                SqlValue::Integer(i64::from(stage)),
                SqlValue::Text(product_code.to_string()),
                SqlValue::Text(resource_id.to_string()),
                SqlValue::Text(operation_token.clone()),
            ],
        ));
    }

    if existing_status == Some(2) {
        for (index, stage) in pending_stages.iter().enumerate() {
            let offset = REVIEW_DAY_OFFSETS[index];
            let reset_columns = if restart_all {
                ", task_status = 0, completed_at = NULL"
            } else {
                ""
            };
            statements.push(SqlStatement::new(
                format!(
                    "UPDATE study_tasks SET scheduled_date = date(?, '+{offset} day'){reset_columns}, \
                     updated_at = CURRENT_TIMESTAMP \
                     WHERE plan_unit_id = (\
                       SELECT u.id FROM study_plan_units u \
                       JOIN books b ON b.id = u.book_id \
                       WHERE b.product_code = ? AND u.resource_id = ? AND u.updated_at = ?\
                     ) AND review_stage = ?"
                ),
                vec![
                    SqlValue::Text(local_date.to_string()),
                    SqlValue::Text(product_code.to_string()),
                    SqlValue::Text(resource_id.to_string()),
                    SqlValue::Text(operation_token.clone()),
                    SqlValue::Integer(i64::from(*stage)),
                ],
            ));
        }
    }

    statements.push(SqlStatement::new(
        "UPDATE study_plan_units SET updated_at = CURRENT_TIMESTAMP \
         WHERE book_id = (SELECT id FROM books WHERE product_code = ?) \
           AND resource_id = ? AND updated_at = ?",
        vec![
            SqlValue::Text(product_code.to_string()),
            SqlValue::Text(resource_id.to_string()),
            SqlValue::Text(operation_token),
        ],
    ));

    let transition_applied = db
        .query_write_batch(statements)
        .await
        .map_err(|e| e.to_string())?
        .first()
        .is_some_and(|rows| !rows.is_empty());

    let plan_row = query_plan_status_row(db, product_code, resource_id, local_date)
        .await?
        .ok_or("BOOK_NOT_FOUND")?;
    if transition_applied {
        return build_upsert_response(&plan_row, outcome);
    }

    match json_i32(&plan_row, "plan_status") {
        Some(0) => build_upsert_response(&plan_row, StudyPlanUpsertOutcome::AlreadyActive),
        Some(1) => build_upsert_response(&plan_row, StudyPlanUpsertOutcome::AlreadyMastered),
        _ => Err("PLAN_STATE_CHANGED".to_string()),
    }
}

/// 将指定单元的活动学习计划标记为已放弃。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
pub async fn abandon_study_plan(
    db: &dyn Database,
    product_code: &str,
    resource_id: &str,
) -> Result<StudyPlanActionResponse, String> {
    let rows = db
        .query_write_statement(SqlStatement::new(
            "UPDATE study_plan_units SET plan_status = 2, updated_at = CURRENT_TIMESTAMP \
             WHERE book_id = (SELECT id FROM books WHERE product_code = ?) \
               AND resource_id = ? RETURNING id",
            vec![
                SqlValue::Text(product_code.to_string()),
                SqlValue::Text(resource_id.to_string()),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?;
    if rows.is_empty() {
        return Err("PLAN_NOT_FOUND".to_string());
    }

    Ok(StudyPlanActionResponse { success: true })
}

/// 按显式本地日期查询学习计划状态。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_study_plan_status_on_date(
    db: &dyn Database,
    product_code: &str,
    resource_id: &str,
    local_date: &str,
) -> Result<StudyPlanStatusResponse, String> {
    validate_local_date(local_date)?;
    let plan_row = query_plan_status_row(db, product_code, resource_id, local_date).await?;

    let plan_row = plan_row.ok_or("BOOK_NOT_FOUND")?;

    let Some(plan_unit_id) = json_i64(&plan_row, "plan_unit_id") else {
        return Ok(StudyPlanStatusResponse {
            in_plan: false,
            plan_status: None,
            plan_unit_id: None,
            completed_stages: Vec::new(),
            next_review_date: None,
            overdue_count: 0,
        });
    };
    let plan_status = json_i32(&plan_row, "plan_status").ok_or("PLAN_NOT_FOUND")?;
    let completed_stages = json_completed_stages(&plan_row);
    let next_review_date = json_string(&plan_row, "next_review_date");
    let overdue_count = json_i64(&plan_row, "overdue_count").unwrap_or(0);

    Ok(StudyPlanStatusResponse {
        in_plan: true,
        plan_status: Some(plan_status),
        plan_unit_id: Some(plan_unit_id),
        completed_stages,
        next_review_date,
        overdue_count,
    })
}

#[cfg(test)]
mod request_count_tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct StatusOnlyDatabase {
        query_calls: AtomicUsize,
    }

    impl Database for StatusOnlyDatabase {
        fn execute(
            &self,
            _sql: String,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async { anyhow::bail!("unexpected execute") })
        }

        fn query(
            &self,
            _sql: String,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Value>>> + Send + '_>> {
            Box::pin(async { anyhow::bail!("unexpected query") })
        }

        fn query_statement(
            &self,
            _statement: SqlStatement,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Value>>> + Send + '_>> {
            self.query_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(vec![serde_json::json!({
                    "book_id": 1,
                    "plan_unit_id": 2,
                    "plan_status": 0,
                    "completed_stages_json": "[1,2]",
                    "next_review_date": "2026-08-04",
                    "overdue_count": 1
                })])
            })
        }

        fn query_write_batch(
            &self,
            _statements: Vec<SqlStatement>,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Vec<Value>>>> + Send + '_>> {
            Box::pin(async { anyhow::bail!("unexpected write batch") })
        }

        fn get_version(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + '_>> {
            Box::pin(async { Ok("0.5.0".to_string()) })
        }
    }

    #[tokio::test]
    async fn explicit_date_status_uses_one_aggregate_query() {
        let database = StatusOnlyDatabase {
            query_calls: AtomicUsize::new(0),
        };

        let status = get_study_plan_status_on_date(&database, "book", "resource", "2026-08-03")
            .await
            .unwrap();

        assert!(status.in_plan);
        assert_eq!(status.completed_stages, vec![1, 2]);
        assert_eq!(database.query_calls.load(Ordering::SeqCst), 1);
    }
}
