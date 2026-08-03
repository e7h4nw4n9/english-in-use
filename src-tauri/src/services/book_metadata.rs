use crate::models::book_metadata::{BookDefinition, BookJson, ExerciseInfo, PageIndex, TocNode};
use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub struct MetadataService;

mod mapping;
mod page_index;
mod parsing;
mod toc;

#[cfg(test)]
mod tests;
