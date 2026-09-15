//! 按单元查询学习安排，只读取现存计划和任务，不生成未来复习。
use super::common::*;
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyArrangementQuery {
    pub book_id: Option<i64>,
    pub book_group: Option<i32>,
    pub status: Option<String>,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyArrangementItem {
    pub plan_unit_id: i64,
    pub book_id: i64,
    pub book_title: String,
    pub book_short_title: Option<String>,
    pub total_duration_seconds: i64,
    pub resource_id: String,
    pub unit_name: String,
    pub status: String,
    pub completed_count: i64,
    pub next_review_date: Option<String>,
    #[serde(skip)]
    book_order: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyArrangementList {
    pub items: Vec<StudyArrangementItem>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyArrangementReview {
    pub task_id: i64,
    pub review_stage: i32,
    pub scheduled_date: String,
    pub task_status: i32,
    pub completed_at: Option<String>,
}

/// 生成与目录编号对应的排序文本；参数为单元标题和资源标识。
fn unit_sort_title(item: &StudyArrangementItem) -> String {
    static NUMBERED_TITLE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)^Unit\s+\d+(?:\s|$|[:：.–—-])").expect("固定单元标题规则有效")
    });
    let title = item.unit_name.trim_start();
    if title.starts_with("Appendix ") || NUMBERED_TITLE.is_match(title) {
        return title.into();
    }
    if let Some(number) = item
        .resource_id
        .strip_prefix("RE_")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n > 0)
    {
        return format!("Unit {number} {title}");
    }
    title.into()
}

/// 将连续数字作为整数片段比较，避免 Unit 10 排在 Unit 2 前；参数为两段标题。
fn natural_compare(left: &str, right: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut a = left.bytes().peekable();
    let mut b = right.bytes().peekable();
    while let (Some(&x), Some(&y)) = (a.peek(), b.peek()) {
        let order = if x.is_ascii_digit() && y.is_ascii_digit() {
            let mut x = String::new();
            let mut y = String::new();
            while a.peek().is_some_and(u8::is_ascii_digit) {
                x.push(a.next().unwrap() as char);
            }
            while b.peek().is_some_and(u8::is_ascii_digit) {
                y.push(b.next().unwrap() as char);
            }
            let x = x.trim_start_matches('0');
            let y = y.trim_start_matches('0');
            x.len().cmp(&y.len()).then_with(|| x.cmp(y))
        } else {
            a.next();
            b.next();
            x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase())
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    a.peek().is_some().cmp(&b.peek().is_some())
}

/// 查询单元列表和书籍范围内的统计；分页在自然排序后进行，避免逐行读取历史。
pub async fn get_study_arrangements(
    db: &dyn Database,
    query: StudyArrangementQuery,
) -> Result<StudyArrangementList, String> {
    let status = query.status.as_deref().unwrap_or("all");
    if !["all", "scheduled", "active", "ended"].contains(&status)
        || query.page == 0
        || !(1..=100).contains(&query.page_size)
        || query.book_id.is_some_and(|id| id <= 0)
        || query.book_group.is_some_and(|group| group <= 0)
    {
        return Err("INVALID_ARRANGEMENT_QUERY".into());
    }
    // 学习时长独立聚合，避免与复习任务的一对多关联重复累计。
    let mut sql = "SELECT u.id, u.book_id, u.resource_id, u.unit_name, u.plan_status, b.title AS book_title, b.short_title AS book_short_title, b.sort_num, COALESCE(s.total_duration_seconds, 0) AS total_duration_seconds, COUNT(CASE WHEN t.task_status = 1 THEN 1 END) AS completed_count, MIN(CASE WHEN t.task_status = 0 THEN t.scheduled_date END) AS next_review_date FROM study_plan_units u JOIN books b ON b.id = u.book_id LEFT JOIN (SELECT book_id, resource_id, SUM(duration) AS total_duration_seconds FROM study_sessions GROUP BY book_id, resource_id) s ON s.book_id = u.book_id AND s.resource_id = u.resource_id LEFT JOIN study_tasks t ON t.plan_unit_id = u.id WHERE u.plan_status IN (0, 1)".to_string();
    let mut params = vec![];
    if let Some(id) = query.book_id {
        sql.push_str(" AND u.book_id = ?");
        params.push(SqlValue::Integer(id));
    }
    if let Some(group) = query.book_group {
        sql.push_str(" AND b.book_group = ?");
        params.push(SqlValue::Integer(i64::from(group)));
    }
    sql.push_str(
        " GROUP BY u.id, u.book_id, u.resource_id, u.unit_name, u.plan_status, b.title, b.short_title, b.sort_num, s.total_duration_seconds",
    );
    let rows = db
        .query_statement(SqlStatement::new(sql, params))
        .await
        .map_err(|e| e.to_string())?;
    let mut items: Vec<_> = rows
        .iter()
        .map(|r| {
            let ended = json_i32(r, "plan_status") == Some(1);
            let completed_count = json_i64(r, "completed_count").unwrap_or(0);
            StudyArrangementItem {
                plan_unit_id: json_i64(r, "id").unwrap_or_default(),
                book_id: json_i64(r, "book_id").unwrap_or_default(),
                book_title: json_string(r, "book_title").unwrap_or_default(),
                book_short_title: json_string(r, "book_short_title"),
                total_duration_seconds: json_i64(r, "total_duration_seconds").unwrap_or(0),
                resource_id: json_string(r, "resource_id").unwrap_or_default(),
                unit_name: json_string(r, "unit_name").unwrap_or_default(),
                status: if ended {
                    "ended"
                } else if completed_count == 0 {
                    "scheduled"
                } else {
                    "active"
                }
                .into(),
                completed_count,
                next_review_date: if ended {
                    None
                } else {
                    json_string(r, "next_review_date")
                },
                book_order: json_i64(r, "sort_num").unwrap_or_default(),
            }
        })
        .collect();
    items.retain(|r| status == "all" || r.status == status);
    items.sort_by(|a, b| {
        a.book_order
            .cmp(&b.book_order)
            .then_with(|| a.book_id.cmp(&b.book_id))
            .then_with(|| natural_compare(&unit_sort_title(a), &unit_sort_title(b)))
            .then_with(|| a.plan_unit_id.cmp(&b.plan_unit_id))
    });
    let total = items.len();
    let page = query
        .page
        .min(total.saturating_sub(1) / query.page_size + 1);
    let items = items
        .into_iter()
        .skip((page - 1) * query.page_size)
        .take(query.page_size)
        .collect();
    Ok(StudyArrangementList {
        items,
        total,
        page,
        page_size: query.page_size,
    })
}

/// 读取单元全部现存复习记录；参数为数据库和计划标识，已放弃计划不可查看。
pub async fn get_study_arrangement_reviews(
    db: &dyn Database,
    plan_unit_id: i64,
) -> Result<Vec<StudyArrangementReview>, String> {
    let rows = db.query_statement(SqlStatement::new(
        "SELECT t.id, t.review_stage, t.scheduled_date, t.task_status, t.completed_at FROM study_plan_units u LEFT JOIN study_tasks t ON t.plan_unit_id = u.id WHERE u.id = ? AND u.plan_status IN (0, 1) ORDER BY t.review_stage, t.id",
        vec![SqlValue::Integer(plan_unit_id)],
    )).await.map_err(|e| e.to_string())?;
    if rows.is_empty() {
        return Err("PLAN_NOT_FOUND".into());
    }
    Ok(rows
        .iter()
        .filter_map(|r| {
            Some(StudyArrangementReview {
                task_id: json_i64(r, "id")?,
                review_stage: json_i32(r, "review_stage")?,
                scheduled_date: json_string(r, "scheduled_date")?,
                task_status: json_i32(r, "task_status")?,
                completed_at: json_string(r, "completed_at"),
            })
        })
        .collect())
}
