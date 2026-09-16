//! 原生书页识别：串行处理、同页去重和有界内存缓存。

use serde_json::Value;
use std::{collections::VecDeque, path::PathBuf, sync::OnceLock};
use tokio::sync::Mutex;

type PageCache = VecDeque<(String, Value)>;
static CACHE: OnceLock<Mutex<PageCache>> = OnceLock::new();

/// 校验产品码，禁止通过拼接路径越出书籍资源目录。
pub fn validate_product_code(code: &str) -> Result<(), String> {
    if code.is_empty()
        || !code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err("无效的书籍产品码".into());
    }
    Ok(())
}

/// 识别已由书籍资源解析器限定的图片；只缓存成功结果。
pub async fn recognize(path: PathBuf) -> Result<Value, String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    let metadata = path.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("书页资源不是文件".into());
    }
    let key = format!(
        "{}:{}:{:?}",
        path.display(),
        metadata.len(),
        metadata.modified()
    );
    // 持锁期间等待后台识别，限制并发为一，同时合并重复页面请求。
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .await;
    if let Some(index) = cache.iter().position(|(cached_key, _)| cached_key == &key) {
        let item = cache.remove(index).expect("已找到的缓存项");
        let result = item.1.clone();
        cache.push_back(item);
        return Ok(result);
    }
    let result = tauri::async_runtime::spawn_blocking(move || native_recognize(&path))
        .await
        .map_err(|error| error.to_string())??;
    cache.push_back((key, result.clone()));
    while cache.len() > 6 {
        cache.pop_front();
    }
    Ok(result)
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe extern "C" {
    fn reader_recognize_page(path: *const std::ffi::c_char) -> *mut std::ffi::c_char;
    fn reader_native_free(value: *mut std::ffi::c_char);
    fn reader_copy_word(word: *const std::ffi::c_char);
}

/// 调用原生识别并在解析结果前释放原生缓冲区。
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn native_recognize(path: &std::path::Path) -> Result<Value, String> {
    let path =
        std::ffi::CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
    // 原生函数返回独立分配的字符串；复制后恰好释放一次。
    let bytes = unsafe {
        let pointer = reader_recognize_page(path.as_ptr());
        if pointer.is_null() {
            return Err("无法分配识别结果内存".into());
        }
        let bytes = std::ffi::CStr::from_ptr(pointer).to_bytes().to_vec();
        reader_native_free(pointer);
        bytes
    };
    let result: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if let Some(error) = result.get("error").and_then(Value::as_str) {
        return Err(error.into());
    }
    Ok(result)
}

/// 非 Apple 平台保留阅读能力，不替换为在线 OCR。
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn native_recognize(_path: &std::path::Path) -> Result<Value, String> {
    Err("当前平台不支持原生文字识别".into())
}

/// 只允许复制本期支持的单个单词。
pub fn copy_word(word: &str) -> Result<(), String> {
    static WORD_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
    let pattern = WORD_PATTERN.get_or_init(|| {
        regex::Regex::new(r"^[\p{Latin}\p{M}]+(?:['’\-][\p{Latin}\p{M}]+)*$")
            .expect("固定单词模式有效")
    });
    if word.len() > 512 || !pattern.is_match(word) {
        return Err("请选择一个单词".into());
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let word = std::ffi::CString::new(word).map_err(|e| e.to_string())?;
        // 原生实现将剪贴板写操作同步派发到主线程，字符串在返回前有效。
        unsafe {
            reader_copy_word(word.as_ptr());
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    Err("当前平台不支持原生复制".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        for code in ["", "../book", "/tmp", "a/b", "a\\b", "."] {
            assert!(validate_product_code(code).is_err());
        }
        assert!(validate_product_code("english-123_abc").is_ok());
    }

    #[test]
    fn rejects_non_word_copy() {
        assert!(copy_word("").is_err());
        assert!(copy_word("two words").is_err());
        assert!(copy_word("---").is_err());
    }
}
