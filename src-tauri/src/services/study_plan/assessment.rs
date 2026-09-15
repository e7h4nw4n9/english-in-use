//! 根据实际完成日期和掌握评价安排下一次复习。
use super::assessment_rules::*;
use super::common::*;
use super::*;

/// 完成任务；第五次及以后必须提供评价，连续三次巩固后必须选择是否结束。
/// 参数依次为数据库、任务标识、本地日期、掌握评价、结束选择。
pub async fn complete_assessed_task(
    db: &dyn Database,
    task_id: i64,
    local_date: &str,
    rating: Option<&str>,
    finish_plan: Option<bool>,
) -> Result<CompleteStudyTaskResponse, String> {
    complete_assessed_task_with_preview(db, task_id, local_date, rating, finish_plan, None).await
}

/// 按共用算法完成任务；额外参数为客户端确认过的预览版本。
pub async fn complete_assessed_task_with_preview(
    db: &dyn Database,
    task_id: i64,
    local_date: &str,
    rating: Option<&str>,
    finish_plan: Option<bool>,
    expected_revision: Option<&str>,
) -> Result<CompleteStudyTaskResponse, String> {
    validate_local_date(local_date)?;
    let context = load_context(db, task_id).await?;
    let plan_id = context.plan_id;
    let status = context.plan_status;
    if context.task_status == 1 {
        return Ok(CompleteStudyTaskResponse {
            task_id,
            task_status: 1,
            plan_status: status,
            completed_stages: context.completed,
        });
    }
    validate_active(&context)?;
    let requires_assessment = context.completed.len() >= 4;
    let mut streak = 0;
    let mut days = 0;
    let mut finish = false;
    if requires_assessment {
        let rating = rating.ok_or("ASSESSMENT_REQUIRED")?;
        let preview = calculate_preview(&context, task_id, local_date)?;
        if expected_revision.is_some_and(|revision| revision != preview.revision) {
            return Err("ASSESSMENT_PREVIEW_CHANGED".into());
        }
        let option = preview
            .options
            .iter()
            .find(|o| o.rating == rating)
            .ok_or("INVALID_MASTERY_RATING")?;
        days = option.interval_days;
        streak = option.mastered_streak;
        if option.requires_finish_decision {
            finish = finish_plan.ok_or("FINISH_DECISION_REQUIRED")?;
        }
        if finish_plan == Some(true) && !finish {
            return Err("FINISH_NOT_ALLOWED".into());
        }
    } else if rating.is_some() || finish_plan.is_some() {
        return Err("ASSESSMENT_NOT_REQUIRED".into());
    }

    // 以完成数量作为乐观锁，操作标记使整个事务的后续语句仅由获胜请求执行。
    let operation = format!("assessment:{}", uuid::Uuid::new_v4());
    let mut statements = vec![SqlStatement::new(
        "UPDATE study_tasks SET task_status = 1, completed_at = CURRENT_TIMESTAMP, updated_at = ?, mastery_rating = ?, mastered_streak = ?, assessment_local_date = ?, next_interval_days = ? WHERE id = ? AND task_status = 0 AND (SELECT COUNT(*) FROM study_tasks WHERE plan_unit_id = ? AND task_status = 1) = ? AND EXISTS (SELECT 1 FROM study_plan_units WHERE id = ? AND plan_status = 0) RETURNING id",
        vec![
            SqlValue::Text(operation.clone()),
            rating
                .map(|r| SqlValue::Text(r.into()))
                .unwrap_or(SqlValue::Null),
            SqlValue::Integer(streak.into()),
            if requires_assessment {
                SqlValue::Text(local_date.into())
            } else {
                SqlValue::Null
            },
            if requires_assessment && !finish {
                SqlValue::Integer(days.into())
            } else {
                SqlValue::Null
            },
            SqlValue::Integer(task_id),
            SqlValue::Integer(plan_id),
            SqlValue::Integer(context.completed.len() as i64),
            SqlValue::Integer(plan_id),
        ],
    )];
    if requires_assessment {
        statements.push(SqlStatement::new(
            "DELETE FROM study_tasks WHERE plan_unit_id = ? AND task_status = 0 AND EXISTS (SELECT 1 FROM study_tasks WHERE id = ? AND updated_at = ?)",
            vec![SqlValue::Integer(plan_id), SqlValue::Integer(task_id), SqlValue::Text(operation.clone())],
        ));
        if !finish {
            statements.push(SqlStatement::new(
                "INSERT INTO study_tasks (plan_unit_id, scheduled_date, review_stage) SELECT ?, date(?, ?), (SELECT COALESCE(MAX(review_stage), 0) + 1 FROM study_tasks WHERE plan_unit_id = ?) WHERE EXISTS (SELECT 1 FROM study_tasks WHERE id = ? AND updated_at = ?)",
                vec![SqlValue::Integer(plan_id), SqlValue::Text(local_date.into()), SqlValue::Text(format!("+{days} day")), SqlValue::Integer(plan_id), SqlValue::Integer(task_id), SqlValue::Text(operation.clone())],
            ));
        }
    }
    statements.push(SqlStatement::new(
        "UPDATE study_plan_units SET plan_status = ?, last_review_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND EXISTS (SELECT 1 FROM study_tasks WHERE id = ? AND updated_at = ?)",
        vec![SqlValue::Integer(if finish { 1 } else { 0 }), SqlValue::Integer(plan_id), SqlValue::Integer(task_id), SqlValue::Text(operation.clone())],
    ));
    statements.push(SqlStatement::new(
        "UPDATE study_tasks SET updated_at = CURRENT_TIMESTAMP WHERE id = ? AND updated_at = ?",
        vec![SqlValue::Integer(task_id), SqlValue::Text(operation)],
    ));
    let result = db
        .query_write_batch(statements)
        .await
        .map_err(|e| e.to_string())?;
    if result.first().is_none_or(|rows| rows.is_empty()) {
        return Err("PLAN_STATE_CHANGED".into());
    }
    Ok(CompleteStudyTaskResponse {
        task_id,
        task_status: 1,
        plan_status: if finish { 1 } else { 0 },
        completed_stages: get_completed_stages(db, plan_id).await?,
    })
}
