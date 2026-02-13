use crate::models::{Book, BookGroup};
use log::info;
use tauri::State;

#[tauri::command]
pub async fn get_books(
    state: State<'_, crate::database::DbState>,
    group: Option<BookGroup>,
) -> Result<Vec<Book>, String> {
    let db_guard = state.db.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    get_books_logic(db.as_ref(), group).await
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
