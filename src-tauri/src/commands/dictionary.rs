use tauri::{AppHandle, State};

#[tauri::command]
/// 查询词典账号的本地登录状态。
pub fn dictionary_auth_status(
    app: AppHandle,
) -> Result<crate::services::dictionary::DictionaryAuthStatus, String> {
    crate::services::dictionary::auth_status(&app)
}

#[tauri::command]
/// 向指定手机号发送词典登录验证码。
pub async fn dictionary_send_verify_code(app: AppHandle, phone: String) -> Result<(), String> {
    crate::services::dictionary::send_verify_code(&app, &phone).await
}

#[tauri::command]
/// 使用手机号和验证码登录词典。
pub async fn dictionary_login(app: AppHandle, phone: String, code: String) -> Result<(), String> {
    crate::services::dictionary::login(&app, &phone, &code).await
}

#[tauri::command]
/// 删除本地词典登录令牌。
pub fn dictionary_logout(app: AppHandle) -> Result<(), String> {
    crate::services::dictionary::logout(&app)
}

#[tauri::command]
/// 随机获取一条图解词汇。
pub async fn dictionary_random_graph(
    app: AppHandle,
) -> Result<crate::services::dictionary::DictionaryGraphItem, String> {
    crate::services::dictionary::random_graph(&app).await
}

#[tauri::command]
/// 获取每日实用贴士。
pub async fn dictionary_daily_tip(
    app: AppHandle,
) -> Result<crate::services::dictionary::DictionaryDailyTip, String> {
    crate::services::dictionary::daily_tip(&app).await
}

#[tauri::command]
/// 下载受控的图解词汇图片。
pub async fn dictionary_graph_image(
    app: AppHandle,
    path: String,
) -> Result<crate::services::dictionary::DictionaryImageResponse, String> {
    crate::services::dictionary::graph_image(&app, &path).await
}

#[tauri::command]
/// 查询与输入内容匹配的词条候选项。
pub async fn dictionary_search(
    app: AppHandle,
    config_state: State<'_, crate::services::config::ConfigState>,
    word: String,
) -> Result<Vec<crate::services::dictionary::DictionarySearchResult>, String> {
    let limit = usize::from(
        config_state
            .0
            .read()
            .map_err(|error| error.to_string())?
            .dictionary
            .search_result_limit,
    );
    crate::services::dictionary::search(&app, &word, limit).await
}

#[tauri::command]
/// 获取指定词条的完整详情。
pub async fn dictionary_word_detail(
    app: AppHandle,
    config_state: State<'_, crate::services::config::ConfigState>,
    word_id: String,
) -> Result<serde_json::Value, String> {
    let save_offline = config_state
        .0
        .read()
        .map_err(|error| error.to_string())?
        .dictionary
        .save_query_results_offline;
    crate::services::dictionary::word_detail(&app, &word_id, save_offline).await
}

#[tauri::command]
/// 获取受控的词条或例句发音。
pub async fn dictionary_audio(
    app: AppHandle,
    config_state: State<'_, crate::services::config::ConfigState>,
    kind: String,
    name: String,
) -> Result<crate::services::dictionary::DictionaryAudioResponse, String> {
    let save_offline = config_state
        .0
        .read()
        .map_err(|error| error.to_string())?
        .dictionary
        .save_query_results_offline;
    crate::services::dictionary::audio(&app, &kind, &name, save_offline).await
}
