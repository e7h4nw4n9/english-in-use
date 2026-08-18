//! 学习计划服务回归测试。

use super::common::{json_i32, json_i64, json_string, validate_local_date};
use super::*;
use crate::database::{Database, SqliteDatabase, migrate_up};
use std::sync::Arc;

const TEST_DATE: &str = "2026-08-03";

#[test]
fn local_date_validation_rejects_invalid_calendar_dates() {
    assert!(validate_local_date("2024-02-29").is_ok());
    assert_eq!(
        validate_local_date("2025-02-29"),
        Err("INVALID_LOCAL_DATE".to_string())
    );
    assert_eq!(
        validate_local_date("2025-13-01"),
        Err("INVALID_LOCAL_DATE".to_string())
    );
}

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

    let first = upsert_study_plan_on_date(&db, "studytestbook", "RE_1001", "Unit 1", TEST_DATE)
        .await
        .unwrap();
    let second = upsert_study_plan_on_date(&db, "studytestbook", "RE_1001", "Unit 1", "2026-08-10")
        .await
        .unwrap();

    assert_eq!(first.plan_unit_id, second.plan_unit_id);
    assert_eq!(first.outcome, StudyPlanUpsertOutcome::Created);
    assert_eq!(second.outcome, StudyPlanUpsertOutcome::AlreadyActive);

    let task_rows = db
        .query(format!(
            "SELECT id, review_stage, scheduled_date FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage ASC",
            first.plan_unit_id
        ))
        .await
        .unwrap();
    assert_eq!(task_rows.len(), 7);
    assert_eq!(
        json_string(&task_rows[0], "scheduled_date").as_deref(),
        Some("2026-08-04")
    );
}

#[tokio::test]
async fn concurrent_upsert_reports_only_one_created_outcome() {
    let db = Arc::new(create_db().await);
    let first_db = Arc::clone(&db);
    let second_db = Arc::clone(&db);

    let (first, second) = tokio::join!(
        upsert_study_plan_on_date(
            first_db.as_ref(),
            "studytestbook",
            "RE_1002",
            "Unit 1.2",
            TEST_DATE,
        ),
        upsert_study_plan_on_date(
            second_db.as_ref(),
            "studytestbook",
            "RE_1002",
            "Unit 1.2",
            "2026-08-10",
        )
    );
    let outcomes = [first.unwrap().outcome, second.unwrap().outcome];

    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == StudyPlanUpsertOutcome::Created)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == StudyPlanUpsertOutcome::AlreadyActive)
            .count(),
        1
    );

    let rows = db
        .query(
            "SELECT COUNT(*) AS count FROM study_tasks t \
             JOIN study_plan_units u ON u.id = t.plan_unit_id \
             WHERE u.resource_id = 'RE_1002'"
                .to_string(),
        )
        .await
        .unwrap();
    assert_eq!(json_i64(&rows[0], "count"), Some(7));
}

#[tokio::test]
async fn concurrent_reactivation_reports_only_one_reactivated_outcome() {
    let db = Arc::new(create_db().await);
    upsert_study_plan_on_date(
        db.as_ref(),
        "studytestbook",
        "RE_1003",
        "Unit 1.3",
        TEST_DATE,
    )
    .await
    .unwrap();
    abandon_study_plan(db.as_ref(), "studytestbook", "RE_1003")
        .await
        .unwrap();

    let first_db = Arc::clone(&db);
    let second_db = Arc::clone(&db);
    let (first, second) = tokio::join!(
        upsert_study_plan_on_date(
            first_db.as_ref(),
            "studytestbook",
            "RE_1003",
            "Unit 1.3",
            "2026-08-10",
        ),
        upsert_study_plan_on_date(
            second_db.as_ref(),
            "studytestbook",
            "RE_1003",
            "Unit 1.3",
            "2026-08-10",
        )
    );
    let outcomes = [first.unwrap().outcome, second.unwrap().outcome];

    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == StudyPlanUpsertOutcome::Reactivated)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == StudyPlanUpsertOutcome::AlreadyActive)
            .count(),
        1
    );
}

#[tokio::test]
async fn new_plan_starts_from_next_local_day() {
    let db = create_db().await;

    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_1101", "Unit 1.1", TEST_DATE)
        .await
        .unwrap();
    let rows = db
        .query(format!(
            "SELECT review_stage, scheduled_date FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage ASC",
            plan.plan_unit_id
        ))
        .await
        .unwrap();

    let dates: Vec<String> = rows
        .iter()
        .filter_map(|row| json_string(row, "scheduled_date"))
        .collect();
    assert_eq!(
        dates,
        [
            "2026-08-04",
            "2026-08-05",
            "2026-08-07",
            "2026-08-10",
            "2026-08-18",
            "2026-09-02",
            "2026-10-02",
        ]
    );
    assert!(!dates.iter().any(|date| date == TEST_DATE));
}

#[tokio::test]
async fn reactivating_partial_plan_preserves_completed_tasks_and_compacts_pending_schedule() {
    let db = create_db().await;
    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_1201", "Unit 1.2", TEST_DATE)
        .await
        .unwrap();
    let original_rows = db
        .query(format!(
            "SELECT id, review_stage, scheduled_date FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage ASC",
            plan.plan_unit_id
        ))
        .await
        .unwrap();

    for stage in [1, 3] {
        let task_id = original_rows
            .iter()
            .find(|row| json_i32(row, "review_stage") == Some(stage))
            .and_then(|row| json_i64(row, "id"))
            .unwrap();
        complete_study_task(&db, task_id).await.unwrap();
    }
    abandon_study_plan(&db, "studytestbook", "RE_1201")
        .await
        .unwrap();

    let reactivated =
        upsert_study_plan_on_date(&db, "studytestbook", "RE_1201", "Unit 1.2", "2026-08-10")
            .await
            .unwrap();
    assert_eq!(reactivated.outcome, StudyPlanUpsertOutcome::Reactivated);
    assert_eq!(reactivated.completed_stages, vec![1, 3]);

    let rows = db
        .query(format!(
            "SELECT review_stage, scheduled_date, task_status, completed_at FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage ASC",
            plan.plan_unit_id
        ))
        .await
        .unwrap();
    let expected_pending = [
        (2, "2026-08-11"),
        (4, "2026-08-12"),
        (5, "2026-08-14"),
        (6, "2026-08-17"),
        (7, "2026-08-25"),
    ];
    for (stage, expected_date) in expected_pending {
        let row = rows
            .iter()
            .find(|row| json_i32(row, "review_stage") == Some(stage))
            .unwrap();
        assert_eq!(json_i32(row, "task_status"), Some(0));
        assert_eq!(
            json_string(row, "scheduled_date").as_deref(),
            Some(expected_date)
        );
        assert!(
            row.get("completed_at")
                .is_some_and(serde_json::Value::is_null)
        );
    }
    for stage in [1, 3] {
        let row = rows
            .iter()
            .find(|row| json_i32(row, "review_stage") == Some(stage))
            .unwrap();
        let original = original_rows
            .iter()
            .find(|row| json_i32(row, "review_stage") == Some(stage))
            .unwrap();
        assert_eq!(json_i32(row, "task_status"), Some(1));
        assert_eq!(
            json_string(row, "scheduled_date"),
            json_string(original, "scheduled_date")
        );
        assert!(json_string(row, "completed_at").is_some());
    }
}

#[tokio::test]
async fn reactivating_fully_completed_abandoned_plan_starts_new_cycle() {
    let db = create_db().await;
    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_1301", "Unit 1.3", TEST_DATE)
        .await
        .unwrap();
    let rows = db
        .query(format!(
            "SELECT id FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage ASC",
            plan.plan_unit_id
        ))
        .await
        .unwrap();
    for row in rows {
        complete_study_task(&db, json_i64(&row, "id").unwrap())
            .await
            .unwrap();
    }
    db.execute(format!(
        "UPDATE study_plan_units SET plan_status = 2 WHERE id = {}",
        plan.plan_unit_id
    ))
    .await
    .unwrap();

    let restarted =
        upsert_study_plan_on_date(&db, "studytestbook", "RE_1301", "Unit 1.3", "2026-08-10")
            .await
            .unwrap();
    assert_eq!(restarted.outcome, StudyPlanUpsertOutcome::Restarted);
    assert!(restarted.completed_stages.is_empty());
    assert_eq!(restarted.next_review_date.as_deref(), Some("2026-08-11"));

    let rows = db
        .query(format!(
            "SELECT task_status, completed_at FROM study_tasks WHERE plan_unit_id = {}",
            plan.plan_unit_id
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 7);
    assert!(
        rows.iter()
            .all(|row| json_i32(row, "task_status") == Some(0))
    );
    assert!(rows.iter().all(|row| {
        row.get("completed_at")
            .is_some_and(serde_json::Value::is_null)
    }));

    let plan_row = db
        .query(format!(
            "SELECT last_review_at FROM study_plan_units WHERE id = {}",
            plan.plan_unit_id
        ))
        .await
        .unwrap();
    assert!(
        plan_row[0]
            .get("last_review_at")
            .is_some_and(serde_json::Value::is_null)
    );
}

#[tokio::test]
async fn mastery_requires_all_stages() {
    let db = create_db().await;

    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_2001", "Unit 2", TEST_DATE)
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

    let status = get_study_plan_status_on_date(&db, "studytestbook", "RE_2001", TEST_DATE)
        .await
        .unwrap();
    assert_eq!(status.plan_status, Some(1));
    assert_eq!(status.completed_stages.len(), 7);

    let retry_result = complete_study_task(&db, stage7_id).await.unwrap();
    assert_eq!(retry_result.task_status, 1);
    assert_eq!(retry_result.plan_status, 1);
    assert_eq!(retry_result.completed_stages.len(), 7);
}

#[tokio::test]
async fn complete_study_task_rejects_pending_task_when_plan_is_mastered() {
    let db = create_db().await;

    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_2501", "Unit 2.5", TEST_DATE)
        .await
        .unwrap();

    let rows = db
            .query(format!(
                "SELECT id, review_stage FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage ASC",
                plan.plan_unit_id
            ))
            .await
            .unwrap();

    let stage1_id = rows
        .iter()
        .find(|row| json_i32(row, "review_stage") == Some(1))
        .and_then(|row| json_i64(row, "id"))
        .unwrap();
    let stage2_id = rows
        .iter()
        .find(|row| json_i32(row, "review_stage") == Some(2))
        .and_then(|row| json_i64(row, "id"))
        .unwrap();

    let _ = complete_study_task(&db, stage1_id).await.unwrap();

    db.execute(format!(
        "UPDATE study_plan_units SET plan_status = 1, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
        plan.plan_unit_id
    ))
    .await
    .unwrap();

    let err = complete_study_task(&db, stage2_id).await.unwrap_err();
    assert_eq!(err, "PLAN_ALREADY_MASTERED");
}

#[tokio::test]
async fn get_tasks_by_date_returns_today_and_overdue_pending_tasks() {
    let db = create_db().await;

    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_3001", "Unit 3", TEST_DATE)
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
    let stage3_id = stage_rows
        .iter()
        .find(|row| json_i32(row, "review_stage") == Some(3))
        .and_then(|row| json_i64(row, "id"))
        .unwrap();

    db.execute(format!(
        "UPDATE study_tasks SET scheduled_date = '2026-08-02' WHERE id = {}",
        stage1_id
    ))
    .await
    .unwrap();
    db.execute(format!(
        "UPDATE study_tasks SET scheduled_date = '2026-08-03' WHERE id = {}",
        stage2_id
    ))
    .await
    .unwrap();
    db.execute(format!(
            "UPDATE study_tasks SET scheduled_date = '2026-08-01', task_status = 1, completed_at = CURRENT_TIMESTAMP WHERE id = {}",
            stage3_id
        ))
        .await
        .unwrap();

    let tasks = get_tasks_by_date_on_date(&db, TEST_DATE, TEST_DATE)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].task_id, stage1_id);
    assert_eq!(tasks[0].review_stage, 1);
    assert!(tasks[0].is_overdue);
    assert_eq!(tasks[1].task_id, stage2_id);
    assert_eq!(tasks[1].review_stage, 2);
    assert!(!tasks[1].is_overdue);
}

#[tokio::test]
async fn get_tasks_by_date_for_non_today_excludes_overdue_tasks() {
    let db = create_db().await;

    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_3201", "Unit 3.2", TEST_DATE)
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
    let stage3_id = stage_rows
        .iter()
        .find(|row| json_i32(row, "review_stage") == Some(3))
        .and_then(|row| json_i64(row, "id"))
        .unwrap();

    db.execute(format!(
        "UPDATE study_tasks SET scheduled_date = '2026-08-02' WHERE id = {}",
        stage1_id
    ))
    .await
    .unwrap();
    db.execute(format!(
        "UPDATE study_tasks SET scheduled_date = '2026-08-04' WHERE id = {}",
        stage2_id
    ))
    .await
    .unwrap();
    db.execute(format!(
            "UPDATE study_tasks SET scheduled_date = '2026-08-04', task_status = 1, completed_at = CURRENT_TIMESTAMP WHERE id = {}",
            stage3_id
        ))
        .await
        .unwrap();

    let tasks = get_tasks_by_date_on_date(&db, "2026-08-04", TEST_DATE)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].task_id, stage2_id);
    assert_eq!(tasks[0].review_stage, 2);
    assert!(!tasks[0].is_overdue);
    assert_eq!(tasks[1].task_id, stage3_id);
    assert_eq!(tasks[1].review_stage, 3);
    assert!(!tasks[1].is_overdue);
}
