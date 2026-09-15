//! 学习计划服务回归测试。

use super::common::{get_completed_stages, json_i32, json_i64, json_string, validate_local_date};
use super::*;
use crate::database::{Database, SqliteDatabase, migrate_up};
use std::sync::Arc;

const TEST_DATE: &str = "2026-08-03";

#[tokio::test]
async fn arrangement_duration_counts_assigned_history_once_and_tracks_changes() {
    let db = create_db().await;
    for resource in ["RE_1", "RE_2"] {
        upsert_study_plan_on_date(&db, "studytestbook", resource, "Same title", TEST_DATE)
            .await
            .unwrap();
    }
    db.execute("UPDATE books SET short_title = '简称' WHERE id = 2001".into())
        .await
        .unwrap();
    db.execute("INSERT INTO study_sessions (id, book_id, resource_id, unit_name, entry_resource_id, entry_unit_name, start_at, end_at, duration, local_date, timezone_offset_minutes) VALUES (9001, 2001, 'RE_1', 'Old title', 'RE_2', 'Entry', '2020-01-01', '2020-01-03', 90000, '2020-01-01', 0), (9002, 2001, 'RE_1', 'Same title', 'RE_1', 'Entry', '2020-01-01', '2020-01-03', 61, '2020-01-01', 0), (9003, 1, 'RE_1', 'Same title', 'RE_1', 'Entry', '2020-01-01', '2020-01-03', 123, '2020-01-01', 0)".into()).await.unwrap();
    let mut query = arrangement_query("all", 1);
    query.page_size = 10;
    let result = get_study_arrangements(&db, query).await.unwrap();
    assert_eq!(result.items[0].total_duration_seconds, 90061);
    assert_eq!(result.items[0].book_short_title.as_deref(), Some("简称"));
    assert_eq!(result.items[1].total_duration_seconds, 0);
    // 修改时长与归属后，下一次查询直接反映结果，不读取计时入口单元。
    db.execute(
        "UPDATE study_sessions SET duration = 3600, resource_id = 'RE_2' WHERE id = 9001".into(),
    )
    .await
    .unwrap();
    assert_eq!(
        get_study_arrangements(&db, arrangement_query("all", 1))
            .await
            .unwrap()
            .items[0]
            .total_duration_seconds,
        61
    );
    assert_eq!(
        get_study_arrangements(&db, arrangement_query("all", 2))
            .await
            .unwrap()
            .items[0]
            .total_duration_seconds,
        3600
    );
    db.execute("DELETE FROM study_sessions WHERE id = 9002".into())
        .await
        .unwrap();
    assert_eq!(
        get_study_arrangements(&db, arrangement_query("all", 1))
            .await
            .unwrap()
            .items[0]
            .total_duration_seconds,
        0
    );
}

/// 构造学习安排筛选参数；status 为状态，page 为页码。
fn arrangement_query(status: &str, page: usize) -> StudyArrangementQuery {
    StudyArrangementQuery {
        book_id: Some(2001),
        book_group: None,
        status: Some(status.into()),
        page,
        page_size: 1,
    }
}

#[tokio::test]
async fn arrangements_series_filter_applies_to_rows() {
    let db = create_db().await;
    upsert_study_plan_on_date(&db, "studytestbook", "RE_1", "Unit 1", TEST_DATE)
        .await
        .unwrap();
    let mut matching = arrangement_query("all", 1);
    matching.book_group = Some(1);
    let matching = get_study_arrangements(&db, matching).await.unwrap();
    assert_eq!(matching.total, 1);
    // 既有测试书属于系列 1；不匹配的系列应清空列表。
    let mut query = arrangement_query("all", 1);
    query.book_group = Some(2);
    let result = get_study_arrangements(&db, query).await.unwrap();
    assert_eq!(result.total, 0);
}

#[tokio::test]
async fn arrangements_filter_count_and_sort_by_unit_before_paging() {
    let db = create_db().await;
    for (resource, title) in [
        ("RE_10", "Ten"),
        ("RE_2", "Two"),
        ("RE_3", "Unit 3 Three"),
        ("RE_4", "Four"),
    ] {
        upsert_study_plan_on_date(&db, "studytestbook", resource, title, TEST_DATE)
            .await
            .unwrap();
    }
    db.execute("UPDATE study_tasks SET task_status = 1 WHERE review_stage = 1 AND plan_unit_id IN (SELECT id FROM study_plan_units WHERE resource_id = 'RE_2')".into()).await.unwrap();
    db.execute("UPDATE study_plan_units SET plan_status = 1 WHERE resource_id = 'RE_3'".into())
        .await
        .unwrap();
    abandon_study_plan(&db, "studytestbook", "RE_4")
        .await
        .unwrap();
    let first = get_study_arrangements(&db, arrangement_query("all", 1))
        .await
        .unwrap();
    assert_eq!(first.total, 3);
    assert_eq!(first.items[0].resource_id, "RE_2");
    assert_eq!(first.items[0].status, "active");
    assert_eq!(first.items[0].completed_count, 1);
    let second = get_study_arrangements(&db, arrangement_query("all", 2))
        .await
        .unwrap();
    assert_eq!(second.items[0].resource_id, "RE_3");
    assert_eq!(second.items[0].next_review_date, None);
    let scheduled = get_study_arrangements(&db, arrangement_query("scheduled", 9))
        .await
        .unwrap();
    assert_eq!((scheduled.total, scheduled.page), (1, 1));
    assert_eq!(scheduled.items[0].resource_id, "RE_10");
    let mut other_book = arrangement_query("all", 1);
    other_book.book_id = Some(999999);
    let empty = get_study_arrangements(&db, other_book).await.unwrap();
    assert_eq!(empty.total, 0);
    assert!(
        get_study_arrangements(&db, arrangement_query("abandoned", 1))
            .await
            .is_err()
    );
    assert!(
        get_study_arrangements(&db, arrangement_query("all", 0))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn arrangement_reviews_show_only_existing_dynamic_history() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    complete_assessed_task(&db, fifth, TEST_DATE, Some("good"), None)
        .await
        .unwrap();
    let reviews = get_study_arrangement_reviews(&db, plan).await.unwrap();
    assert_eq!(reviews.len(), 6);
    assert_eq!(
        reviews.iter().map(|r| r.review_stage).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert!(
        reviews[..5]
            .iter()
            .all(|r| r.task_status == 1 && r.completed_at.is_some())
    );
    assert_eq!(reviews[5].scheduled_date, "2026-08-17");
    assert_eq!(reviews[5].task_status, 0);
    abandon_study_plan(&db, "studytestbook", "adaptive")
        .await
        .unwrap();
    assert_eq!(
        get_study_arrangement_reviews(&db, plan).await.unwrap_err(),
        "PLAN_NOT_FOUND"
    );
    assert!(get_study_arrangement_reviews(&db, -1).await.is_err());
}

/// 只允许一次参数化读取，检测预览是否增加数据库往返。
struct SingleReadDatabase<'a>(&'a SqliteDatabase, std::sync::atomic::AtomicUsize);

impl Database for SingleReadDatabase<'_> {
    fn query_statement(
        &self,
        statement: crate::database::SqlStatement,
    ) -> crate::database::DatabaseFuture<'_, Vec<serde_json::Value>> {
        assert_eq!(self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0);
        self.0.query_statement(statement)
    }
    fn query(&self, _: String) -> crate::database::DatabaseFuture<'_, Vec<serde_json::Value>> {
        panic!("不应额外查询")
    }
    fn execute(&self, _: String) -> crate::database::DatabaseFuture<'_, ()> {
        panic!("预览不应写入")
    }
    fn query_write_batch(
        &self,
        _: Vec<crate::database::SqlStatement>,
    ) -> crate::database::DatabaseFuture<'_, Vec<Vec<serde_json::Value>>> {
        panic!("预览不应写入")
    }
    fn get_version(&self) -> crate::database::DatabaseFuture<'_, String> {
        panic!("不应额外查询版本")
    }
}

#[tokio::test]
async fn assessment_context_uses_single_read_and_preserves_history() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    let counted = SingleReadDatabase(&db, std::sync::atomic::AtomicUsize::new(0));
    let context = super::assessment_rules::load_context(&counted, fifth)
        .await
        .unwrap();
    assert_eq!(context.completed, vec![1, 2, 3, 4]);
    assert_eq!(counted.1.load(std::sync::atomic::Ordering::SeqCst), 1);
    complete_assessed_task(&db, fifth, TEST_DATE, Some("good"), None)
        .await
        .unwrap();
    let pending = next_pending(&db, plan).await;
    let counted = SingleReadDatabase(&db, std::sync::atomic::AtomicUsize::new(0));
    let preview = get_assessment_preview(&counted, json_i64(&pending, "id").unwrap(), "2026-08-04")
        .await
        .unwrap();
    assert_eq!(preview.options[2].interval_days, 21);
    assert_eq!(counted.1.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn preview_matches_saved_interval_and_rejects_stale_date() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    let preview = get_assessment_preview(&db, fifth, TEST_DATE).await.unwrap();
    let error = complete_assessed_task_with_preview(
        &db,
        fifth,
        "2026-08-04",
        Some("hard"),
        None,
        Some(&preview.revision),
    )
    .await
    .unwrap_err();
    assert_eq!(error, "ASSESSMENT_PREVIEW_CHANGED");
    assert_eq!(get_completed_stages(&db, plan).await.unwrap().len(), 4);
    complete_assessed_task_with_preview(
        &db,
        fifth,
        TEST_DATE,
        Some("hard"),
        None,
        Some(&preview.revision),
    )
    .await
    .unwrap();
    let pending = next_pending(&db, plan).await;
    assert_eq!(
        json_string(&pending, "scheduled_date").unwrap(),
        preview.options[1].scheduled_date
    );
    let next = json_i64(&pending, "id").unwrap();
    let preview = get_assessment_preview(&db, next, "2026-09-01")
        .await
        .unwrap();
    assert_eq!(preview.options[2].interval_days, 5);
    complete_assessed_task_with_preview(
        &db,
        next,
        "2026-09-01",
        Some("good"),
        None,
        Some(&preview.revision),
    )
    .await
    .unwrap();
    assert_eq!(
        json_string(&next_pending(&db, plan).await, "scheduled_date").as_deref(),
        Some("2026-09-06")
    );
    let row = db
        .query(format!(
            "SELECT next_interval_days, assessment_local_date FROM study_tasks WHERE id = {next}"
        ))
        .await
        .unwrap();
    assert_eq!(json_i32(&row[0], "next_interval_days"), Some(5));
    assert_eq!(
        json_string(&row[0], "assessment_local_date").as_deref(),
        Some("2026-09-01")
    );
}

#[tokio::test]
async fn repeated_same_day_assessments_do_not_finish_plan() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    complete_assessed_task(&db, fifth, TEST_DATE, Some("mastered"), None)
        .await
        .unwrap();
    for _ in 0..4 {
        let next = json_i64(&next_pending(&db, plan).await, "id").unwrap();
        let preview = get_assessment_preview(&db, next, TEST_DATE).await.unwrap();
        assert!(preview.same_day);
        assert_eq!(preview.options[3].mastered_streak, 1);
        assert!(!preview.options[3].requires_finish_decision);
        assert_eq!(
            complete_assessed_task(&db, next, TEST_DATE, Some("mastered"), Some(true))
                .await
                .unwrap_err(),
            "FINISH_NOT_ALLOWED"
        );
        complete_assessed_task(&db, next, TEST_DATE, Some("mastered"), None)
            .await
            .unwrap();
    }
    assert_eq!(
        json_string(&next_pending(&db, plan).await, "scheduled_date").as_deref(),
        Some("2026-09-02")
    );
}

#[tokio::test]
async fn interval_migration_preserves_old_assessment_and_dates() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    crate::database::migrate_down(&db, Some("0.9.0"))
        .await
        .unwrap();
    db.execute(format!("UPDATE study_tasks SET task_status = 1, mastery_rating = 'mastered', mastered_streak = 3, completed_at = '2026-08-03 10:00:00' WHERE id = {fifth}")).await.unwrap();
    db.execute(format!("INSERT INTO study_tasks (plan_unit_id, scheduled_date, review_stage) VALUES ({plan}, '2026-12-01', 6)")).await.unwrap();
    // 显式比较旧字段，避免跨 ALTER TABLE 复用 SELECT * 的列元数据缓存。
    let history_query = "SELECT id, plan_unit_id, scheduled_date, review_stage, task_status, completed_at, created_at, updated_at, mastery_rating, mastered_streak FROM study_tasks ORDER BY id";
    let before = db.query(history_query.into()).await.unwrap();
    migrate_up(&db, None).await.unwrap();
    let after = db.query(history_query.into()).await.unwrap();
    assert_eq!(before.len(), after.len());
    for (old, new) in before.iter().zip(after.iter()) {
        for (key, value) in old.as_object().unwrap() {
            assert_eq!(new.get(key), Some(value));
        }
    }
    let pending = next_pending(&db, plan).await;
    let id = json_i64(&pending, "id").unwrap();
    let preview = get_assessment_preview(&db, id, "2026-12-01").await.unwrap();
    assert!(preview.legacy_date);
    assert_eq!(preview.options[3].mastered_streak, 3);
    assert!(!preview.options[3].requires_finish_decision);
    complete_assessed_task(&db, id, "2026-12-01", Some("mastered"), None)
        .await
        .unwrap();
    let next = json_i64(&next_pending(&db, plan).await, "id").unwrap();
    assert!(
        get_assessment_preview(&db, next, "2027-03-31")
            .await
            .unwrap()
            .options[3]
            .requires_finish_decision
    );
    assert!(
        crate::database::migrate_down(&db, Some("0.9.0"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn assessment_counts_actual_completions_not_stage_number() {
    let db = create_db().await;
    let plan = upsert_study_plan_on_date(&db, "studytestbook", "unordered", "顺序测试", TEST_DATE)
        .await
        .unwrap();
    let rows = db
        .query(format!(
            "SELECT id FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage",
            plan.plan_unit_id
        ))
        .await
        .unwrap();
    for index in [4, 0, 1, 2] {
        complete_assessed_task(
            &db,
            json_i64(&rows[index], "id").unwrap(),
            TEST_DATE,
            None,
            None,
        )
        .await
        .unwrap();
    }
    complete_assessed_task(
        &db,
        json_i64(&rows[3], "id").unwrap(),
        TEST_DATE,
        Some("mastered"),
        None,
    )
    .await
    .unwrap();
    let pending = next_pending(&db, plan.plan_unit_id).await;
    complete_assessed_task(
        &db,
        json_i64(&pending, "id").unwrap(),
        "2026-09-08",
        Some("mastered"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        json_string(
            &next_pending(&db, plan.plan_unit_id).await,
            "scheduled_date"
        )
        .as_deref(),
        Some("2026-11-07")
    );
}

#[tokio::test]
async fn migration_preserves_legacy_tasks_and_prevents_lossy_downgrade() {
    let db = create_db().await;
    crate::database::migrate_down(&db, Some("0.8.0"))
        .await
        .unwrap();
    db.execute("INSERT INTO study_plan_units (id, book_id, resource_id, unit_name) VALUES (900, 2001, 'legacy', '旧计划')".into()).await.unwrap();
    for stage in 1..=7 {
        db.execute(format!("INSERT INTO study_tasks (plan_unit_id, scheduled_date, review_stage, task_status, completed_at) VALUES (900, '2026-08-01', {stage}, {}, {})", if stage <= 5 { 1 } else { 0 }, if stage <= 5 { "'2026-08-01 12:00:00'" } else { "NULL" })).await.unwrap();
    }
    let before = db
        .query(
            "SELECT id, scheduled_date, task_status, completed_at FROM study_tasks ORDER BY id"
                .into(),
        )
        .await
        .unwrap();
    migrate_up(&db, None).await.unwrap();
    let after = db
        .query(
            "SELECT id, scheduled_date, task_status, completed_at FROM study_tasks ORDER BY id"
                .into(),
        )
        .await
        .unwrap();
    assert_eq!(before, after);
    let tasks = get_tasks_by_date_on_date(&db, "2026-09-08", "2026-09-08")
        .await
        .unwrap();
    assert_eq!(tasks.len(), 2);
    assert!(
        tasks
            .iter()
            .all(|t| t.assessment_required && t.mastered_streak == 0)
    );
    complete_assessed_task(&db, tasks[0].task_id, "2026-09-08", Some("forgotten"), None)
        .await
        .unwrap();
    assert_eq!(
        json_string(&next_pending(&db, 900).await, "scheduled_date").as_deref(),
        Some("2026-09-09")
    );
    assert!(
        crate::database::migrate_down(&db, Some("0.8.0"))
            .await
            .is_err()
    );
    assert_eq!(db.get_version().await.unwrap(), "0.10.0");
    assert_eq!(get_completed_stages(&db, 900).await.unwrap().len(), 6);
}

/// 创建完成前四次任务的计划，返回第五次任务和计划标识。
async fn assessment_ready_plan(db: &SqliteDatabase) -> (i64, i64) {
    let plan = upsert_study_plan_on_date(db, "studytestbook", "adaptive", "动态单元", TEST_DATE)
        .await
        .unwrap();
    let rows = db
        .query(format!(
            "SELECT id FROM study_tasks WHERE plan_unit_id = {} ORDER BY review_stage",
            plan.plan_unit_id
        ))
        .await
        .unwrap();
    for row in &rows[..4] {
        complete_assessed_task(db, json_i64(row, "id").unwrap(), TEST_DATE, None, None)
            .await
            .unwrap();
    }
    (json_i64(&rows[4], "id").unwrap(), plan.plan_unit_id)
}

/// 查询动态计划唯一的待办任务。
async fn next_pending(db: &SqliteDatabase, plan: i64) -> Value {
    let rows = db
        .query(format!(
            "SELECT * FROM study_tasks WHERE plan_unit_id = {plan} AND task_status = 0"
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    rows[0].clone()
}

#[tokio::test]
async fn assessment_requires_fifth_rating_and_three_consolidations_before_finish() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    assert_eq!(
        complete_assessed_task(&db, fifth, TEST_DATE, None, None)
            .await
            .unwrap_err(),
        "ASSESSMENT_REQUIRED"
    );
    // 兼容旧计划：遗留的第六、七次待办应被新的安排替换。
    db.execute(format!("INSERT INTO study_tasks (plan_unit_id, scheduled_date, review_stage) VALUES ({plan}, '2026-09-01', 6), ({plan}, '2026-10-01', 7)")).await.unwrap();
    let mut task = fifth;
    for (date, expected) in [
        ("2026-09-08", "2026-10-08"),
        ("2026-10-08", "2026-12-07"),
        ("2026-12-07", "2027-04-06"),
    ] {
        let response = complete_assessed_task(&db, task, date, Some("mastered"), None)
            .await
            .unwrap();
        assert_eq!(response.plan_status, 0);
        let pending = next_pending(&db, plan).await;
        assert_eq!(json_string(&pending, "scheduled_date").unwrap(), expected);
        // 重复提交已完成任务不会生成第二个待办。
        complete_assessed_task(&db, task, date, Some("mastered"), None)
            .await
            .unwrap();
        assert_eq!(
            json_i64(&next_pending(&db, plan).await, "id"),
            json_i64(&pending, "id")
        );
        task = json_i64(&pending, "id").unwrap();
    }
    assert_eq!(
        complete_assessed_task(&db, task, "2027-04-06", Some("mastered"), None)
            .await
            .unwrap_err(),
        "FINISH_DECISION_REQUIRED"
    );
    complete_assessed_task(&db, task, "2027-04-06", Some("mastered"), Some(false))
        .await
        .unwrap();
    let pending = next_pending(&db, plan).await;
    assert_eq!(
        json_string(&pending, "scheduled_date").as_deref(),
        Some("2027-08-04")
    );
    let result = complete_assessed_task(
        &db,
        json_i64(&pending, "id").unwrap(),
        "2027-08-04",
        Some("mastered"),
        Some(true),
    )
    .await
    .unwrap();
    assert_eq!(result.plan_status, 1);
    assert_eq!(result.completed_stages.len(), 9);
    assert!(
        db.query(format!(
            "SELECT id FROM study_tasks WHERE plan_unit_id = {plan} AND task_status = 0"
        ))
        .await
        .unwrap()
        .is_empty()
    );
}

#[tokio::test]
async fn lower_rating_resets_consolidation_and_reactivation_preserves_history() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    complete_assessed_task(&db, fifth, TEST_DATE, Some("mastered"), None)
        .await
        .unwrap();
    let pending = next_pending(&db, plan).await;
    let id = json_i64(&pending, "id").unwrap();
    complete_assessed_task(&db, id, "2026-09-10", Some("hard"), None)
        .await
        .unwrap();
    let pending = next_pending(&db, plan).await;
    assert_eq!(
        json_string(&pending, "scheduled_date").as_deref(),
        Some("2026-09-17")
    );
    complete_assessed_task(
        &db,
        json_i64(&pending, "id").unwrap(),
        "2026-09-13",
        Some("mastered"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        json_string(&next_pending(&db, plan).await, "scheduled_date").as_deref(),
        Some("2026-10-13")
    );
    abandon_study_plan(&db, "studytestbook", "adaptive")
        .await
        .unwrap();
    let resumed =
        upsert_study_plan_on_date(&db, "studytestbook", "adaptive", "动态单元", "2026-10-20")
            .await
            .unwrap();
    assert_eq!(resumed.completed_stages.len(), 7);
    assert_eq!(resumed.next_review_date.as_deref(), Some("2026-10-21"));
    next_pending(&db, plan).await;
}

#[tokio::test]
async fn simultaneous_completion_creates_one_followup() {
    let db = create_db().await;
    let (fifth, plan) = assessment_ready_plan(&db).await;
    let (a, b) = tokio::join!(
        complete_assessed_task(&db, fifth, TEST_DATE, Some("good"), None),
        complete_assessed_task(&db, fifth, TEST_DATE, Some("good"), None)
    );
    assert!(a.is_ok() || b.is_ok());
    let pending = next_pending(&db, plan).await;
    assert_eq!(
        json_string(&pending, "scheduled_date").as_deref(),
        Some("2026-08-17")
    );
}

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
    assert_eq!(task_rows.len(), 5);
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
    assert_eq!(json_i64(&rows[0], "count"), Some(5));
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
        complete_assessed_task(&db, task_id, TEST_DATE, None, None)
            .await
            .unwrap();
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
    let expected_pending = [(2, "2026-08-11"), (4, "2026-08-12"), (5, "2026-08-14")];
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

    let _ = complete_assessed_task(&db, stage1_id, TEST_DATE, None, None)
        .await
        .unwrap();

    db.execute(format!(
        "UPDATE study_plan_units SET plan_status = 1, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
        plan.plan_unit_id
    ))
    .await
    .unwrap();

    let err = complete_assessed_task(&db, stage2_id, TEST_DATE, None, None)
        .await
        .unwrap_err();
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

#[tokio::test]
async fn shifts_all_pending_tasks_and_keeps_completed_task_date() {
    let db = create_db().await;
    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_4001", "Unit 4", TEST_DATE)
        .await
        .unwrap();

    db.execute(format!(
        "UPDATE study_tasks SET task_status = 1, completed_at = CURRENT_TIMESTAMP \
         WHERE plan_unit_id = {} AND review_stage = 2",
        plan.plan_unit_id
    ))
    .await
    .unwrap();

    let result = shift_study_plan_on_date(&db, plan.plan_unit_id, 2, TEST_DATE)
        .await
        .unwrap();
    assert_eq!(result.first_stage_date, "2026-08-06");
    assert_eq!(result.affected_tasks, 4);

    let rows = db
        .query(format!(
            "SELECT review_stage, scheduled_date FROM study_tasks \
             WHERE plan_unit_id = {} ORDER BY review_stage",
            plan.plan_unit_id
        ))
        .await
        .unwrap();
    assert_eq!(
        json_string(&rows[0], "scheduled_date").as_deref(),
        Some("2026-08-06")
    );
    assert_eq!(
        json_string(&rows[1], "scheduled_date").as_deref(),
        Some("2026-08-05")
    );
    assert_eq!(
        json_string(&rows[2], "scheduled_date").as_deref(),
        Some("2026-08-09")
    );
}

#[tokio::test]
async fn rejects_shift_before_today_or_after_first_stage_completion() {
    let db = create_db().await;
    let plan = upsert_study_plan_on_date(&db, "studytestbook", "RE_4002", "Unit 4.2", TEST_DATE)
        .await
        .unwrap();

    let early_error = shift_study_plan_on_date(&db, plan.plan_unit_id, -2, TEST_DATE)
        .await
        .unwrap_err();
    assert_eq!(early_error, "FIRST_STAGE_BEFORE_TODAY");

    db.execute(format!(
        "UPDATE study_tasks SET task_status = 1, completed_at = CURRENT_TIMESTAMP \
         WHERE plan_unit_id = {} AND review_stage = 1",
        plan.plan_unit_id
    ))
    .await
    .unwrap();
    let completed_error = shift_study_plan_on_date(&db, plan.plan_unit_id, 1, TEST_DATE)
        .await
        .unwrap_err();
    assert_eq!(completed_error, "FIRST_STAGE_COMPLETED");
}
