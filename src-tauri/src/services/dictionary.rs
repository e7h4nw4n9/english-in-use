use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const API_BASE_URL: &str = "https://oxfordx.cp.com.cn";
const AUDIO_BASE_URL: &str = "https://oxford-x-file.oss-cn-hangzhou.aliyuncs.com/audio";
const GRAPH_IMAGE_BASE_URL: &str =
    "https://oxford-x-file.oss-cn-hangzhou.aliyuncs.com/ill-picture/for_list";
const MAX_AUDIO_BYTES: usize = 16 * 1024 * 1024;
const MAX_GRAPH_IMAGE_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryAuthStatus {
    pub authenticated: bool,
    pub cached_phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DictionarySearchResult {
    pub word_id: String,
    pub word: String,
    pub parts_of_speech: Vec<String>,
    pub definition_eng: String,
    pub definition_zh: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryAudioResponse {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryGraphItem {
    pub id: i64,
    pub file: String,
    pub file_sub: String,
    pub english: String,
    pub chinese: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryDailyTip {
    pub id: i64,
    pub word_id: String,
    pub word: String,
    pub pos: String,
    pub unbox: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryImageResponse {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope {
    status_code: i64,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Value,
}

fn dictionary_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("dictionary"))
}

fn token_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dictionary_dir(app)?.join("auth.token"))
}

fn phone_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dictionary_dir(app)?.join("last-phone.txt"))
}

fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dictionary_dir(app)?.join("cache"))
}

fn safe_identifier(value: &str) -> Result<String, String> {
    let normalized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let normalized = normalized.trim_matches(['.', '-']).to_string();
    if normalized.is_empty() || normalized.len() > 255 {
        return Err("词典资源标识无效".to_string());
    }
    Ok(normalized)
}

fn write_private_file(path: &Path, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary_path = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary_path)
        .map_err(|error| error.to_string())?;
    if let Err(error) = file.write_all(content).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temporary_path);
        return Err(error.to_string());
    }
    drop(file);
    if let Err(error) = fs::rename(&temporary_path, path) {
        #[cfg(windows)]
        if path.exists() {
            let backup_path = path.with_extension(format!("{}.backup", uuid::Uuid::new_v4()));
            fs::rename(path, &backup_path).map_err(|backup_error| {
                let _ = fs::remove_file(&temporary_path);
                format!("备份旧词典文件失败: {backup_error}")
            })?;
            return match fs::rename(&temporary_path, path) {
                Ok(()) => {
                    let _ = fs::remove_file(backup_path);
                    Ok(())
                }
                Err(replace_error) => {
                    let _ = fs::rename(backup_path, path);
                    let _ = fs::remove_file(&temporary_path);
                    Err(format!("替换词典文件失败: {replace_error}"))
                }
            };
        }
        let _ = fs::remove_file(&temporary_path);
        return Err(error.to_string());
    }
    Ok(())
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/135.0.0.0 Safari/537.36")
        .build()
        .map_err(|error| error.to_string())
}

fn api_request(
    app: &AppHandle,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, String> {
    let request = request
        .header(ACCEPT, "application/x.oxford10.v1+json")
        .header(ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9")
        .header(ORIGIN, "https://oalecd10.cp.com.cn")
        .header(REFERER, "https://oalecd10.cp.com.cn/");
    Ok(match fs::read_to_string(token_path(app)?) {
        Ok(token) if !token.trim().is_empty() => {
            request.header(AUTHORIZATION, format!("Bearer {}", token.trim()))
        }
        _ => request,
    })
}

async fn decode_response(app: &AppHandle, response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        logout(app)?;
        return Err("[ERR_DICTIONARY_AUTH_REQUIRED] 词典登录已失效，请重新登录".to_string());
    }
    let envelope = response
        .json::<ApiEnvelope>()
        .await
        .map_err(|error| format!("词典接口返回格式错误: {error}"))?;
    if envelope.status_code == 411 {
        logout(app)?;
        return Err("[ERR_DICTIONARY_AUTH_REQUIRED] 词典登录已失效，请重新登录".to_string());
    }
    if envelope.status_code != 0 {
        return Err(if envelope.message.is_empty() {
            format!("词典接口返回错误: {}", envelope.status_code)
        } else {
            envelope.message
        });
    }
    Ok(envelope.data)
}

/// 返回本地是否保存了词典认证令牌。
pub fn auth_status(app: &AppHandle) -> Result<DictionaryAuthStatus, String> {
    let authenticated = fs::read_to_string(token_path(app)?)
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false);
    let cached_phone = fs::read_to_string(phone_path(app)?)
        .ok()
        .map(|phone| phone.trim().to_string())
        .filter(|phone| !phone.is_empty());
    Ok(DictionaryAuthStatus {
        authenticated,
        cached_phone,
    })
}

/// 请求手机号验证码。
pub async fn send_verify_code(app: &AppHandle, phone: &str) -> Result<(), String> {
    let phone = phone.trim();
    if phone.is_empty() || phone.len() > 32 {
        return Err("请输入有效手机号".to_string());
    }
    let response = api_request(
        app,
        client()?
            .post(format!("{API_BASE_URL}/api/auth/login/getVerifyCode"))
            .form(&[("phone", phone)]),
    )?
    .send()
    .await
    .map_err(|error| format!("验证码请求失败: {error}"))?;
    decode_response(app, response).await.map(|_| ())
}

/// 使用手机号和验证码登录并私密保存令牌。
pub async fn login(app: &AppHandle, phone: &str, code: &str) -> Result<(), String> {
    let phone = phone.trim();
    let code = code.trim();
    if phone.is_empty() || code.is_empty() || phone.len() > 32 || code.len() > 16 {
        return Err("手机号或验证码无效".to_string());
    }
    let response = api_request(
        app,
        client()?
            .post(format!("{API_BASE_URL}/api/auth/login"))
            .form(&[("phone", phone), ("code", code)]),
    )?
    .send()
    .await
    .map_err(|error| format!("词典登录失败: {error}"))?;
    let data = decode_response(app, response).await?;
    let token = data
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| "登录接口未返回 Token".to_string())?;
    write_private_file(&phone_path(app)?, phone.as_bytes())?;
    write_private_file(&token_path(app)?, token.as_bytes())
}

/// 删除本地词典认证令牌。
pub fn logout(app: &AppHandle) -> Result<(), String> {
    let path = token_path(app)?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn normalize_search_results(data: Value, limit: usize) -> Vec<DictionarySearchResult> {
    let mut indexes = HashMap::<String, usize>::new();
    let mut results = Vec::<DictionarySearchResult>::new();

    for item in data.as_array().into_iter().flatten() {
        let Some(word_id) = item
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(word) = item
            .get("word")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let parts_of_speech = item
            .get("pos")
            .and_then(Value::as_str)
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Some(index) = indexes.get(word_id).copied() {
            for part_of_speech in parts_of_speech {
                if !results[index]
                    .parts_of_speech
                    .iter()
                    .any(|value| value == &part_of_speech)
                {
                    results[index].parts_of_speech.push(part_of_speech);
                }
            }
            continue;
        }
        if results.len() >= limit {
            continue;
        }

        indexes.insert(word_id.to_string(), results.len());
        results.push(DictionarySearchResult {
            word_id: word_id.to_string(),
            word: word.to_string(),
            parts_of_speech,
            definition_eng: item
                .get("def_eng")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string(),
            definition_zh: item
                .get("def_simp")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string(),
        });
    }

    results
}

fn is_auth_required(error: &str) -> bool {
    error.starts_with("[ERR_DICTIONARY_AUTH_REQUIRED]")
}

fn required_string(data: &Value, field: &str, resource_name: &str) -> Result<String, String> {
    data.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{resource_name}缺少字段: {field}"))
}

fn normalize_graph_item(data: Value) -> Result<DictionaryGraphItem, String> {
    Ok(DictionaryGraphItem {
        id: data.get("id").and_then(Value::as_i64).unwrap_or_default(),
        file: required_string(&data, "file", "图解词汇")?,
        file_sub: required_string(&data, "file_sub", "图解词汇")?,
        english: required_string(&data, "english", "图解词汇")?,
        chinese: required_string(&data, "chinese", "图解词汇")?,
    })
}

fn normalize_daily_tip(data: Value) -> Result<DictionaryDailyTip, String> {
    Ok(DictionaryDailyTip {
        id: data.get("id").and_then(Value::as_i64).unwrap_or_default(),
        word_id: required_string(&data, "word_id", "实用贴士")?,
        word: required_string(&data, "word", "实用贴士")?,
        pos: data
            .get("pos")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string(),
        unbox: required_string(&data, "unbox", "实用贴士")?,
    })
}

/// 随机获取一条图解词汇元数据。
pub async fn random_graph(app: &AppHandle) -> Result<DictionaryGraphItem, String> {
    let response = api_request(
        app,
        client()?.get(format!("{API_BASE_URL}/api/words/graph/random")),
    )?
    .send()
    .await
    .map_err(|error| format!("图解词汇请求失败: {error}"))?;
    normalize_graph_item(decode_response(app, response).await?)
}

/// 获取当天的实用贴士元数据。
pub async fn daily_tip(app: &AppHandle) -> Result<DictionaryDailyTip, String> {
    let response = api_request(
        app,
        client()?.get(format!("{API_BASE_URL}/api/misc/daily-explain")),
    )?
    .send()
    .await
    .map_err(|error| format!("实用贴士请求失败: {error}"))?;
    normalize_daily_tip(decode_response(app, response).await?)
}

fn validate_graph_image_path(path: &str) -> Result<&str, String> {
    if path.is_empty() || path.len() > 255 || path.contains(['\\', '?', '#', '%']) {
        return Err("图解词汇图片路径无效".to_string());
    }
    let mut parts = path.split('/');
    let directory = parts.next().unwrap_or_default();
    let file_name = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || !matches!(directory, "img" | "img_sub")
        || file_name.is_empty()
        || !file_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    {
        return Err("图解词汇图片路径无效".to_string());
    }
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "webp") {
        return Err("图解词汇图片格式不受支持".to_string());
    }
    Ok(path)
}

fn graph_image_mime_type(path: &str) -> Result<&'static str, String> {
    match path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => Ok("image/jpeg"),
        Some("png") => Ok("image/png"),
        Some("webp") => Ok("image/webp"),
        _ => Err("图解词汇图片格式不受支持".to_string()),
    }
}

/// 优先读取非空图片缓存，未命中时下载并写入缓存。
async fn resolve_graph_image<F, Fut>(cache_path: &Path, fetch_online: F) -> Result<Vec<u8>, String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<u8>, String>>,
{
    if let Ok(bytes) = fs::read(cache_path)
        && !bytes.is_empty()
        && bytes.len() <= MAX_GRAPH_IMAGE_BYTES
    {
        return Ok(bytes);
    }

    let bytes = fetch_online().await?;
    write_private_file(cache_path, &bytes)?;
    Ok(bytes)
}

/// 下载图解词汇的受控缩略图或完整图片。
pub async fn graph_image(app: &AppHandle, path: &str) -> Result<DictionaryImageResponse, String> {
    let path = validate_graph_image_path(path)?;
    let mime_type = graph_image_mime_type(path)?.to_string();
    let cache_path = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("无法获取图解词汇缓存目录: {error}"))?
        .join("dictionary")
        .join("graph-images")
        .join(path);
    let bytes = resolve_graph_image(&cache_path, || async {
        let response = client()?
            .get(format!("{GRAPH_IMAGE_BASE_URL}/{path}"))
            .header(ACCEPT, "image/avif,image/webp,image/apng,image/*,*/*;q=0.8")
            .header(REFERER, "https://oalecd10.cp.com.cn/")
            .send()
            .await
            .map_err(|error| format!("图解词汇图片下载失败: {error}"))?
            .error_for_status()
            .map_err(|error| format!("图解词汇图片下载失败: {error}"))?;
        if response
            .content_length()
            .is_some_and(|length| length as usize > MAX_GRAPH_IMAGE_BYTES)
        {
            return Err("图解词汇图片超过大小限制".to_string());
        }
        let response_mime_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        if !response_mime_type.starts_with("image/") {
            return Err("图解词汇资源不是有效图片".to_string());
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| error.to_string())?
            .to_vec();
        if bytes.len() > MAX_GRAPH_IMAGE_BYTES {
            return Err("图解词汇图片超过大小限制".to_string());
        }
        Ok(bytes)
    })
    .await?;
    Ok(DictionaryImageResponse { bytes, mime_type })
}

/// 查询候选词，网络失败时仍可按词头读取离线详情。
pub async fn search(
    app: &AppHandle,
    word: &str,
    limit: usize,
) -> Result<Vec<DictionarySearchResult>, String> {
    let word = word.trim();
    if word.is_empty() || word.chars().count() > 128 {
        return Err("请输入要查询的内容".to_string());
    }
    let limit_value = limit.to_string();
    let online = async {
        let response = api_request(
            app,
            client()?.get(format!("{API_BASE_URL}/api/search")).query(&[
                ("word", word),
                ("type", "1"),
                ("limit", limit_value.as_str()),
            ]),
        )?
        .send()
        .await
        .map_err(|error| format!("词典查询失败: {error}"))?;
        decode_response(app, response).await
    }
    .await;
    match online {
        Ok(data) => Ok(normalize_search_results(data, limit)),
        Err(online_error) if is_auth_required(&online_error) => Err(online_error),
        Err(online_error) => cached_search(app, word, limit).map_err(|_| online_error),
    }
}

fn detail_cache_path(app: &AppHandle, word_id: &str) -> Result<PathBuf, String> {
    Ok(cache_dir(app)?
        .join("details")
        .join(format!("{}.json", safe_identifier(word_id)?)))
}

fn cached_search(
    app: &AppHandle,
    word: &str,
    limit: usize,
) -> Result<Vec<DictionarySearchResult>, String> {
    let directory = cache_dir(app)?.join("details");
    let entries = fs::read_dir(directory).map_err(|error| error.to_string())?;
    let normalized = word.to_lowercase();
    for entry in entries.flatten() {
        let Ok(content) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<Value>(&content) else {
            continue;
        };
        if data
            .get("word")
            .and_then(Value::as_str)
            .is_some_and(|cached_word| cached_word.to_lowercase() == normalized)
        {
            return Ok(normalize_search_results(
                serde_json::json!([{
                    "id": data.get("id").cloned().unwrap_or(Value::Null),
                    "word": data.get("word").cloned().unwrap_or(Value::Null),
                    "def_eng": "已保存的离线查询结果",
                    "def_simp": "",
                    "pos": ""
                }]),
                limit,
            ));
        }
    }
    Err("没有可用的离线查询结果".to_string())
}

/// 按配置从缓存或远端取得单词详情。
async fn resolve_word_detail<F, Fut>(
    cache_path: &Path,
    cache_enabled: bool,
    fetch_online: F,
) -> Result<Value, String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    if cache_enabled
        && let Ok(content) = fs::read(cache_path)
        && let Ok(data) = serde_json::from_slice::<Value>(&content)
    {
        return Ok(data);
    }

    let data = fetch_online().await?;
    if cache_enabled {
        let encoded = serde_json::to_vec(&data).map_err(|error| error.to_string())?;
        write_private_file(cache_path, &encoded)?;
    }
    Ok(data)
}

/// 获取单词详情；启用离线保存时优先读取有效缓存。
pub async fn word_detail(
    app: &AppHandle,
    word_id: &str,
    save_offline: bool,
) -> Result<Value, String> {
    let cache_path = detail_cache_path(app, word_id)?;
    resolve_word_detail(&cache_path, save_offline, || async {
        let response = api_request(
            app,
            client()?
                .get(format!("{API_BASE_URL}/api/word-detail"))
                .query(&[("word_id", word_id)]),
        )?
        .send()
        .await
        .map_err(|error| format!("单词详情查询失败: {error}"))?;
        decode_response(app, response).await
    })
    .await
}

fn validate_audio_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 255
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_#.-".contains(character))
    {
        return Err("音频资源标识无效".to_string());
    }
    Ok(())
}

/// 下载受控的单词或例句音频；启用离线保存时仅缓存实际播放的音频。
pub async fn audio(
    app: &AppHandle,
    kind: &str,
    name: &str,
    save_offline: bool,
) -> Result<DictionaryAudioResponse, String> {
    validate_audio_name(name)?;
    let (relative_path, extension, mime_type) = match kind {
        "word" => ("words/words_audio", "mp3", "audio/mpeg"),
        "example" => ("xgs/xgs_audio", "wav", "audio/wav"),
        _ => return Err("不支持的音频类型".to_string()),
    };
    let cache_path = cache_dir(app)?.join("audio").join(format!(
        "{}-{}.{}",
        kind,
        safe_identifier(name)?,
        extension
    ));
    if let Ok(bytes) = fs::read(&cache_path) {
        return Ok(DictionaryAudioResponse {
            bytes,
            mime_type: mime_type.to_string(),
        });
    }
    let encoded_name = name.replace('#', "%23");
    let response = client()?
        .get(format!(
            "{AUDIO_BASE_URL}/{relative_path}/{encoded_name}.{extension}"
        ))
        .header(ACCEPT, "*/*")
        .header(REFERER, "https://oalecd10.cp.com.cn/")
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|error| format!("音频下载失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("音频下载失败: {error}"))?;
    if response
        .content_length()
        .is_some_and(|length| length as usize > MAX_AUDIO_BYTES)
    {
        return Err("音频文件超过大小限制".to_string());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| error.to_string())?
        .to_vec();
    if bytes.len() > MAX_AUDIO_BYTES {
        return Err("音频文件超过大小限制".to_string());
    }
    if save_offline {
        write_private_file(&cache_path, &bytes)?;
    }
    Ok(DictionaryAudioResponse {
        bytes,
        mime_type: mime_type.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    #[test]
    fn validates_dictionary_resource_identifiers() {
        assert_eq!(safe_identifier("u596c.123-abc").unwrap(), "u596c.123-abc");
        assert!(safe_identifier("../../").is_err());
        assert!(validate_audio_name("analogy#_gb_2").is_ok());
        assert!(validate_audio_name("../secret").is_err());
        assert!(validate_audio_name("https://example.com/a").is_err());
        assert_eq!(
            validate_graph_image_path("img/livingroom.jpg").unwrap(),
            "img/livingroom.jpg"
        );
        assert!(validate_graph_image_path("img_sub/livingroom.jpg").is_ok());
        assert!(validate_graph_image_path("../livingroom.jpg").is_err());
        assert!(validate_graph_image_path("img/../../secret.jpg").is_err());
        assert!(validate_graph_image_path("https://example.com/a.jpg").is_err());
        assert!(validate_graph_image_path("img/livingroom.svg").is_err());
        assert_eq!(
            graph_image_mime_type("img/livingroom.JPG").unwrap(),
            "image/jpeg"
        );
    }

    #[tokio::test]
    async fn graph_image_cache_hit_skips_online_request() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("img/livingroom.jpg");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"cached-image").unwrap();
        let requested = Arc::new(AtomicBool::new(false));
        let request_flag = Arc::clone(&requested);

        let result = resolve_graph_image(&path, || async move {
            request_flag.store(true, Ordering::SeqCst);
            Ok(b"online-image".to_vec())
        })
        .await
        .unwrap();

        assert_eq!(result, b"cached-image");
        assert!(!requested.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn empty_graph_image_cache_is_replaced_by_online_response() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("img/livingroom.jpg");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, []).unwrap();
        let request_count = Arc::new(AtomicUsize::new(0));
        let request_count_for_fetch = Arc::clone(&request_count);

        let result = resolve_graph_image(&path, || async move {
            request_count_for_fetch.fetch_add(1, Ordering::SeqCst);
            Ok(b"online-image".to_vec())
        })
        .await
        .unwrap();

        assert_eq!(result, b"online-image");
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        assert_eq!(fs::read(path).unwrap(), b"online-image");
    }

    #[test]
    fn normalizes_dictionary_discovery_metadata() {
        let graph = normalize_graph_item(serde_json::json!({
            "id": 85,
            "file": "img/livingroom.jpg",
            "file_sub": "img_sub/livingroom.jpg",
            "english": "livingroom",
            "chinese": "客厅"
        }))
        .unwrap();
        assert_eq!(graph.english, "livingroom");
        assert_eq!(graph.file_sub, "img_sub/livingroom.jpg");

        let tip = normalize_daily_tip(serde_json::json!({
            "id": 0,
            "word_id": "word-id",
            "word": "fashion",
            "pos": "noun",
            "unbox": "{\"tile\":{}}"
        }))
        .unwrap();
        assert_eq!(tip.word, "fashion");
        assert_eq!(tip.unbox, "{\"tile\":{}}");
    }

    #[test]
    fn merges_search_results_by_word_id_and_applies_limit() {
        let results = normalize_search_results(
            serde_json::json!([
                {"id": "word-1", "word": "word", "pos": "noun", "def_eng": "one"},
                {"id": "word-1", "word": "word", "pos": "verb", "def_eng": "two"},
                {"id": "word-2", "word": "words", "pos": "noun"}
            ]),
            1,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].word_id, "word-1");
        assert_eq!(results[0].parts_of_speech, vec!["noun", "verb"]);
    }

    #[tokio::test]
    async fn detail_cache_hit_skips_online_request() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("word.json");
        fs::write(&path, br#"{"word":"cached"}"#).unwrap();
        let requested = Arc::new(AtomicBool::new(false));
        let request_flag = Arc::clone(&requested);

        let result = resolve_word_detail(&path, true, || async move {
            request_flag.store(true, Ordering::SeqCst);
            Ok(serde_json::json!({"word": "online"}))
        })
        .await
        .unwrap();

        assert_eq!(result["word"], "cached");
        assert!(!requested.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn disabled_detail_cache_does_not_read_or_write_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("word.json");
        fs::write(&path, br#"{"word":"cached"}"#).unwrap();

        let error = resolve_word_detail(&path, false, || async {
            Err("online unavailable".to_string())
        })
        .await
        .unwrap_err();

        assert_eq!(error, "online unavailable");
        assert_eq!(fs::read_to_string(path).unwrap(), r#"{"word":"cached"}"#);
    }

    #[tokio::test]
    async fn invalid_detail_cache_is_replaced_by_online_response() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("word.json");
        fs::write(&path, b"invalid json").unwrap();

        let result = resolve_word_detail(&path, true, || async {
            Ok(serde_json::json!({"word": "online"}))
        })
        .await
        .unwrap();

        assert_eq!(result["word"], "online");
        let cached: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(cached["word"], "online");
    }
}
