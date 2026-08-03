use crate::database::{SqlStatement, SqlValue};
use crate::models::{Book, BookGroup, BookSource, ReadingProgress};
use crate::utils::cache::CacheKey;
use log::info;
use std::future::Future;
use std::path::Path;
use tauri::{AppHandle, Manager, Runtime, State};

mod application;
mod infrastructure;

pub use application::BookMetadataResponse;

pub struct BookCacheState {
    pub cache: moka::future::Cache<String, Vec<Book>>,
}

async fn read_cached_or_fetch_and_store<F, Fut>(
    local_path: &Path,
    fetch_remote: F,
) -> Result<Vec<u8>, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Vec<u8>, String>>,
{
    if crate::utils::cache::is_non_empty_file(local_path).await {
        return tokio::fs::read(local_path)
            .await
            .map_err(|e| format!("读取缓存文件失败 (path: {}): {}", local_path.display(), e));
    }

    let bytes = fetch_remote().await?;

    if let Some(parent) = local_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建缓存目录失败 (path: {}): {}", parent.display(), e))?;
    }
    tokio::fs::write(local_path, &bytes)
        .await
        .map_err(|e| format!("写入缓存文件失败 (path: {}): {}", local_path.display(), e))?;

    Ok(bytes)
}

#[tauri::command]
/// 处理获取图书聚合元数据的 Tauri 命令。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `config_state`：当前应用配置状态。
/// - `product_code`：图书产品码。
pub async fn get_book_metadata<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
) -> Result<BookMetadataResponse, String> {
    application::get_book_metadata(app, config_state, product_code).await
}

#[tauri::command]
/// 处理解析页面图片资源的 Tauri 命令。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `config_state`：当前应用配置状态。
/// - `product_code`：图书产品码。
/// - `page_label`：阅读页面标签。
pub async fn resolve_page_resource<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    page_label: String,
) -> Result<String, String> {
    application::resolve_page_resource(app, config_state, product_code, page_label).await
}

#[tauri::command]
/// 处理解析图书相对资产的 Tauri 命令。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `config_state`：当前应用配置状态。
/// - `product_code`：图书产品码。
/// - `relative_path`：图书目录内的相对资源路径。
pub async fn resolve_book_asset<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    relative_path: String,
) -> Result<String, String> {
    application::resolve_book_asset(app, config_state, product_code, relative_path).await
}

#[tauri::command]
/// 处理解析练习入口资源的 Tauri 命令。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `config_state`：当前应用配置状态。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
pub async fn resolve_exercise_resource<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    resource_id: String,
) -> Result<String, String> {
    application::resolve_exercise_resource(app, config_state, product_code, resource_id).await
}

#[tauri::command]
/// 处理获取已加工练习 HTML 的 Tauri 命令。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `config_state`：当前应用配置状态。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
pub async fn get_exercise_html<R: Runtime>(
    app: AppHandle<R>,
    config_state: State<'_, crate::services::config::ConfigState>,
    product_code: String,
    resource_id: String,
) -> Result<application::ExerciseHtmlResponse, String> {
    application::get_exercise_html(app, config_state, product_code, resource_id).await
}
#[tauri::command]
/// 读取指定图书当前保存的阅读进度。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `product_code`：图书产品码。
pub async fn get_reading_progress(
    state: State<'_, crate::database::DbState>,
    product_code: String,
) -> Result<Option<ReadingProgress>, String> {
    info!("正在获取书籍进度 (product_code: {})", product_code);
    let db = state.get().await?;
    let statement = SqlStatement::new(
        "SELECT rp.book_id, rp.resource_id, rp.page_label, rp.scale, \
                rp.offset_x, rp.offset_y, rp.updated_at \
         FROM reading_progress rp \
         JOIN books b ON rp.book_id = b.id \
         WHERE b.product_code = ?",
        vec![SqlValue::Text(product_code)],
    );
    let rows = db
        .query_statement(statement)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(row) = rows.into_iter().next() {
        Ok(ReadingProgress::from_json(row))
    } else {
        Ok(None)
    }
}

#[tauri::command]
/// 新增或更新指定图书的阅读位置与视图状态。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `product_code`：图书产品码。
/// - `resource_id`：练习或学习单元资源标识。
/// - `page_label`：阅读页面标签。
/// - `scale`：阅读器缩放比例。
/// - `offset_x`：阅读器水平偏移量。
/// - `offset_y`：阅读器垂直偏移量。
pub async fn update_reading_progress(
    state: State<'_, crate::database::DbState>,
    product_code: String,
    resource_id: Option<String>,
    page_label: Option<String>,
    scale: f64,
    offset_x: i32,
    offset_y: i32,
) -> Result<(), String> {
    info!("正在更新书籍进度 (product_code: {})", product_code);
    let db = state.get().await?;
    let statement = SqlStatement::new(
        "INSERT INTO reading_progress (book_id, resource_id, page_label, scale, offset_x, offset_y) \
         SELECT id, ?, ?, ?, ?, ? FROM books WHERE product_code = ? \
         ON CONFLICT(book_id) DO UPDATE SET \
         resource_id=excluded.resource_id, \
         page_label=excluded.page_label, \
         scale=excluded.scale, \
         offset_x=excluded.offset_x, \
         offset_y=excluded.offset_y, \
         updated_at=CURRENT_TIMESTAMP",
        vec![
            resource_id.map_or(SqlValue::Null, SqlValue::Text),
            page_label.map_or(SqlValue::Null, SqlValue::Text),
            SqlValue::Real(scale),
            SqlValue::Integer(i64::from(offset_x)),
            SqlValue::Integer(i64::from(offset_y)),
            SqlValue::Text(product_code),
        ],
    );

    db.execute_statement(statement)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
/// 按分组读取书籍列表，并复用应用级缓存。
///
/// # 参数
/// - `state`：对应命令使用的共享状态。
/// - `cache_state`：书籍列表缓存状态。
/// - `group`：可选的图书分组筛选。
pub async fn get_books(
    state: State<'_, crate::database::DbState>,
    cache_state: State<'_, BookCacheState>,
    group: Option<BookGroup>,
) -> Result<Vec<Book>, String> {
    let key = CacheKey::book_list(group);

    if let Some(books) = cache_state.cache.get(&key).await {
        info!("从缓存获取书籍列表 (key: {})", key);
        return Ok(books);
    }

    let db = state.get().await?;
    let books = get_books_logic(db.as_ref(), group).await?;

    cache_state.cache.insert(key, books.clone()).await;

    Ok(books)
}

#[tauri::command]
/// 从本地来源或 R2 缓存读取图书封面。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `state`：对应命令使用的共享状态。
/// - `book`：目标图书模型。
pub async fn get_book_cover(
    app: tauri::AppHandle,
    state: State<'_, crate::services::config::ConfigState>,
    book: Book,
) -> Result<Vec<u8>, String> {
    let source = {
        let config = state.0.read().map_err(|e| e.to_string())?;
        config
            .book_source
            .as_ref()
            .ok_or("Book source not configured")?
            .clone()
    };

    let cover_name = book.cover.as_ref().ok_or("Book cover not defined")?;
    let relative_path = format!("books/{}/assets/{}", book.product_code, cover_name);

    match source {
        BookSource::Local { path } => {
            let base = std::path::PathBuf::from(&path);
            crate::utils::local::ensure_path_not_in_project_temp(&base, "book_source.local.path")?;
            info!(
                "读取封面（本地源）: product_code={}, path={}/{}",
                book.product_code, path, relative_path
            );
            crate::utils::local::read_file(&path, &relative_path).await
        }
        BookSource::CloudflareGateway {} => {
            let cache_dir = app
                .path()
                .app_cache_dir()
                .map_err(|e| format!("无法获取缓存目录: {}", e))?;
            let cache_cover_path = cache_dir.join(&relative_path);
            let key = relative_path.clone();
            let product_code = book.product_code.clone();

            info!(
                "读取封面（网关源，本地优先）: product_code={}, key={}, cache_path={}",
                product_code,
                key,
                cache_cover_path.display()
            );

            read_cached_or_fetch_and_store(&cache_cover_path, || async {
                let r2_state = app.state::<crate::utils::gateway::GatewayClientState>();
                let client = crate::utils::r2::get_client(&state, &r2_state).await?;
                crate::utils::r2::get_object(&client, &key)
                    .await
                    .map_err(|e| {
                        format!(
                            "从网关下载封面失败 (product_code: {}, key: {}): {}",
                            product_code, key, e
                        )
                    })
            })
            .await
        }
    }
}

/// 执行书籍列表数据库查询，供命令和测试复用。
///
/// # 参数
/// - `db`：目标数据库实现。
/// - `group`：可选的图书分组筛选。
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
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::NamedTempFile;

    fn fixture_root() -> PathBuf {
        PathBuf::from("tests").join("fixtures")
    }

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

    #[tokio::test]
    async fn test_get_books_command_integration() {
        use crate::database::{DbState, migrate_up};
        use tauri::test::mock_app;
        use tokio::sync::RwLock as AsyncRwLock;

        let app = mock_app();

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = SqliteDatabase::new(&path).await.unwrap();
        migrate_up(&db, None).await.unwrap();

        db.execute("INSERT INTO books (book_group, product_code, title, author, product_type, sort_num) VALUES (1, 'book1', 'Title 1', NULL, 'imgbook', 1)".to_string()).await.unwrap();

        app.manage(DbState {
            db: AsyncRwLock::new(Some(std::sync::Arc::new(db))),
            ..DbState::default()
        });
        app.manage(BookCacheState {
            cache: moka::future::Cache::new(10),
        });

        let state = app.state::<DbState>();
        let cache_state = app.state::<BookCacheState>();

        let result = get_books(state, cache_state, None).await.unwrap();
        assert_eq!(result.len(), 4);
        assert!(result.iter().any(|b| b.product_code == "book1"));
    }

    #[tokio::test]
    async fn test_reading_progress_commands() {
        use crate::database::{DbState, migrate_up};
        use tauri::test::mock_app;
        use tokio::sync::RwLock as AsyncRwLock;

        let app = mock_app();

        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let db = crate::database::SqliteDatabase::new(&path).await.unwrap();
        migrate_up(&db, None).await.unwrap();

        db.execute("INSERT INTO books (id, book_group, product_code, title, product_type, sort_num) VALUES (999, 1, 'test', 'Test', 'imgbook', 1)".to_string()).await.unwrap();

        app.manage(DbState {
            db: AsyncRwLock::new(Some(std::sync::Arc::new(db))),
            ..DbState::default()
        });

        let state = app.state::<DbState>();

        let progress = get_reading_progress(state.clone(), "test".to_string())
            .await
            .unwrap();
        assert!(progress.is_none());

        update_reading_progress(
            state.clone(),
            "test".to_string(),
            Some("RE_001".to_string()),
            Some("1".to_string()),
            1.5,
            10,
            20,
        )
        .await
        .unwrap();

        let progress = get_reading_progress(state.clone(), "test".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(progress.book_id, 999);
        assert_eq!(progress.resource_id, Some("RE_001".to_string()));
        assert_eq!(progress.scale, 1.5);
        assert_eq!(progress.offset_x, 10);
        assert_eq!(progress.offset_y, 20);

        update_reading_progress(
            state.clone(),
            "test".to_string(),
            Some("RE_002".to_string()),
            Some("2".to_string()),
            2.0,
            30,
            40,
        )
        .await
        .unwrap();

        let progress = get_reading_progress(state.clone(), "test".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(progress.resource_id, Some("RE_002".to_string()));
        assert_eq!(progress.page_label, Some("2".to_string()));
        assert_eq!(progress.scale, 2.0);
        assert_eq!(progress.offset_x, 30);
        assert_eq!(progress.offset_y, 40);
    }

    #[tokio::test]
    async fn test_get_book_metadata_command() {
        use crate::models::AppConfig;
        use crate::services::config::ConfigState;
        use std::sync::RwLock;
        use tauri::test::mock_app;

        let app = mock_app();

        let mut config = AppConfig::default();
        let base_path = fixture_root();
        config.book_source = Some(BookSource::Local {
            path: base_path.to_str().unwrap().to_string(),
        });

        app.manage(ConfigState(RwLock::new(config)));

        let handle = app.app_handle();
        let config_state = app.state::<ConfigState>();

        let product_code = "essgiuebk".to_string();

        let result = get_book_metadata(handle.clone(), config_state, product_code)
            .await
            .unwrap();

        assert!(!result.toc.is_empty());
        assert!(!result.pages.is_empty());
        assert!(result.exercise_toc.is_some());
        assert!(!result.exercise_toc.as_ref().unwrap().is_empty());
        assert!(result.pages.contains_key("13"));
        assert!(result.page_width > 0.0);
    }

    #[tokio::test]
    async fn test_resolve_page_resource_command() {
        use crate::models::AppConfig;
        use crate::services::config::ConfigState;
        use std::sync::RwLock;
        use tauri::test::mock_app;

        let app = mock_app();

        let mut config = AppConfig::default();
        let base_path = fixture_root();
        config.book_source = Some(BookSource::Local {
            path: base_path.to_str().unwrap().to_string(),
        });

        // Ensure the dummy image file exists for the test
        let img_path =
            base_path.join("books/essgiuebk/assets/images/xlrg/9781107480551book-updated13.jpg");
        std::fs::create_dir_all(img_path.parent().unwrap()).unwrap();
        if !img_path.exists() {
            std::fs::write(&img_path, b"dummy").unwrap();
        }

        app.manage(ConfigState(RwLock::new(config)));

        let handle = app.app_handle();
        let config_state = app.state::<ConfigState>();

        let product_code = "essgiuebk".to_string();
        let page_label = "12".to_string();

        let result = resolve_page_resource(handle.clone(), config_state, product_code, page_label)
            .await
            .unwrap();

        assert!(result.contains("9781107480551book-updated13.jpg"));
    }

    #[tokio::test]
    async fn test_resolve_book_asset_command() {
        use crate::models::AppConfig;
        use crate::services::config::ConfigState;
        use std::sync::RwLock;
        use tauri::test::mock_app;

        let app = mock_app();

        let mut config = AppConfig::default();
        let base_path = fixture_root();
        config.book_source = Some(BookSource::Local {
            path: base_path.to_str().unwrap().to_string(),
        });

        // Mock an audio file inside assets folder
        let audio_rel_path = "overlays/audio/audio1.mp3";
        // The actual physical path should be under assets/
        let audio_path = base_path
            .join("books/essgiuebk/assets")
            .join(audio_rel_path);
        std::fs::create_dir_all(audio_path.parent().unwrap()).unwrap();
        std::fs::write(&audio_path, b"dummy audio").unwrap();

        app.manage(ConfigState(RwLock::new(config)));

        let handle = app.app_handle();
        let config_state = app.state::<ConfigState>();

        let product_code = "essgiuebk".to_string();

        // This should fail currently because it doesn't look into assets/
        let result = resolve_book_asset(
            handle.clone(),
            config_state,
            product_code,
            audio_rel_path.to_string(),
        )
        .await;

        assert!(
            result.is_ok(),
            "Should find the file even if requested without 'assets/' prefix. Error: {:?}",
            result.err()
        );
        assert!(result.unwrap().contains("audio1.mp3"));
    }

    #[tokio::test]
    async fn test_resolve_book_asset_rejects_parent_dir_traversal() {
        use crate::models::AppConfig;
        use crate::services::config::ConfigState;
        use std::sync::RwLock;
        use tauri::test::mock_app;

        let app = mock_app();

        let mut config = AppConfig::default();
        let base_path = fixture_root();
        config.book_source = Some(BookSource::Local {
            path: base_path.to_str().unwrap().to_string(),
        });

        app.manage(ConfigState(RwLock::new(config)));
        let handle = app.app_handle();
        let config_state = app.state::<ConfigState>();

        let result = resolve_book_asset(
            handle.clone(),
            config_state,
            "essgiuebk".to_string(),
            "../secrets.txt".to_string(),
        )
        .await;

        let err = result.expect_err("parent traversal should be rejected");
        assert!(err.contains("[ERR_PATH_OUTSIDE_BASE]"));
    }

    #[tokio::test]
    async fn test_resolve_exercise_resource_command() {
        use crate::models::AppConfig;
        use crate::services::config::ConfigState;
        use std::sync::RwLock;
        use tauri::test::mock_app;

        let app = mock_app();

        let mut config = AppConfig::default();
        let base_path = fixture_root();
        config.book_source = Some(BookSource::Local {
            path: base_path.to_str().unwrap().to_string(),
        });

        app.manage(ConfigState(RwLock::new(config)));

        let handle = app.app_handle();
        let config_state = app.state::<ConfigState>();

        let product_code = "essgiuebk".to_string();
        let resource_id = "RE_0001".to_string();

        let result =
            resolve_exercise_resource(handle.clone(), config_state, product_code, resource_id)
                .await
                .unwrap();

        let expected_prefix = base_path
            .join("courses")
            .join("essgiuebkcon")
            .to_string_lossy()
            .to_string();
        assert!(result.contains("index.html"));
        assert!(result.contains("07cf7db0991e11ecb1d45b87d87d8905"));
        assert!(
            result.starts_with(&expected_prefix),
            "resolve_exercise_resource should use fixtures under tests/, got: {}",
            result
        );
        let uses_temp_dir = PathBuf::from(&result)
            .components()
            .any(|component| component.as_os_str() == std::ffi::OsStr::new("temp"));
        assert!(
            !uses_temp_dir,
            "resolve_exercise_resource should not fallback to temp directory in tests, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_read_cached_or_fetch_and_store_prefers_local_cache() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("books/essgiuebk/assets/cover.jpg");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"cached-cover").unwrap();

        let remote_called = Arc::new(AtomicUsize::new(0));
        let remote_called_clone = remote_called.clone();
        let bytes = read_cached_or_fetch_and_store(&path, move || async move {
            remote_called_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<Vec<u8>, String>(b"remote-cover".to_vec())
        })
        .await
        .unwrap();

        assert_eq!(bytes, b"cached-cover");
        assert_eq!(remote_called.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_read_cached_or_fetch_and_store_fetches_and_persists_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("books/essgiuebk/assets/cover.jpg");

        let remote_called = Arc::new(AtomicUsize::new(0));
        let remote_called_clone = remote_called.clone();
        let bytes = read_cached_or_fetch_and_store(&path, move || async move {
            remote_called_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<Vec<u8>, String>(b"remote-cover".to_vec())
        })
        .await
        .unwrap();

        assert_eq!(bytes, b"remote-cover");
        assert_eq!(remote_called.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(&path).unwrap(), b"remote-cover");
    }
}
