//! 学习统计聚合和会话明细查询。

use super::common::*;
use super::types::*;
use super::*;

/// 按显式本地日期查询分页学习统计。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `period_type`：统计周期类型。
/// - `filters`：可选的统计筛选条件。
/// - `page`：可选的分页页码。
/// - `page_size`：可选的每页记录数。
/// - `local_date`：客户端本地日期，格式为 YYYY-MM-DD。
pub async fn get_study_stats_on_date(
    db: &dyn Database,
    period_type: &str,
    filters: Option<StudyStatsFilters>,
    page: Option<i64>,
    page_size: Option<i64>,
    local_date: &str,
) -> Result<StudyStatsResponse, String> {
    let period_type = normalize_period_type(period_type)?;
    let (range_start_expr, range_end_expr) = period_range_expr(period_type, local_date)?;
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * page_size;

    let where_sql = build_common_where_clause(&range_start_expr, &range_end_expr, filters.as_ref());

    let range_sql = format!(
        "SELECT {} AS range_start, {} AS range_end",
        range_start_expr, range_end_expr
    );
    let statements = vec![
        SqlStatement::new(range_sql, vec![]),
        SqlStatement::new(
            format!(
                "SELECT s.local_date AS stat_key, CAST(SUM(s.duration) AS TEXT) AS duration \
                 FROM study_sessions s JOIN books b ON s.book_id = b.id {} \
                 GROUP BY s.local_date ORDER BY s.local_date ASC",
                where_sql
            ),
            vec![],
        ),
        SqlStatement::new(
            format!(
                "SELECT s.book_id AS book_id, b.product_code AS product_code, \
                        b.short_title AS short_title, b.title AS title, \
                        CAST(SUM(s.duration) AS TEXT) AS duration \
                 FROM study_sessions s JOIN books b ON s.book_id = b.id {} \
                 GROUP BY s.book_id, b.product_code, b.short_title, b.title \
                 ORDER BY duration DESC",
                where_sql
            ),
            vec![],
        ),
        SqlStatement::new(
            format!(
                "SELECT b.book_group AS book_group, CAST(SUM(s.duration) AS TEXT) AS duration \
                 FROM study_sessions s JOIN books b ON s.book_id = b.id {} \
                 GROUP BY b.book_group ORDER BY duration DESC, b.book_group ASC",
                where_sql
            ),
            vec![],
        ),
        SqlStatement::new(
            format!(
                "SELECT CAST(COUNT(*) AS TEXT) AS total_recent \
                 FROM study_sessions s JOIN books b ON s.book_id = b.id {}",
                where_sql
            ),
            vec![],
        ),
        SqlStatement::new(
            format!(
                "SELECT s.id AS session_id, s.book_id AS book_id, b.book_group AS book_group, \
                        b.product_code AS product_code, b.short_title AS short_title, b.title AS title, \
                        s.resource_id AS resource_id, s.unit_name AS unit_name, \
                        s.entry_resource_id AS entry_resource_id, s.entry_unit_name AS entry_unit_name, \
                        s.visited_units_json AS visited_units_json, s.start_at AS start_at, \
                        s.end_at AS end_at, s.duration AS duration \
                 FROM study_sessions s JOIN books b ON s.book_id = b.id {} \
                 ORDER BY s.start_at DESC, s.id DESC LIMIT {} OFFSET {}",
                where_sql, page_size, offset
            ),
            vec![],
        ),
    ];
    let mut result_sets = db
        .query_batch(statements)
        .await
        .map_err(|e| e.to_string())?
        .into_iter();
    let range_rows = result_sets.next().unwrap_or_default();
    let range_start = range_rows
        .first()
        .and_then(|row| json_string(row, "range_start"))
        .unwrap_or_default();
    let range_end = range_rows
        .first()
        .and_then(|row| json_string(row, "range_end"))
        .unwrap_or_default();

    // 聚合和分页均在数据库内完成，避免周期较长时把全部会话加载到应用内存。
    let trend_rows = result_sets.next().unwrap_or_default();
    let trend = trend_rows
        .iter()
        .filter_map(|row| {
            Some(StudyStatsTrendItem {
                date: json_string(row, "stat_key")?,
                duration: json_i64(row, "duration")?,
            })
        })
        .collect::<Vec<_>>();

    let book_rows = result_sets.next().unwrap_or_default();
    let mut book_breakdown = book_rows
        .iter()
        .filter_map(|row| {
            Some(StudyStatsBookBreakdownItem {
                book_id: json_i64(row, "book_id")?,
                product_code: json_string(row, "product_code")?,
                book_title: book_display_title(row)?,
                duration: json_i64(row, "duration")?,
            })
        })
        .collect::<Vec<_>>();
    book_breakdown.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| left.book_title.cmp(&right.book_title))
    });

    let series_rows = result_sets.next().unwrap_or_default();
    let series_breakdown = series_rows
        .iter()
        .filter_map(|row| {
            let book_group = json_i32(row, "book_group")?;
            Some(StudyStatsSeriesBreakdownItem {
                book_group,
                series_key: series_key_from_group(book_group),
                duration: json_i64(row, "duration")?,
            })
        })
        .collect::<Vec<_>>();

    let count_rows = result_sets.next().unwrap_or_default();
    let total_recent = count_rows
        .first()
        .and_then(|row| json_i64(row, "total_recent"))
        .unwrap_or(0);

    let recent_rows = result_sets.next().unwrap_or_default();
    let recent_sessions = recent_rows.iter().filter_map(row_to_session).collect();

    Ok(StudyStatsResponse {
        period_type: period_type.to_string(),
        range_start,
        range_end,
        trend,
        book_breakdown,
        series_breakdown,
        recent_sessions,
        page,
        page_size,
        total_recent,
    })
}

/// 查询指定日期和筛选条件下的学习会话。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `date`：需要查询的日期。
/// - `filters`：可选的统计筛选条件。
pub async fn get_study_sessions_by_date(
    db: &dyn Database,
    date: &str,
    filters: Option<StudyStatsFilters>,
) -> Result<Vec<StudySessionListItem>, String> {
    crate::services::study_plan::validate_local_date_for_payload(date)?;
    let mut clauses = vec!["s.local_date = ?".to_string()];
    let mut params = vec![SqlValue::Text(date.to_string())];

    if let Some(filter) = filters {
        if let Some(book_id) = filter.book_id {
            clauses.push("s.book_id = ?".to_string());
            params.push(SqlValue::Integer(book_id));
        }
        if let Some(book_group) = filter.book_group {
            clauses.push("b.book_group = ?".to_string());
            params.push(SqlValue::Integer(i64::from(book_group)));
        }
    }

    let where_sql = format!("WHERE {}", clauses.join(" AND "));

    let sql = format!(
        "SELECT s.id AS session_id, s.book_id AS book_id, b.book_group AS book_group, b.product_code AS product_code, \
                b.short_title AS short_title, b.title AS title, \
                s.resource_id AS resource_id, s.unit_name AS unit_name, s.entry_resource_id AS entry_resource_id, s.entry_unit_name AS entry_unit_name, \
                s.visited_units_json AS visited_units_json, s.start_at AS start_at, s.end_at AS end_at, s.duration AS duration \
         FROM study_sessions s \
         JOIN books b ON s.book_id = b.id \
         {} \
         ORDER BY s.start_at DESC, s.id DESC",
        where_sql
    );

    let rows = db
        .query_statement(SqlStatement::new(sql, params))
        .await
        .map_err(|e| e.to_string())?;

    let sessions = rows.iter().filter_map(row_to_session).collect();

    Ok(sessions)
}

#[cfg(test)]
mod request_count_tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct BatchOnlyDatabase {
        batch_calls: AtomicUsize,
    }

    impl Database for BatchOnlyDatabase {
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

        fn query_batch(
            &self,
            _statements: Vec<SqlStatement>,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Vec<Value>>>> + Send + '_>> {
            self.batch_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(vec![
                    vec![serde_json::json!({
                        "range_start": "2026-08-03",
                        "range_end": "2026-08-09"
                    })],
                    vec![],
                    vec![],
                    vec![],
                    vec![serde_json::json!({ "total_recent": "0" })],
                    vec![],
                ])
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
    async fn explicit_date_stats_use_one_read_batch() {
        let database = BatchOnlyDatabase {
            batch_calls: AtomicUsize::new(0),
        };

        get_study_stats_on_date(&database, "week", None, None, None, "2026-08-03")
            .await
            .unwrap();

        assert_eq!(database.batch_calls.load(Ordering::SeqCst), 1);
    }
}
