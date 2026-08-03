//! 自定义资源协议的请求解码、安全校验与响应构造。

use super::mapping::resolve_path_with_engine_fallback;
use super::roots::collect_allowed_roots;
use super::*;

/// 校验自定义协议路径边界并构造资源响应。
///
/// # 参数
/// - `app`：Tauri 应用句柄。
/// - `request`：自定义协议请求。
pub(super) fn build_response<R: Runtime>(
    app: &tauri::AppHandle<R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    if !matches!(
        request.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    ) {
        return plain_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "method not allowed".to_string(),
        );
    }

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
            "exercise asset not found".to_string(),
        );
    };

    let canonical_path = match resolved_path.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            error!(
                "eiuasset canonicalize failed: path={} error={}",
                resolved_path.display(),
                err
            );
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to resolve exercise asset".to_string(),
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
            "exercise asset is outside allowed scope".to_string(),
        );
    };
    debug!(
        "eiuasset resolve success: uri={} canonical_path={} matched_root={}",
        uri,
        canonical_path.to_string_lossy(),
        matched_root.to_string_lossy()
    );

    match *request.method() {
        Method::OPTIONS => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Allow", "GET, HEAD, OPTIONS")
            .header("Access-Control-Max-Age", "86400")
            .body(Vec::new())
            .unwrap_or_else(|_| Response::new(Vec::new())),
        Method::HEAD => {
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
                .header("Content-Length", "0")
                .body(Vec::new())
                .unwrap_or_else(|_| Response::new(Vec::new()))
        }
        _ => {
            if let Ok(cache) = HTML_CACHE.read()
                && let Some(cached_html) = cache.entries.get(&uri)
            {
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
                    .body(cached_html.as_bytes().to_vec())
                    .unwrap_or_else(|_| Response::new(Vec::new()));
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
                        .body(bytes)
                        .unwrap_or_else(|_| Response::new(Vec::new()))
                }
                Err(err) => {
                    error!(
                        "eiuasset read failed: path={} error={}",
                        canonical_path.display(),
                        err
                    );
                    plain_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to read exercise asset".to_string(),
                    )
                }
            }
        }
    }
}

/// 从自定义协议请求中提取并 URL 解码资源路径。
///
/// # 参数
/// - `request`：自定义协议请求。
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

/// 按平台规则规范化已解码的资源路径。
///
/// # 参数
/// - `raw`：原始请求路径。
/// - `decoded`：URL 解码后的路径。
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

/// 判断资源请求是否需要输出诊断日志。
///
/// # 参数
/// - `path`：目标文件或目录路径。
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

/// 结合文件扩展名和内容嗅探结果确定 MIME 类型。
///
/// # 参数
/// - `path`：目标文件或目录路径。
/// - `content`：资源字节内容。
fn content_type_for_path(path: &Path, content: &[u8]) -> String {
    let sniffed_len = content.len().min(8192);
    let sniffed = &content[..sniffed_len];
    MimeType::parse(sniffed, &path.to_string_lossy())
}

/// 构造纯文本错误响应，并按状态码记录日志。
///
/// # 参数
/// - `status`：HTTP 响应状态。
/// - `message`：响应和日志使用的错误信息。
fn plain_response(status: StatusCode, message: String) -> Response<Vec<u8>> {
    if status.is_server_error() {
        error!("{}", message);
    } else {
        warn!("{}", message);
    }

    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(message.into_bytes())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}
