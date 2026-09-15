//! 评估预览与提交共用的间隔算法。
use super::common::*;
use super::*;
use time::{Date, Duration, format_description};

pub(super) struct AssessmentContext {
    pub plan_id: i64,
    pub task_status: i32,
    pub plan_status: i32,
    pub completed: Vec<i32>,
    previous: Option<Value>,
}

/// 读取当前任务和最近一次有效评估；参数为数据库和任务标识。
pub(super) async fn load_context(
    db: &dyn Database,
    task_id: i64,
) -> Result<AssessmentContext, String> {
    let rows = db.query_statement(SqlStatement::new(
        "SELECT t.plan_unit_id, t.task_status, u.plan_status, h.id AS history_id, h.review_stage, h.mastery_rating, h.mastered_streak, h.assessment_local_date, h.next_interval_days FROM study_tasks t JOIN study_plan_units u ON u.id = t.plan_unit_id LEFT JOIN study_tasks h ON h.plan_unit_id = t.plan_unit_id AND h.task_status = 1 WHERE t.id = ? ORDER BY h.review_stage ASC",
        vec![SqlValue::Integer(task_id)],
    )).await.map_err(|e| e.to_string())?;
    let row = rows.first().ok_or("TASK_NOT_FOUND")?;
    let plan_id = json_i64(row, "plan_unit_id").ok_or("PLAN_NOT_FOUND")?;
    // 一次读取完成历史，避免远程数据库的三次串行往返；无历史时仍保留任务行。
    let completed = rows
        .iter()
        .filter_map(|r| json_i32(r, "review_stage"))
        .collect();
    let previous = rows
        .iter()
        .rev()
        .find(|r| json_string(r, "mastery_rating").is_some())
        .map(|r| {
            serde_json::json!({
                "id": r["history_id"],
                "mastery_rating": r["mastery_rating"],
                "mastered_streak": r["mastered_streak"],
                "assessment_local_date": r["assessment_local_date"],
                "next_interval_days": r["next_interval_days"],
            })
        });
    Ok(AssessmentContext {
        plan_id,
        task_status: json_i32(row, "task_status").unwrap_or_default(),
        plan_status: json_i32(row, "plan_status").unwrap_or_default(),
        completed,
        previous,
    })
}

/// 校验计划状态；参数为当前上下文。
pub(super) fn validate_active(context: &AssessmentContext) -> Result<(), String> {
    match context.plan_status {
        0 => Ok(()),
        1 => Err("PLAN_ALREADY_MASTERED".into()),
        _ => Err("PLAN_NOT_ACTIVE".into()),
    }
}

/// 兼容旧算法：旧记录没有保存间隔时，根据当时的评价和计数还原。
fn previous_interval(row: &Value) -> i32 {
    if let Some(days) = json_i32(row, "next_interval_days").filter(|n| *n > 0) {
        return days;
    }
    match json_string(row, "mastery_rating").as_deref() {
        Some("hard") => 3,
        Some("good") => 14,
        Some("mastered") => match json_i32(row, "mastered_streak").unwrap_or(1) {
            0 | 1 => 30,
            2 => 60,
            _ => 120,
        },
        _ => 1,
    }
}

/// 计算四种评价的候选安排；参数为上下文、任务标识及本次本地日期。
pub(super) fn calculate_preview(
    context: &AssessmentContext,
    task_id: i64,
    local_date: &str,
) -> Result<StudyAssessmentPreview, String> {
    validate_local_date(local_date)?;
    let format = format_description::parse("[year]-[month]-[day]").map_err(|e| e.to_string())?;
    let date = Date::parse(local_date, &format).map_err(|_| "INVALID_LOCAL_DATE")?;
    let previous = context.previous.as_ref();
    let interval = previous.map(previous_interval);
    let streak = previous
        .and_then(|r| json_i32(r, "mastered_streak"))
        .unwrap_or(0)
        .clamp(0, 4);
    let previous_date = previous.and_then(|r| json_string(r, "assessment_local_date"));
    let same_day = previous_date.as_deref().is_some_and(|d| local_date <= d);
    let legacy_date = previous.is_some() && previous_date.is_none();
    let can_advance = !same_day && !legacy_date;
    let mut options = Vec::new();
    for rating in ["forgotten", "hard", "good", "mastered"] {
        let next_streak = if rating == "mastered" {
            if can_advance {
                (streak + 1).min(4)
            } else {
                streak
            }
        } else {
            0
        };
        let mut days = match rating {
            "forgotten" => 1,
            "hard" => interval
                .map(|i| ((i64::from(i) + 1) / 2).clamp(1, 7) as i32)
                .unwrap_or(3),
            "good" => interval
                .map(|i| ((i64::from(i) * 3 + 1) / 2).clamp(3, 30) as i32)
                .unwrap_or(14),
            _ => match next_streak {
                0 | 1 => 30,
                2 => 60,
                _ => 120,
            },
        };
        if same_day {
            days = days.min(interval.unwrap_or(days));
        }
        let scheduled_date = date
            .checked_add(Duration::days(i64::from(days)))
            .ok_or("INVALID_LOCAL_DATE")?
            .to_string();
        options.push(StudyAssessmentOption {
            rating: rating.into(),
            interval_days: days,
            scheduled_date,
            mastered_streak: next_streak,
            requires_finish_decision: rating == "mastered" && next_streak >= 4 && can_advance,
        });
    }
    // 日期和历史均参与版本标识，跨日或另一设备完成任务后旧预览不能提交。
    let revision = format!(
        "{task_id}:{}:{local_date}:{}",
        context.completed.len(),
        serde_json::to_string(&context.previous).map_err(|e| e.to_string())?
    );
    Ok(StudyAssessmentPreview {
        local_date: local_date.into(),
        revision,
        same_day,
        legacy_date,
        options,
    })
}

/// 获取评估预览；参数为数据库、未完成任务标识、本地日期。
pub async fn get_assessment_preview(
    db: &dyn Database,
    task_id: i64,
    local_date: &str,
) -> Result<StudyAssessmentPreview, String> {
    let context = load_context(db, task_id).await?;
    validate_active(&context)?;
    if context.task_status == 1 {
        return Err("TASK_ALREADY_COMPLETED".into());
    }
    if context.completed.len() < 4 {
        return Err("ASSESSMENT_NOT_REQUIRED".into());
    }
    calculate_preview(&context, task_id, local_date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 构造已完成四次的评估上下文，previous 为可选历史记录。
    fn context(previous: Option<Value>) -> AssessmentContext {
        AssessmentContext {
            plan_id: 1,
            task_status: 0,
            plan_status: 0,
            completed: vec![1, 2, 3, 4],
            previous,
        }
    }

    /// 生成旧日评估记录；参数为间隔和巩固计数。
    fn previous(days: i32, streak: i32) -> Value {
        json!({"id": 4, "mastery_rating": "good", "next_interval_days": days, "mastered_streak": streak, "assessment_local_date": "2026-09-08"})
    }

    #[test]
    fn first_rating_and_dynamic_bounds() {
        let first = calculate_preview(&context(None), 5, "2026-09-09").unwrap();
        assert_eq!(
            first
                .options
                .iter()
                .map(|o| o.interval_days)
                .collect::<Vec<_>>(),
            vec![1, 3, 14, 30]
        );
        for (interval, hard, good) in [
            (1, 1, 3),
            (2, 1, 3),
            (3, 2, 5),
            (5, 3, 8),
            (8, 4, 12),
            (12, 6, 18),
            (18, 7, 27),
            (27, 7, 30),
            (120, 7, 30),
        ] {
            let result =
                calculate_preview(&context(Some(previous(interval, 0))), 5, "2026-09-09").unwrap();
            assert_eq!(result.options[0].interval_days, 1);
            assert_eq!(result.options[1].interval_days, hard);
            assert_eq!(result.options[2].interval_days, good);
        }
    }

    #[test]
    fn mastery_progression_and_reset() {
        for (streak, days, decision) in [
            (0, 30, false),
            (1, 60, false),
            (2, 120, false),
            (3, 120, true),
            (4, 120, true),
        ] {
            let result =
                calculate_preview(&context(Some(previous(30, streak))), 5, "2026-09-09").unwrap();
            let mastery = &result.options[3];
            assert_eq!(
                (mastery.interval_days, mastery.requires_finish_decision),
                (days, decision)
            );
            assert!(result.options[..3].iter().all(|o| o.mastered_streak == 0));
        }
    }

    #[test]
    fn same_day_cannot_extend_or_finish() {
        for streak in [0, 1, 3, 4] {
            let result =
                calculate_preview(&context(Some(previous(3, streak))), 5, "2026-09-08").unwrap();
            assert!(result.same_day);
            assert!(
                result
                    .options
                    .iter()
                    .all(|o| o.interval_days <= 3 && !o.requires_finish_decision)
            );
            assert_eq!(result.options[3].mastered_streak, streak);
            assert_eq!(result.options[0].mastered_streak, 0);
        }
    }

    #[test]
    fn legacy_intervals_and_unknown_date_are_preserved() {
        for (rating, streak, hard) in [
            ("forgotten", 0, 1),
            ("hard", 0, 2),
            ("good", 0, 7),
            ("mastered", 1, 7),
            ("mastered", 2, 7),
            ("mastered", 3, 7),
        ] {
            let row = json!({"id": 4, "mastery_rating": rating, "mastered_streak": streak});
            let result = calculate_preview(&context(Some(row)), 5, "2026-09-09").unwrap();
            assert!(result.legacy_date);
            assert_eq!(result.options[1].interval_days, hard);
            assert_eq!(result.options[3].mastered_streak, streak);
            assert!(!result.options[3].requires_finish_decision);
        }
    }

    #[test]
    fn overdue_uses_planned_interval_and_calendar_dates() {
        let result = calculate_preview(&context(Some(previous(3, 0))), 5, "2028-02-28").unwrap();
        assert_eq!(result.options[0].scheduled_date, "2028-02-29");
        assert_eq!(result.options[1].scheduled_date, "2028-03-01");
        assert_eq!(result.options[2].interval_days, 5);
        let tomorrow = calculate_preview(&context(Some(previous(3, 0))), 5, "2028-02-29").unwrap();
        assert_ne!(result.revision, tomorrow.revision);
    }
}
