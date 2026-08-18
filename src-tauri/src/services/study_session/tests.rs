//! 学习会话服务回归测试。

use super::*;
use crate::database::{Database, SqliteDatabase};

async fn setup_test_db() -> SqliteDatabase {
    let db_path =
        std::env::temp_dir().join(format!("study_session_test_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(db_path.to_str().unwrap())
        .await
        .unwrap();

    crate::database::migrate_up(&db, None).await.unwrap();

    db.execute("INSERT INTO books (id, book_group, product_code, title, short_title, author, product_type, sort_num) VALUES (3001, 1, 'timerbooka', 'Timer Book A', 'Timer A', NULL, 'imgbook', 1)".to_string()).await.unwrap();
    db.execute("INSERT INTO books (id, book_group, product_code, title, short_title, author, product_type, sort_num) VALUES (3002, 2, 'timerbookb', 'Timer Book B', '\t　', NULL, 'imgbook', 2)".to_string()).await.unwrap();

    db
}

async fn current_db_datetime(db: &dyn Database, modifier: &str) -> String {
    let rows = db
        .query(format!(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '{}') AS now_at",
            modifier
        ))
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
    let start_at = current_db_datetime(&db, "-20 minutes").await;
    let end_at = current_db_datetime(&db, "0 seconds").await;
    let local_date = end_at[..10].to_string();

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
        start_at,
        end_at,
        duration: 1200,
        local_date: local_date.clone(),
        timezone_offset_minutes: 0,
    };

    let save_result = save_study_session(&db, payload).await.unwrap();
    assert!(save_result.success);
    assert!(save_result.id > 0);

    let stats = get_study_stats_on_date(&db, "week", None, Some(1), Some(20), &local_date)
        .await
        .unwrap();

    assert_eq!(stats.period_type, "week");
    assert!(!stats.book_breakdown.is_empty());
    assert!(!stats.recent_sessions.is_empty());
    assert_eq!(stats.book_breakdown[0].book_title, "Timer A");
    assert_eq!(stats.recent_sessions[0].book_title, "Timer A");
    assert_eq!(stats.recent_sessions[0].resource_id, "RE_U1");
    assert_eq!(stats.recent_sessions[0].duration, 1200);

    let sessions = get_study_sessions_by_date(&db, &local_date, None)
        .await
        .unwrap();
    assert_eq!(sessions[0].book_title, "Timer A");
}

#[tokio::test]
async fn filters_stats_by_book_group() {
    let db = setup_test_db().await;
    let start_at = current_db_datetime(&db, "-20 minutes").await;
    let end_at = current_db_datetime(&db, "0 seconds").await;
    let local_date = end_at[..10].to_string();

    let payload_a = SaveStudySessionPayload {
        product_code: "timerbooka".to_string(),
        entry_resource_id: "RE_A".to_string(),
        entry_unit_name: "Unit A".to_string(),
        assigned_resource_id: "RE_A".to_string(),
        assigned_unit_name: "Unit A".to_string(),
        visited_units: vec![],
        start_at: start_at.clone(),
        end_at: end_at.clone(),
        duration: 600,
        local_date: local_date.clone(),
        timezone_offset_minutes: 0,
    };

    let payload_b = SaveStudySessionPayload {
        product_code: "timerbookb".to_string(),
        entry_resource_id: "RE_B".to_string(),
        entry_unit_name: "Unit B".to_string(),
        assigned_resource_id: "RE_B".to_string(),
        assigned_unit_name: "Unit B".to_string(),
        visited_units: vec![],
        start_at,
        end_at,
        duration: 1200,
        local_date: local_date.clone(),
        timezone_offset_minutes: 0,
    };

    save_study_session(&db, payload_a).await.unwrap();
    save_study_session(&db, payload_b).await.unwrap();

    let filtered = get_study_stats_on_date(
        &db,
        "week",
        Some(StudyStatsFilters {
            book_id: None,
            book_group: Some(2),
        }),
        Some(1),
        Some(20),
        &local_date,
    )
    .await
    .unwrap();

    assert_eq!(filtered.book_breakdown.len(), 1);
    assert_eq!(filtered.book_breakdown[0].product_code, "timerbookb");
    assert_eq!(filtered.book_breakdown[0].book_title, "Timer Book B");
    assert_eq!(filtered.recent_sessions[0].book_title, "Timer Book B");
    assert_eq!(filtered.series_breakdown.len(), 1);
    assert_eq!(filtered.series_breakdown[0].book_group, 2);
}

#[tokio::test]
async fn week_stats_use_monday_to_sunday_and_exclude_adjacent_weeks() {
    let db = setup_test_db().await;
    let session_dates = ["2026-08-09", "2026-08-10", "2026-08-16", "2026-08-17"];

    for (index, local_date) in session_dates.iter().enumerate() {
        save_study_session(
            &db,
            SaveStudySessionPayload {
                product_code: "timerbooka".to_string(),
                entry_resource_id: format!("RE_{index}"),
                entry_unit_name: format!("Unit {index}"),
                assigned_resource_id: format!("RE_{index}"),
                assigned_unit_name: format!("Unit {index}"),
                visited_units: vec![],
                start_at: format!("{local_date}T10:00:00Z"),
                end_at: format!("{local_date}T10:10:00Z"),
                duration: 600,
                local_date: (*local_date).to_string(),
                timezone_offset_minutes: 0,
            },
        )
        .await
        .unwrap();
    }

    for local_date in ["2026-08-10", "2026-08-11", "2026-08-16"] {
        let stats = get_study_stats_on_date(&db, "week", None, Some(1), Some(20), local_date)
            .await
            .unwrap();

        assert_eq!(stats.range_start, "2026-08-10");
        assert_eq!(stats.range_end, "2026-08-16");
        assert_eq!(stats.total_recent, 2);
        assert_eq!(
            stats
                .trend
                .iter()
                .map(|item| item.date.as_str())
                .collect::<Vec<_>>(),
            vec!["2026-08-10", "2026-08-16"]
        );
    }
}
