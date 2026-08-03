//! 练习 HTML、依赖地址和运行时保护脚本改写。

use log::{error, warn};
use regex::Regex;
use std::path::PathBuf;

use super::source::{ExerciseSource, to_exercise_asset_url};

/// 按既有顺序完成练习 HTML 的全部兼容性改写。
///
/// # 参数
/// - `source`：已读取的练习源码及本地依赖路径。
/// - `resource_id`：用于诊断日志的练习资源标识。
pub(super) fn rewrite_exercise_html(
    source: ExerciseSource,
    resource_id: &str,
) -> Result<String, String> {
    let ExerciseSource {
        mut html,
        data_js_content_raw,
        engine_dir,
        dp_dir,
        engine_base_url,
        dp_base_url,
        media_base_url,
        ..
    } = source;

    // srcdoc 在打包环境下通常处于 tauri:// 协议，`//host/path` 会被错误解析为 tauri://host/path。
    // 这里统一升级为 https://，避免练习资源在 release 包中空白。
    html = normalize_protocol_relative_urls(&html);
    let data_js_content = data_js_content_raw.as_ref().map(|raw| {
        let sanitized = sanitize_data_js_media_content(raw, &media_base_url);
        normalize_protocol_relative_urls(&sanitized)
    });

    // 4. 内联 data.js
    if let Some(content) = data_js_content {
        let re_data_script =
            Regex::new(r#"<script\s+[^>]*src=['\"]data\.js['\"][^>]*></script>"#).unwrap();
        let inline_script = format!("<script type=\"text/javascript\">\n{}\n</script>", content);
        html = re_data_script
            .replace(&html, inline_script.as_str())
            .to_string();
    }

    // 5. 若本地 deps 已就绪，则替换为本地路径；否则保留原始远程配置并后台预热
    let normalize_engine_version = |v: &str| v.split_whitespace().collect::<String>();
    let mut resolved_engine_version: Option<String> = None;

    if engine_dir.exists() {
        let mut available_engine_versions: Vec<String> = std::fs::read_dir(&engine_dir)
            .map_err(|e| format!("读取引擎目录失败: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        available_engine_versions.sort();

        if available_engine_versions.is_empty() {
            warn!(
                "本地引擎目录为空，暂不替换为本地 deps: {}",
                engine_dir.display()
            );
        } else {
            let re_engine = Regex::new(
                r#"A5\.ENGINE_ROOT\s*=\s*['\"](?:(?:https?:)?//)[^/]+/(buckminster-[^/'\"]+)/?['\"]"#,
            )
            .map_err(|e| e.to_string())?;
            let preferred_engine_version_raw = re_engine
                .captures(&html)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()));

            let mut selected = preferred_engine_version_raw
                .as_ref()
                .and_then(|raw| {
                    available_engine_versions
                        .iter()
                        .find(|v| *v == raw)
                        .cloned()
                        .or_else(|| {
                            let normalized_raw = normalize_engine_version(raw);
                            available_engine_versions
                                .iter()
                                .find(|v| normalize_engine_version(v) == normalized_raw)
                                .cloned()
                        })
                })
                .or_else(|| available_engine_versions.last().cloned());

            let has_player_js =
                |version: &str| -> bool { engine_dir.join(version).join("player.js").exists() };
            if let Some(version) = selected.as_ref()
                && !has_player_js(version)
            {
                selected = available_engine_versions
                    .iter()
                    .rev()
                    .find(|candidate| has_player_js(candidate))
                    .cloned();
            }

            if let Some(version) = selected {
                let resolved_engine_root = format!("{}/{}/", engine_base_url, version);
                if re_engine.is_match(&html) {
                    html = re_engine
                        .replace_all(
                            &html,
                            format!("A5.ENGINE_ROOT = '{}'", resolved_engine_root),
                        )
                        .to_string();
                } else {
                    let inject = format!(
                        "<script>A5=A5||{{}};A5.ENGINE_ROOT='{}';</script>",
                        resolved_engine_root
                    );
                    if let Some(pos) = html.find("</head>") {
                        html.insert_str(pos, &inject);
                    } else {
                        html.push_str(&inject);
                    }
                }

                let re_geogebra =
                    Regex::new(r#"A5\.GEOGEBRA\s*=\s*['\"]https?://[^/]+/geogebra/(.*)['\"]"#)
                        .map_err(|e| e.to_string())?;
                html = re_geogebra
                    .replace_all(&html, |caps: &regex::Captures| {
                        format!("A5.GEOGEBRA = '{}/geogebra/{}/'", engine_base_url, &caps[1])
                    })
                    .to_string();

                let re_mathjax =
                    Regex::new(r#"A5\.MATHJAX\s*=\s*['\"]https?://[^/]+/mathjax/(.*)['\"]"#)
                        .map_err(|e| e.to_string())?;
                html = re_mathjax
                    .replace_all(&html, |caps: &regex::Captures| {
                        format!("A5.MATHJAX = '{}/mathjax/{}/'", engine_base_url, &caps[1])
                    })
                    .to_string();

                resolved_engine_version = Some(version);
            } else {
                warn!(
                    "未找到可用本地引擎版本，保持远程引擎配置: {}",
                    engine_dir.display()
                );
            }
        }
    } else {
        warn!(
            "本地引擎目录不存在，保持远程引擎配置: {}",
            engine_dir.display()
        );
    }

    if dp_dir.exists() {
        let mut available_dp_versions: Vec<String> = std::fs::read_dir(&dp_dir)
            .map_err(|e| format!("读取皮肤目录失败: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        available_dp_versions.sort();

        if available_dp_versions.is_empty() {
            warn!("本地皮肤目录为空，保持远程皮肤配置: {}", dp_dir.display());
        } else {
            let has_dp_entry = |version: &str| -> bool {
                let root = dp_dir.join(version);
                root.join("css").join("style.css").exists()
                    || root.join("js.js").exists()
                    || root.join("templates.js").exists()
            };

            let resolved_dp_version = available_dp_versions
                .iter()
                .rev()
                .find(|version| has_dp_entry(version))
                .cloned()
                .or_else(|| available_dp_versions.last().cloned());

            if let Some(dp_version) = resolved_dp_version {
                let resolved_skin_root = format!("{}/{}/", dp_base_url, dp_version);
                let re_skin = Regex::new(r#"A5\.SKIN_URL\s*=\s*['\"][^'\"]+['\"]"#)
                    .map_err(|e| e.to_string())?;
                if re_skin.is_match(&html) {
                    html = re_skin
                        .replace_all(&html, format!("A5.SKIN_URL = '{}'", resolved_skin_root))
                        .to_string();
                } else {
                    let inject = format!(
                        "<script>A5=A5||{{}};A5.SKIN_URL='{}';</script>",
                        resolved_skin_root
                    );
                    if let Some(pos) = html.find("</head>") {
                        html.insert_str(pos, &inject);
                    } else {
                        html.push_str(&inject);
                    }
                }
            } else {
                warn!(
                    "未找到可用本地皮肤版本，保持远程皮肤配置: {}",
                    dp_dir.display()
                );
            }
        }
    } else {
        warn!("本地皮肤目录不存在，保持远程皮肤配置: {}", dp_dir.display());
    }

    let re_asset_cdn =
        Regex::new(r#"LOInfo\.assetCDNURL\s*=\s*['\"][^'\"]*['\"]"#).map_err(|e| e.to_string())?;
    html = re_asset_cdn
        .replace_all(&html, format!("LOInfo.assetCDNURL = '{}'", media_base_url))
        .to_string();

    // 6. 将启动脚本改为本地 deps，确保不依赖外网
    let launcher_file_name = "launcher-connector-channel.bundle.js";

    let resolve_local_launcher = || -> Option<PathBuf> {
        let preferred = engine_dir
            .join(resolved_engine_version.as_ref()?)
            .join(launcher_file_name);
        if preferred.exists() {
            return Some(preferred);
        }

        let mut candidates: Vec<PathBuf> = std::fs::read_dir(&engine_dir)
            .ok()?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() && path.join(launcher_file_name).exists())
            .collect();
        candidates.sort();
        candidates.pop().map(|dir| dir.join(launcher_file_name))
    };

    if let Some(local_launcher_path) = resolve_local_launcher() {
        let local_launcher_url = to_exercise_asset_url(&local_launcher_path);
        let re_launcher_script = Regex::new(
            r#"(?is)<script\b[^>]*\bsrc=['\"][^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"][^>]*>\s*</script>"#,
        )
        .map_err(|e| e.to_string())?;
        let replaced_script = re_launcher_script.is_match(&html);
        html = re_launcher_script
            .replace_all(
                &html,
                format!(
                    "<script type=\"text/javascript\" src=\"{}\"></script>",
                    local_launcher_url
                ),
            )
            .to_string();

        // 启动器 URL 也可能出现在内联脚本或数据块中；只改写带引号的远程地址，
        // 避免破坏已经转换完成的 eiuasset:// 地址。
        let re_launcher_url_quoted = Regex::new(
            r#"(?:'(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?'|\"(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?\")"#,
        )
        .map_err(|e| e.to_string())?;
        html = re_launcher_url_quoted
            .replace_all(&html, |caps: &regex::Captures| {
                let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("'");
                let quote = if matched.starts_with('"') { "\"" } else { "'" };
                format!("{quote}{local_launcher_url}{quote}")
            })
            .to_string();

        let re_launcher_tag =
            Regex::new(r#"LOInfo\.launcherTag\s*=\s*['\"][^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"]"#)
                .map_err(|e| e.to_string())?;
        html = re_launcher_tag
            .replace_all(
                &html,
                format!("LOInfo.launcherTag = '{}'", local_launcher_url),
            )
            .to_string();

        if !replaced_script {
            warn!(
                "未在 HTML 中命中 launcher script 标签，已尝试全局 URL 重写: resource_id={}, local_launcher={}",
                resource_id,
                local_launcher_path.display()
            );
        }

        let re_remote_launcher_remaining = Regex::new(
            r#"(?is)<script\b[^>]*\bsrc=['\"](?:(?:https?:)?//)[^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"]|LOInfo\.launcherTag\s*=\s*['\"](?:(?:https?:)?//)[^'\"]*launcher-connector-channel\.bundle\.js(?:\?[^'\"]*)?['\"]|'(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?'|\"(?:(?:https?:)?//)[^'"<>\s]+/buckminster-[^/'"<>\s]+/launcher-connector-channel\.bundle\.js(?:\?[^'"<>\s]*)?\""#,
        )
        .map_err(|e| e.to_string())?;
        if re_remote_launcher_remaining.is_match(&html) {
            warn!(
                "练习 HTML 仍包含远程 launcher 地址，已保留兜底: resource_id={}",
                resource_id
            );
        }
    } else if resolved_engine_version.is_some() {
        error!(
            "未找到本地 launcher 脚本: {:?}",
            engine_dir.join("*/launcher-connector-channel.bundle.js")
        );
    }

    // 7. 同源 URL 环境下不再需要注入 base 标签，移除相关逻辑以避免干扰路径解析

    // 8. 补全 script 标签的 crossorigin，避免生产环境 WebKit 报 opaques script error 并阻止执行
    let re_script_crossorigin =
        Regex::new(r#"(?is)<script\b([^>]*\bsrc=['\"][^'\"]*['\"])([^>]*)>"#).unwrap();
    html = re_script_crossorigin
        .replace_all(&html, |caps: &regex::Captures| {
            let before_src = &caps[1];
            let after_src = &caps[2];
            if before_src.contains("crossorigin=") || after_src.contains("crossorigin=") {
                format!("<script{before_src}{after_src}>")
            } else {
                format!("<script{before_src} crossorigin=\"anonymous\"{after_src}>")
            }
        })
        .to_string();

    // 9. 注入同源布局修复及 URI 容错脚本 (Runtime Guard)
    // 必须在后端注入，确保 eiuasset:// 加载时包含此逻辑
    let runtime_guard_source = include_str!("runtime_guard.js").trim_end_matches('\n');
    let runtime_guard =
        format!("\n<script data-eiu-runtime-guard=\"1\">\n{runtime_guard_source}\n</script>");

    if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, &runtime_guard);
    } else {
        html.push_str(&runtime_guard);
    }

    Ok(html)
}

/// 清理 data.js 中失效的查询参数，并重写媒体 CDN 与协议相对地址。
///
/// # 参数
/// - `content`：原始 data.js 内容。
/// - `media_base_url`：本地媒体资源的基准 URL。
fn sanitize_data_js_media_content(content: &str, media_base_url: &str) -> String {
    let re_encoded =
        Regex::new(r#"\?filename\\u003d[^"\\<>\s]+"#).expect("valid encoded filename regex");
    let tmp = re_encoded.replace_all(content, "");

    let re_plain = Regex::new(r#"\?filename=[^"'<>\s]+"#).expect("valid plain filename regex");
    let tmp = re_plain.replace_all(&tmp, "").into_owned();

    let re_asset_cdn_encoded =
        Regex::new(r#"\\u003cassetCDNURL\\u003emedia\\u003c/assetCDNURL\\u003e"#)
            .expect("valid encoded assetCDNURL regex");
    let encoded_replacement = format!(
        "\\u003cassetCDNURL\\u003e{}\\u003c/assetCDNURL\\u003e",
        media_base_url
    );

    let content = re_asset_cdn_encoded
        .replace_all(&tmp, encoded_replacement.as_str())
        .into_owned();

    // LearningObjectInfo.xml 通常以 Unicode 转义 XML 嵌入 data.js。
    // 在 tauri:// 的 srcdoc 环境中，协议相对地址可能被错误解析为 tauri://host/...，
    // 因此需要同时规范化转义形式和普通 XML 形式。
    let re_protocol_relative_escaped =
        Regex::new(r#"\\u003e//"#).expect("valid escaped xml protocol-relative regex");
    let content = re_protocol_relative_escaped
        .replace_all(&content, "\\u003ehttps://")
        .into_owned();

    let re_protocol_relative_plain =
        Regex::new(r#">//"#).expect("valid plain xml protocol-relative regex");
    re_protocol_relative_plain
        .replace_all(&content, ">https://")
        .into_owned()
}

/// 将脚本字符串中的协议相对 URL 统一转换为 HTTPS。
///
/// # 参数
/// - `content`：需要规范化的脚本文本。
fn normalize_protocol_relative_urls(content: &str) -> String {
    let single_quoted =
        Regex::new(r#"'//([^']+)'"#).expect("valid single-quoted protocol-relative regex");
    let content = single_quoted
        .replace_all(content, "'https://$1'")
        .into_owned();

    let double_quoted =
        Regex::new(r#""//([^"]+)""#).expect("valid double-quoted protocol-relative regex");
    double_quoted
        .replace_all(&content, "\"https://$1\"")
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_protocol_relative_urls, rewrite_exercise_html, sanitize_data_js_media_content,
    };
    use crate::commands::books::application::exercise::source::ExerciseSource;

    #[test]
    fn test_normalize_protocol_relative_urls_keeps_absolute_urls() {
        let input = r#"
            A5.ENGINE_ROOT = "https://author-engine-production.avallain.net/buckminster-34.0.0/";
            A5.SKIN_URL = '//author-assets-runtime-prod-cup.avallain.net/designpack/RDP_Base/abc/';
            LOInfo.toolBeltURL = "//cup-toolbelt-prod.avallain.net";
        "#;

        let output = normalize_protocol_relative_urls(input);

        assert!(
            output.contains("https://author-engine-production.avallain.net/buckminster-34.0.0/")
        );
        assert!(output.contains("A5.SKIN_URL = 'https://author-assets-runtime-prod-cup.avallain.net/designpack/RDP_Base/abc/'"));
        assert!(output.contains("LOInfo.toolBeltURL = \"https://cup-toolbelt-prod.avallain.net\""));
    }

    #[test]
    fn test_sanitize_data_js_media_content_rewrites_asset_cdn() {
        let input = r#"{"xml":"\u003cassetCDNURL\u003emedia\u003c/assetCDNURL\u003e\u003ctoolbeltServer\u003e//cup-toolbelt-prod.avallain.net\u003c/toolbeltServer\u003e"}"#;
        let media_base_url = "eiuasset://localhost/tmp/media";

        let output = sanitize_data_js_media_content(input, media_base_url);

        assert!(output.contains(
            "\\u003cassetCDNURL\\u003eeiuasset://localhost/tmp/media\\u003c/assetCDNURL\\u003e"
        ));
        assert!(output.contains(
            "\\u003ctoolbeltServer\\u003ehttps://cup-toolbelt-prod.avallain.net\\u003c/toolbeltServer\\u003e"
        ));
    }

    #[test]
    fn rewrite_exercise_html_injects_runtime_guard_and_crossorigin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = ExerciseSource {
            html: "<html><head><script src=\"local.js\"></script></head></html>".to_string(),
            data_js_content_raw: None,
            engine_dir: temp_dir.path().join("engine"),
            dp_dir: temp_dir.path().join("dp"),
            engine_base_url: "eiuasset://localhost/engine".to_string(),
            dp_base_url: "eiuasset://localhost/dp".to_string(),
            media_base_url: "eiuasset://localhost/media".to_string(),
            index_url: "eiuasset://localhost/exercise/index.html".to_string(),
        };

        let html = rewrite_exercise_html(source, "exercise-1").unwrap();

        assert!(html.contains("crossorigin=\"anonymous\""));
        assert!(html.contains("<script data-eiu-runtime-guard=\"1\">\n;(function () {"));
    }
}
