//! Data transfer types shared with the UI (mirrored in `src/types/index.ts`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::filesystem::metadata::{category_for, extension_of, EntryKind, FileCategory, FileMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size_logical: u64,
    pub size_allocated: u64,
    /// Unix time in milliseconds.
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub extension: Option<String>,
    pub category: FileCategory,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub is_cloud_placeholder: bool,
}

impl FileEntry {
    pub fn new(path: &Path, meta: &FileMeta) -> Self {
        FileEntry {
            path: path.display().to_string(),
            name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            size_logical: meta.size_logical,
            size_allocated: meta.size_allocated,
            created_at: meta.created_ms,
            modified_at: meta.modified_ms,
            extension: extension_of(path),
            category: category_for(path),
            is_directory: meta.kind == EntryKind::Dir,
            is_symlink: meta.kind == EntryKind::Symlink,
            is_cloud_placeholder: meta.dataless,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    /// Safe to remove: selected by default.
    Safe,
    /// Probably removable, but the user should look first. Never selected by default.
    Review,
    /// Removing it may break something (e.g. a container shared by several apps).
    Danger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCandidate {
    pub path: String,
    pub size: u64,
    pub reason: String,
    pub confidence: Confidence,
    pub selected: bool,
}

/// Result of a destructive operation on one path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationOutcome {
    pub path: String,
    pub size: u64,
    pub ok: bool,
    pub error: Option<String>,
}
