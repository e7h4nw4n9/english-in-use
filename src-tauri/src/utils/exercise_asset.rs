use log::{debug, error, info, warn};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use tauri::Manager;
use tauri::http::{Method, Request, Response, StatusCode, header::CONTENT_TYPE};
use tauri::utils::mime_type::MimeType;
use tauri::{Runtime, UriSchemeContext, UriSchemeResponder};

static HTML_CACHE: LazyLock<RwLock<HtmlCache>> =
    LazyLock::new(|| RwLock::new(HtmlCache::default()));

const MAX_HTML_CACHE_ENTRIES: usize = 64;
const MAX_HTML_CACHE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Default)]
struct HtmlCache {
    entries: HashMap<String, String>,
    insertion_order: VecDeque<String>,
    total_bytes: usize,
}

impl HtmlCache {
    /// 写入处理后的 HTML，并限制缓存条目数与总内存占用。
    fn insert(&mut self, url: String, html: String) {
        if html.len() > MAX_HTML_CACHE_BYTES {
            return;
        }

        if let Some(previous) = self.entries.remove(&url) {
            self.total_bytes = self.total_bytes.saturating_sub(previous.len());
            self.insertion_order.retain(|key| key != &url);
        }

        self.total_bytes += html.len();
        self.insertion_order.push_back(url.clone());
        self.entries.insert(url, html);

        while self.entries.len() > MAX_HTML_CACHE_ENTRIES || self.total_bytes > MAX_HTML_CACHE_BYTES
        {
            let Some(oldest_url) = self.insertion_order.pop_front() else {
                break;
            };
            if let Some(removed) = self.entries.remove(&oldest_url) {
                self.total_bytes = self.total_bytes.saturating_sub(removed.len());
            }
        }
    }
}

/// 缓存已处理的练习 HTML，供自定义资源协议直接返回。
///
/// # 参数
/// - `url`：练习资源 URL。
/// - `html`：已处理的练习 HTML。
pub fn cache_processed_html(url: String, html: String) {
    if let Ok(mut cache) = HTML_CACHE.write() {
        cache.insert(url, html);
    }
}

mod mapping;
mod protocol;
mod roots;

#[cfg(test)]
mod tests;

/// 处理 eiuasset 自定义协议请求并异步返回资源响应。
///
/// # 参数
/// - `ctx`：自定义协议请求上下文。
/// - `request`：自定义协议 HTTP 请求。
/// - `responder`：用于异步返回协议响应的响应器。
pub fn handle_request<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let response = protocol::build_response(ctx.app_handle(), request);
    responder.respond(response);
}
