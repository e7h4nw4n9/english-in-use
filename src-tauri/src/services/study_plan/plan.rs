//! 学习计划的创建、放弃和状态查询。

use super::common::*;
use super::types::*;
use super::*;

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
    let mut statements = vec![SqlStatement::new(
        "INSERT OR IGNORE INTO study_plan_units \
         (book_id, resource_id, unit_name, plan_status, created_at, updated_at) \
         SELECT id, ?, ?, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP \
         FROM books WHERE product_code = ?",
        vec![
            SqlValue::Text(resource_id.to_string()),
            SqlValue::Text(unit_name.to_string()),
            SqlValue::Text(product_code.to_string()),
        ],
    )];
    for (index, offset) in REVIEW_DAY_OFFSETS.iter().enumerate() {
        statements.push(SqlStatement::new(
            format!(
                "INSERT OR IGNORE INTO study_tasks \
                 (plan_unit_id, scheduled_date, review_stage, task_status, created_at, updated_at) \
                 SELECT u.id, date(?, '+{offset} day'), ?, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP \
                 FROM study_plan_units u \
                 JOIN books b ON b.id = u.book_id \
                 WHERE b.product_code = ? AND u.resource_id = ?"
            ),
            vec![
                SqlValue::Text(local_date.to_string()),
                SqlValue::Integer((index + 1) as i64),
                SqlValue::Text(product_code.to_string()),
                SqlValue::Text(resource_id.to_string()),
            ],
        ));
    }
    statements.push(SqlStatement::new(
        "UPDATE study_plan_units SET plan_status = 0, updated_at = CURRENT_TIMESTAMP \
         WHERE book_id = (SELECT id FROM books WHERE product_code = ?) \
           AND resource_id = ? AND plan_status = 2",
        vec![
            SqlValue::Text(product_code.to_string()),
            SqlValue::Text(resource_id.to_string()),
        ],
    ));
    db.execute_batch(statements)
        .await
        .map_err(|e| e.to_string())?;

    let plan_row = query_plan_status_row(db, product_code, resource_id, local_date)
        .await?
        .ok_or("BOOK_NOT_FOUND")?;
    let plan_unit_id = json_i64(&plan_row, "plan_unit_id").ok_or("PLAN_NOT_FOUND")?;
    let plan_status = json_i32(&plan_row, "plan_status").ok_or("PLAN_NOT_FOUND")?;
    let completed_stages = json_completed_stages(&plan_row);
    let next_review_date = json_string(&plan_row, "next_review_date");

    Ok(StudyPlanUpsertResponse {
        plan_unit_id,
        plan_status,
        next_review_date,
        total_stages: TOTAL_STAGES,
        completed_stages,
    })
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
