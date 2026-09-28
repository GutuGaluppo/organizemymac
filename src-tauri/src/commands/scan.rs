use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;

use super::{start_job, JobEvent, JobOutcome};
use crate::db::{now_ms, ScanRecord};
use crate::error::{AppError, AppResult};
use crate::filesystem::scanner::{prompting_locations, scan, ScanOptions, ScanStats};
use crate::filesystem::{probe_full_disk_access, AccessStatus};
use crate::state::{AppState, ScanSession};
use crate::storage::largest::LargestFiles;
use crate::storage::tree::{StorageNode, StorageTree};
use crate::storage::Fanout;
use crate::types::FileEntry;

/// Files from 1 MB up get their own node in the storage tree.
const MIN_FILE_NODE: u64 = 1_000_000;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub id: String,
    pub started_at: i64,
    pub finished_at: i64,
    #[serde(flatten)]
    pub stats: ScanStats,
    pub reclaimable_bytes: u64,
    pub largest_files: Vec<FileEntry>,
    /// The scanned folder with its direct children, largest first.
    pub tree: Option<StorageNode>,
}

pub fn validate_root(root: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(root);
    if !path.is_absolute() {
        return Err(AppError::Invalid("the folder must be an absolute path".into()));
    }
    Ok(path)
}

/// Scans a folder: totals, the largest files and the aggregated folder tree. Never modifies files.
#[tauri::command]
pub fn start_scan(app: AppHandle, root: String, on_event: Channel<JobEvent<ScanResult>>) -> AppResult<String> {
    let root = validate_root(&root)?;
    Ok(start_job(&app, on_event, "scan", move |state, cancel, emit| {
        let started_at = now_ms();
        let options = scan_options(state, &root);
        let mut tree = StorageTree::new(&root, MIN_FILE_NODE);
        let mut largest = LargestFiles::new(100);
        let stats = {
            let mut fan = Fanout(vec![&mut tree, &mut largest]);
            scan(&options, cancel, &mut fan, emit)?
        };
        tree.finish();
        let id = uuid::Uuid::new_v4().to_string();
        let result = ScanResult {
            id: id.clone(),
            started_at,
            finished_at: now_ms(),
            reclaimable_bytes: 0,
            largest_files: largest.into_sorted(),
            tree: tree.root_id().and_then(|r| tree.view(r, 1, 60)),
            stats: stats.clone(),
        };
        record(state, "scanner", &result)?;
        state.store_session(ScanSession { id, root, tree });
        Ok(JobOutcome { cancelled: stats.cancelled, result })
    }))
}

/// Scan options with the ignore list and the folders that would make macOS prompt.
pub fn scan_options(state: &AppState, root: &std::path::Path) -> ScanOptions {
    let fda = probe_full_disk_access(&state.paths.home) == AccessStatus::Granted;
    ScanOptions {
        root: root.to_path_buf(),
        exclusions: state.ignored_paths(),
        max_depth: None,
        skipped: prompting_locations(&state.paths.home, fda),
    }
}

pub fn record(state: &AppState, module: &str, r: &ScanResult) -> AppResult<()> {
    tracing::info!(module, root = %r.stats.root, files = r.stats.files, ms = r.stats.duration_ms, cancelled = r.stats.cancelled, "scan finished");
    state.db.lock().unwrap().insert_scan(&ScanRecord {
        id: r.id.clone(),
        module: module.into(),
        root: r.stats.root.clone(),
        started_at: r.started_at,
        finished_at: Some(r.finished_at),
        files: r.stats.files,
        directories: r.stats.directories,
        bytes_scanned: r.stats.bytes_scanned,
        bytes_allocated: r.stats.bytes_allocated,
        reclaimable_bytes: r.reclaimable_bytes,
        warnings: r.stats.warnings,
        duration_ms: r.stats.duration_ms,
        cancelled: r.stats.cancelled,
    })
}

/// A node of a cached scan tree with `depth` levels of children (Space Map navigation).
#[tauri::command]
pub fn storage_node(
    state: tauri::State<'_, AppState>,
    scan_id: String,
    path: Option<String>,
    depth: Option<u32>,
    limit: Option<usize>,
) -> AppResult<StorageNode> {
    let session = state.session(&scan_id).ok_or_else(|| AppError::NotFound("scan (run it again)".into()))?;
    let session = session.lock().unwrap();
    let id = match path {
        Some(p) => session.tree.find(&PathBuf::from(&p)).ok_or_else(|| AppError::NotFound(p))?,
        None => session.tree.root_id().ok_or_else(|| AppError::NotFound("empty scan".into()))?,
    };
    let limit = limit.unwrap_or(200).clamp(1, 400);
    session.tree.view(id, depth.unwrap_or(1).min(3), limit).ok_or_else(|| AppError::NotFound("node".into()))
}
