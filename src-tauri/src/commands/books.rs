use crate::models::{Book, BookGroup, BookSource};
use crate::utils::cache::CacheKey;
use log::info;
use tauri::{AppHandle, State};

pub struct BookCacheState {
    pub cache: moka::future::Cache<String, Vec<Book>>,
}

#[tauri::command]
pub async fn get_books(
    state: State<'_, crate::database::DbState>,
    cache_state: State<'_, BookCacheState>,
    group: Option<BookGroup>,
) -> Result<Vec<Book>, String> {
    let key = CacheKey::book_list(group);

    // 尝试从缓存中获取
    if let Some(books) = cache_state.cache.get(&key).await {
        info!("从缓存获取书籍列表 (key: {})", key);
        return Ok(books);
    }

    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let books = get_books_logic(db.as_ref(), group).await?;

    // 存入缓存
    cache_state.cache.insert(key, books.clone()).await;

    Ok(books)
}

#[tauri::command]
pub async fn get_book_cover(app: AppHandle, book: Book) -> Result<Vec<u8>, String> {
    let config = crate::services::config::load(&app);
    let source = config.book_source.ok_or("Book source not configured")?;

    let cover_name = book.cover.as_ref().ok_or("Book cover not defined")?;
    let relative_path = format!("{}/assets/{}", book.product_code, cover_name);

    match source {
        BookSource::Local { path } => {
            info!("正在从本地读取封面: {}/{}", path, relative_path);
            crate::utils::local::read_file(&path, &relative_path).await
        }
        BookSource::CloudflareR2 { .. } => {
            let client = crate::utils::r2::create_r2_client(&source).await?;
            let bucket = match &source {
                BookSource::CloudflareR2 { bucket_name, .. } => bucket_name,
                _ => unreachable!(),
            };

            info!(
                "正在从 R2 读取封面: bucket={}, key={}",
                bucket, relative_path
            );
            crate::utils::r2::get_object(&client, bucket, &relative_path).await
        }
    }
}

pub async fn get_books_logic(
    db: &dyn crate::database::Database,
    group: Option<BookGroup>,
) -> Result<Vec<Book>, String> {
    info!("正在获取书籍列表 (group: {:?})...", group);
    let sql = match group {
        Some(g) => format!(
            "SELECT * FROM books WHERE book_group = {} ORDER BY book_group, sort_num ASC",
            g as i32
        ),
        None => "SELECT * FROM books ORDER BY book_group, sort_num ASC".to_string(),
    };

    let rows = db.query(sql).await.map_err(|e| e.to_string())?;
    let books = rows.into_iter().filter_map(Book::from_json).collect();

    Ok(books)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Database, SqliteDatabase};
    use crate::models::BookGroup;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_get_books_logic() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let db = SqliteDatabase::new(&path).await.unwrap();

        db.execute("CREATE TABLE books (id INTEGER PRIMARY KEY AUTOINCREMENT, book_group INTEGER NOT NULL, product_code VARCHAR(60) NOT NULL, title NVARCHAR(100) NOT NULL, author VARCHAR(60), product_type VARCHAR(50) NOT NULL, cover TEXT, sort_num INTEGER NOT NULL)".to_string()).await.unwrap();

        db.execute("INSERT INTO books (book_group, product_code, title, author, product_type, sort_num) VALUES (1, 'book1', 'Title 1', NULL, 'imgbook', 2)".to_string()).await.unwrap();
        db.execute("INSERT INTO books (book_group, product_code, title, author, product_type, sort_num) VALUES (1, 'book2', 'Title 2', NULL, 'imgbook', 4)".to_string()).await.unwrap();
        db.execute("INSERT INTO books (book_group, product_code, title, author, product_type, sort_num) VALUES (2, 'book3', 'Title 3', NULL, 'imgbook', 1)".to_string()).await.unwrap();

        let books = get_books_logic(&db, None).await.unwrap();
        assert_eq!(books.len(), 3);
        assert_eq!(books[0].product_code, "book1");
        assert_eq!(books[1].product_code, "book2");
        assert_eq!(books[2].product_code, "book3");

        let books_v = get_books_logic(&db, Some(BookGroup::Vocabulary))
            .await
            .unwrap();
        assert_eq!(books_v.len(), 2);
        assert_eq!(books_v[0].product_code, "book1");
        assert_eq!(books_v[1].product_code, "book2");
    }
}
