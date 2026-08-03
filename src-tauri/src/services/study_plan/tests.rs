//! 学习计划服务回归测试。

use super::common::{json_i32, json_i64, validate_local_date};
use super::*;
use crate::database::{Database, SqliteDatabase, migrate_up};

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
    let second = upsert_study_plan_on_date(&db, "studytestbook", "RE_1001", "Unit 1", TEST_DATE)
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
