//! 自定义资源协议允许访问的本地根目录收集。

use super::*;

/// 收集自定义协议允许访问的缓存、数据和应用包资源根目录。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
pub(super) fn collect_allowed_roots<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(cache_dir) = app.path().app_cache_dir() {
        push_canonical_root(&mut roots, cache_dir);
    }

    for candidate in local_book_source_candidates(app) {
        push_canonical_root(&mut roots, candidate);
    }

    for candidate in bundle_resource_candidates(app) {
        push_canonical_root(&mut roots, candidate);
    }

    roots.sort();
    roots.dedup();
    roots
}

/// 仅将可规范化且尚未出现的根目录加入白名单。
///
/// # 参数
/// - `roots`：允许访问的规范化根目录集合。
/// - `candidate`：待加入的候选目录。
fn push_canonical_root(roots: &mut Vec<PathBuf>, candidate: PathBuf) {
    if let Ok(path) = candidate.canonicalize() {
        roots.push(path);
    }
}

/// 根据本地图书配置生成协议资源候选路径。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
fn local_book_source_candidates<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(config_state) = app.try_state::<crate::services::config::ConfigState>()
        && let Ok(config) = config_state.0.read()
        && let Some(crate::models::BookSource::Local { path }) = &config.book_source
    {
        let base = PathBuf::from(path);
        if crate::utils::local::is_path_in_project_temp(&base) {
            warn!(
                "skip forbidden local book source root in project temp: {}",
                base.display()
            );
            return candidates;
        }
        candidates.push(base.clone());
        candidates.push(base.join("books"));
        candidates.push(base.join("courses"));
    }

    candidates
}

/// 根据应用包资源目录生成协议资源候选路径。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
fn bundle_resource_candidates<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        return bundle_candidates_from_resource_dir(&resource_dir);
    }
    Vec::new()
}

/// 生成兼容不同打包布局的图书与课程资源根目录。
///
/// # 参数
/// - `resource_dir`：应用包资源目录。
pub(super) fn bundle_candidates_from_resource_dir(resource_dir: &Path) -> Vec<PathBuf> {
    let resource_dir = resource_dir.to_path_buf();
    vec![
        resource_dir.join("books"),
        resource_dir.join("courses"),
        resource_dir.join("assets"),
        resource_dir.join("assets").join("books"),
        resource_dir.join("assets").join("courses"),
    ]
}
