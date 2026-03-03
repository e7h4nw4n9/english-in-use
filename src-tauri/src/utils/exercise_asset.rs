use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tauri::Manager;
use tauri::http::{Method, Request, Response, StatusCode, header::CONTENT_TYPE};
use tauri::utils::mime_type::MimeType;
use tauri::{Runtime, UriSchemeContext, UriSchemeResponder};

lazy_static::lazy_static! {
    static ref HTML_CACHE: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

pub fn cache_processed_html(url: String, html: String) {
    if let Ok(mut cache) = HTML_CACHE.write() {
        cache.insert(url, html);
    }
}

pub fn handle_request<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let response = build_response(ctx.app_handle(), request);
    responder.respond(response);
}

fn build_response<R: Runtime>(
    app: &tauri::AppHandle<R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let decoded_path = match decode_request_path(&request) {
        Ok(path) => path,
        Err(message) => return plain_response(StatusCode::BAD_REQUEST, message),
    };
    let trace_request = should_trace_asset_request(&decoded_path);
    if trace_request {
        info!(
            "eiuasset request: method={} uri={} decoded_path={}",
            request.method(),
            uri,
            decoded_path
        );
    }

    let requested_path = PathBuf::from(&decoded_path);
    let resolved_path = resolve_path_with_engine_fallback(&requested_path);

    let Some(resolved_path) = resolved_path else {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());
        warn!(
            "eiuasset 404: uri={} decoded_path={} requested_is_absolute={} cwd={}",
            uri,
            decoded_path,
            requested_path.is_absolute(),
            cwd
        );
        return plain_response(
            StatusCode::NOT_FOUND,
            format!(
                "exercise asset not found: uri={}, decoded_path={}, requested_path={}",
                uri,
                decoded_path,
                requested_path.to_string_lossy()
            ),
        );
    };

    let canonical_path = match resolved_path.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "failed to canonicalize path {}: {}",
                    resolved_path.to_string_lossy(),
                    err
                ),
            );
        }
    };

    let allowed_roots = collect_allowed_roots(app);
    let Some(matched_root) = allowed_roots
        .iter()
        .find(|root| canonical_path.starts_with(root))
    else {
        warn!(
            "eiuasset 403: uri={} canonical_path={} allowed_roots={}",
            uri,
            canonical_path.to_string_lossy(),
            allowed_roots
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
        return plain_response(
            StatusCode::FORBIDDEN,
            format!(
                "path is outside allowed scope: {}",
                canonical_path.to_string_lossy()
            ),
        );
    };
    debug!(
        "eiuasset resolve success: uri={} canonical_path={} matched_root={}",
        uri,
        canonical_path.to_string_lossy(),
        matched_root.to_string_lossy()
    );

    match request.method() {
        &Method::OPTIONS => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS, HEAD")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Allow-Private-Network", "true")
            .header("Access-Control-Max-Age", "86400")
            .body(Vec::new())
            .unwrap_or_else(|_| Response::new(Vec::new())),
        &Method::HEAD => {
            let content_type = content_type_for_path(&canonical_path, &[]);
            if trace_request {
                info!(
                    "eiuasset response: status=200 method=HEAD path={} content_type={} bytes=0",
                    canonical_path.to_string_lossy(),
                    content_type
                );
            }
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, content_type)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Private-Network", "true")
                .header("Content-Length", "0")
                .body(Vec::new())
                .unwrap_or_else(|_| Response::new(Vec::new()))
        }
        _ => {
            if let Ok(cache) = HTML_CACHE.read() {
                if let Some(cached_html) = cache.get(&uri) {
                    if trace_request {
                        info!(
                            "eiuasset response: status=200 method={} path={} (from cache) bytes={}",
                            request.method(),
                            canonical_path.to_string_lossy(),
                            cached_html.len()
                        );
                    }
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, "text/html; charset=utf-8")
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Private-Network", "true")
                        .body(cached_html.as_bytes().to_vec())
                        .unwrap_or_else(|_| Response::new(Vec::new()));
                }
            }

            match std::fs::read(&canonical_path) {
                Ok(bytes) => {
                    let content_type = content_type_for_path(&canonical_path, &bytes);
                    if trace_request {
                        info!(
                            "eiuasset response: status=200 method={} path={} content_type={} bytes={}",
                            request.method(),
                            canonical_path.to_string_lossy(),
                            content_type,
                            bytes.len()
                        );
                    }
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, content_type)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Private-Network", "true")
                        .body(bytes)
                        .unwrap_or_else(|_| Response::new(Vec::new()))
                }
                Err(err) => plain_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "failed to read file {}: {}",
                        canonical_path.to_string_lossy(),
                        err
                    ),
                ),
            }
        }
    }
}

fn decode_request_path(request: &Request<Vec<u8>>) -> Result<String, String> {
    let raw_full = request.uri().path();
    let raw = raw_full
        .split('?')
        .next()
        .unwrap_or(raw_full)
        .split('#')
        .next()
        .unwrap_or(raw_full);
    let decoded = urlencoding::decode(raw)
        .map_err(|err| format!("failed to decode request path {}: {}", raw, err))?
        .into_owned();
    let normalized = normalize_decoded_path(raw, decoded);
    if normalized.is_empty() {
        return Err("empty request path".to_string());
    }
    Ok(normalized)
}

fn normalize_decoded_path(raw: &str, decoded: String) -> String {
    #[cfg(target_os = "windows")]
    {
        let normalized = if looks_like_windows_drive_path(&decoded) {
            decoded.trim_start_matches('/').to_string()
        } else {
            decoded
        };
        if raw != normalized {
            warn!(
                "eiuasset normalize path (windows): raw={} normalized={}",
                raw, normalized
            );
        }
        normalized
    }

    #[cfg(not(target_os = "windows"))]
    {
        if decoded.starts_with('/') {
            return decoded;
        }
        let normalized = format!("/{}", decoded.trim_start_matches('/'));
        warn!(
            "eiuasset path missing leading slash, auto-fixed: raw={} normalized={}",
            raw, normalized
        );
        normalized
    }
}

#[cfg(target_os = "windows")]
fn looks_like_windows_drive_path(path: &str) -> bool {
    let trimmed = path.trim_start_matches('/');
    let bytes = trimmed.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn collect_allowed_roots<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(cache_dir) = app.path().app_cache_dir() {
        push_canonical_root(&mut roots, cache_dir);
    }

    if let Ok(data_dir) = app.path().app_data_dir() {
        push_canonical_root(&mut roots, data_dir);
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

fn push_canonical_root(roots: &mut Vec<PathBuf>, candidate: PathBuf) {
    if let Ok(path) = candidate.canonicalize() {
        roots.push(path);
    }
}

fn local_book_source_candidates<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(config_state) = app.try_state::<crate::services::config::ConfigState>() {
        if let Ok(config) = config_state.0.read() {
            if let Some(crate::models::BookSource::Local { path }) = &config.book_source {
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
        }
    }

    candidates
}

fn bundle_resource_candidates<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        return bundle_candidates_from_resource_dir(&resource_dir);
    }
    Vec::new()
}

fn bundle_candidates_from_resource_dir(resource_dir: &Path) -> Vec<PathBuf> {
    let resource_dir = resource_dir.to_path_buf();
    vec![
        resource_dir.join("books"),
        resource_dir.join("courses"),
        resource_dir.join("assets"),
        resource_dir.join("assets").join("books"),
        resource_dir.join("assets").join("courses"),
    ]
}

fn resolve_path_with_engine_fallback(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }

    if let Some(path) = resolve_engine_mapped_path(path) {
        return Some(path);
    }

    if let Some(path) = resolve_dp_mapped_path(path) {
        return Some(path);
    }

    warn!(
        "eiuasset resolve: path not found and not a known mapped path: {}",
        path.to_string_lossy()
    );
    None
}

fn resolve_engine_mapped_path(path: &Path) -> Option<PathBuf> {
    let (engine_dir, requested_version, suffix) = split_engine_path(path)?;

    let mut versions: Vec<String> = std::fs::read_dir(&engine_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            entry
                .file_name()
                .to_str()
                .map(std::string::ToString::to_string)
        })
        .collect();

    if versions.is_empty() {
        return None;
    }

    let target_norm = normalize_engine_version(&requested_version);
    versions.sort_by(|a, b| {
        let a_match = normalize_engine_version(a) == target_norm;
        let b_match = normalize_engine_version(b) == target_norm;
        b_match.cmp(&a_match).then_with(|| b.cmp(a))
    });

    let mut attempted = Vec::new();
    for version in versions {
        let candidate = engine_dir.join(&version).join(&suffix);
        if attempted.len() < 6 {
            attempted.push(format!(
                "{} (exists={})",
                candidate.to_string_lossy(),
                candidate.is_file()
            ));
        }
        if candidate.is_file() {
            if version != requested_version {
                warn!(
                    "exercise asset fallback: requested engine {} but served {} for {}",
                    requested_version,
                    version,
                    suffix.to_string_lossy()
                );
            }
            return Some(candidate);
        }
    }

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());
    warn!(
        "eiuasset resolve failed: requested_path={} engine_dir={} requested_version={} suffix={} cwd={} attempts={}",
        path.to_string_lossy(),
        engine_dir.to_string_lossy(),
        requested_version,
        suffix.to_string_lossy(),
        cwd,
        attempted.join(" | ")
    );

    None
}

fn resolve_dp_mapped_path(path: &Path) -> Option<PathBuf> {
    let (dp_dir, suffix) = split_dp_path(path)?;

    let mut versions: Vec<String> = std::fs::read_dir(&dp_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            entry
                .file_name()
                .to_str()
                .map(std::string::ToString::to_string)
        })
        .collect();
    if versions.is_empty() {
        return None;
    }

    let requested_first = suffix
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .unwrap_or_default()
        .to_string();
    versions.sort_by(|a, b| {
        let a_match = a == &requested_first;
        let b_match = b == &requested_first;
        b_match.cmp(&a_match).then_with(|| b.cmp(a))
    });

    let suffix_candidates = build_suffix_candidates(&suffix);
    if suffix_candidates.is_empty() {
        return None;
    }

    let mut attempted = Vec::new();
    for version in versions {
        let base = dp_dir.join(&version);
        for candidate_suffix in &suffix_candidates {
            let candidate = base.join(candidate_suffix);
            if attempted.len() < 8 {
                attempted.push(format!(
                    "{} (exists={})",
                    candidate.to_string_lossy(),
                    candidate.is_file()
                ));
            }
            if candidate.is_file() {
                if version != requested_first || candidate_suffix != &suffix {
                    warn!(
                        "exercise asset dp fallback: requested_suffix={} served_version={} served_suffix={}",
                        suffix.to_string_lossy(),
                        version,
                        candidate_suffix.to_string_lossy()
                    );
                }
                return Some(candidate);
            }
        }
    }

    warn!(
        "eiuasset dp resolve failed: requested_path={} dp_dir={} suffix={} attempts={}",
        path.to_string_lossy(),
        dp_dir.to_string_lossy(),
        suffix.to_string_lossy(),
        attempted.join(" | ")
    );
    None
}

fn split_engine_path(path: &Path) -> Option<(PathBuf, String, PathBuf)> {
    let mut cursor = path.parent()?;
    loop {
        let parent = cursor.parent()?;
        let deps_dir = parent.parent()?;
        let assets_dir = deps_dir.parent()?;

        let parent_name = parent.file_name()?.to_string_lossy();
        let deps_name = deps_dir.file_name()?.to_string_lossy();
        let assets_name = assets_dir.file_name()?.to_string_lossy();

        if parent_name == "engine" && deps_name == "deps" && assets_name == "assets" {
            let requested_version = cursor.file_name()?.to_string_lossy().to_string();
            let suffix = path.strip_prefix(cursor).ok()?.to_path_buf();
            return Some((parent.to_path_buf(), requested_version, suffix));
        }

        cursor = parent;
    }
}

fn split_dp_path(path: &Path) -> Option<(PathBuf, PathBuf)> {
    for ancestor in path.ancestors() {
        let name = ancestor.file_name()?.to_string_lossy();
        if name != "dp" {
            continue;
        }
        let deps_dir = ancestor.parent()?;
        let assets_dir = deps_dir.parent()?;
        let deps_name = deps_dir.file_name()?.to_string_lossy();
        let assets_name = assets_dir.file_name()?.to_string_lossy();
        if deps_name == "deps" && assets_name == "assets" {
            let suffix = path.strip_prefix(ancestor).ok()?.to_path_buf();
            return Some((ancestor.to_path_buf(), suffix));
        }
    }
    None
}

fn build_suffix_candidates(suffix: &Path) -> Vec<PathBuf> {
    let components: Vec<std::ffi::OsString> = suffix
        .components()
        .map(|component| component.as_os_str().to_os_string())
        .collect();
    if components.is_empty() {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for i in 0..components.len() {
        let mut path = PathBuf::new();
        for component in &components[i..] {
            path.push(component);
        }
        if !path.as_os_str().is_empty() {
            candidates.push(path);
        }
    }
    candidates
}

fn normalize_engine_version(version: &str) -> String {
    version
        .split_whitespace()
        .collect::<String>()
        .to_ascii_lowercase()
}

fn should_trace_asset_request(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".js",
        ".css",
        ".json",
        ".html",
        ".svg",
        ".wasm",
        "player.js",
        "launcher-connector-channel.bundle.js",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn content_type_for_path(path: &Path, content: &[u8]) -> String {
    let sniffed_len = content.len().min(8192);
    let sniffed = &content[..sniffed_len];
    MimeType::parse(sniffed, &path.to_string_lossy())
}

fn plain_response(status: StatusCode, message: String) -> Response<Vec<u8>> {
    if status.is_server_error() {
        error!("{}", message);
    } else {
        warn!("{}", message);
    }

    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Private-Network", "true")
        .body(message.into_bytes())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::bundle_candidates_from_resource_dir;
    use std::path::PathBuf;

    #[test]
    fn bundle_candidates_cover_book_and_course_subdirs() {
        let resource_dir = PathBuf::from("/Application/App.app/Contents/Resources");
        let candidates = bundle_candidates_from_resource_dir(&resource_dir);

        assert!(candidates.contains(&resource_dir.join("books")));
        assert!(candidates.contains(&resource_dir.join("courses")));
        assert!(candidates.contains(&resource_dir.join("assets")));
        assert!(candidates.contains(&resource_dir.join("assets").join("books")));
        assert!(candidates.contains(&resource_dir.join("assets").join("courses")));
    }
}
