//! 页面、图书资产和练习包的本地优先资源解析。

use crate::models::BookSource;
use crate::services::book_metadata::MetadataService;
use log::info;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime, State};

use super::metadata::get_book_metadata;
use super::overlay::{extract_container_code_from_overlay, load_book_overlay_config};
use super::paths::{
    bundle_books_base_paths, bundle_courses_base_paths, file_exists_and_non_empty,
    normalize_safe_relative_path, path_to_slash_string, push_unique_path, write_bytes_to_path,
};
use super::{DownloadBatchOptions, DownloadProgress, download_r2_objects_concurrently};
use super::{ERR_OVERLAY_INVALID_STRUCTURE, ERR_RESOURCE_NOT_FOUND, coded_error};

mod book_asset;
mod exercise;
mod page;

pub use book_asset::resolve_book_asset;
pub use exercise::resolve_exercise_resource;
pub(super) use exercise::resolve_exercise_resource_internal;
pub use page::resolve_page_resource;
