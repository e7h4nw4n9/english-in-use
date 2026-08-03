//! Engine 与 Design Pack 资源路径的本地版本回退映射。

use super::*;

/// 优先使用原路径，并在版本目录不匹配时尝试 Engine 与 Design Pack 回退。
///
/// # 参数
/// - `path`：目标文件或目录路径。
pub(super) fn resolve_path_with_engine_fallback(path: &Path) -> Option<PathBuf> {
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

/// 在已安装 Engine 版本中寻找与请求后缀匹配的资源。
///
/// # 参数
/// - `path`：目标文件或目录路径。
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

/// 在已安装 Design Pack 目录中寻找与请求后缀匹配的资源。
///
/// # 参数
/// - `path`：目标文件或目录路径。
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

/// 将 Engine 请求路径拆分为依赖根、版本和资源后缀。
///
/// # 参数
/// - `path`：目标文件或目录路径。
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

/// 将 Design Pack 请求路径拆分为依赖根和资源后缀。
///
/// # 参数
/// - `path`：目标文件或目录路径。
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

/// 为资源后缀生成兼容旧目录层级的候选形式。
///
/// # 参数
/// - `suffix`：请求资源在依赖目录内的相对后缀。
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

/// 移除 Engine 版本文本中的附加描述以便比较。
///
/// # 参数
/// - `version`：Engine 版本文本。
fn normalize_engine_version(version: &str) -> String {
    version
        .split_whitespace()
        .collect::<String>()
        .to_ascii_lowercase()
}
