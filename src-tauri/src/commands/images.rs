use std::path::PathBuf;

use base64::Engine;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;

use super::scan::{record, scan_options, validate_root, ScanResult};
use super::{start_job, JobEvent, JobOutcome};
use crate::db::now_ms;
use crate::error::{AppError, AppResult};
use crate::filesystem::metadata::{category_for, FileCategory};
use crate::filesystem::scanner::scan;
use crate::images::{self, find_similar, ImageCandidates, PhotoMember, Sensitivity, SimilarGroup, VisionStats};
use crate::state::AppState;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarResult {
    #[serde(flatten)]
    pub scan: ScanResult,
    pub groups: Vec<SimilarGroup>,
    pub images: u64,
    pub truncated: bool,
    pub vision: VisionStats,
    pub reclaimable: u64,
}

#[tauri::command]
pub fn start_similar_images(app: AppHandle, root: String, sensitivity: Sensitivity, on_event: Channel<JobEvent<SimilarResult>>) -> AppResult<String> {
    let root = validate_root(&root)?;
    Ok(start_job(&app, on_event, "similar-images", move |state, cancel, emit| {
        let started_at = now_ms();
        let mut candidates = ImageCandidates::new();
        let stats = scan(&scan_options(state, &root), cancel, &mut candidates, &mut *emit)?;
        let empty = |stats| ScanResult { id: uuid::Uuid::new_v4().to_string(), started_at, finished_at: now_ms(), stats, reclaimable_bytes: 0, largest_files: Vec::new(), tree: None };
        let (groups, vision, cancelled) = if stats.cancelled {
            (Vec::new(), VisionStats::default(), true)
        } else {
            match find_similar(&candidates, sensitivity, cancel, emit) {
                Ok((g, v)) => (g, v, false),
                Err(AppError::Cancelled) => (Vec::new(), VisionStats::default(), true),
                Err(e) => return Err(e),
            }
        };
        let reclaimable = groups.iter().map(|g| g.reclaimable).sum();
        let mut scan = empty(stats);
        scan.stats.cancelled = cancelled;
        scan.reclaimable_bytes = reclaimable;
        record(state, "similarImages", &scan)?;
        Ok(JobOutcome {
            cancelled,
            result: SimilarResult { scan, groups, images: candidates.files.len() as u64, truncated: candidates.truncated, vision, reclaimable },
        })
    }))
}

/// A small JPEG preview of an image file, as a data URL.
#[tauri::command]
pub async fn image_thumbnail(state: tauri::State<'_, AppState>, path: String) -> AppResult<Option<String>> {
    let p = PathBuf::from(&path);
    if !p.is_absolute() || category_for(&p) != FileCategory::Image || !p.is_file() {
        return Err(AppError::Invalid("not an image file".into()));
    }
    let cache = state.paths.data_dir.join("thumbnails");
    let bytes = tauri::async_runtime::spawn_blocking(move || images::thumbnail_jpeg(&p, &cache)).await.map_err(|e| AppError::Other(e.to_string()))?;
    Ok(bytes.map(|b| format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(b))))
}

#[tauri::command]
pub async fn photos_status(request: bool) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || images::photos_status(request)).await.map_err(|e| AppError::Other(e.to_string()))?
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarPhotosResult {
    pub groups: Vec<Vec<PhotoMember>>,
    pub vision: VisionStats,
}

#[tauri::command]
pub fn start_similar_photos(app: AppHandle, sensitivity: Sensitivity, on_event: Channel<JobEvent<SimilarPhotosResult>>) -> AppResult<String> {
    Ok(start_job(&app, on_event, "similar-photos", move |_state, cancel, emit| {
        emit(crate::filesystem::scanner::ScanEvent::Start { root: "Fotos".into() });
        match images::similar_photos(sensitivity, cancel, emit) {
            Ok((groups, vision)) => Ok(JobOutcome { cancelled: false, result: SimilarPhotosResult { groups, vision } }),
            Err(AppError::Cancelled) => Ok(JobOutcome { cancelled: true, result: SimilarPhotosResult { groups: Vec::new(), vision: VisionStats::default() } }),
            Err(e) => Err(e),
        }
    }))
}

#[tauri::command]
pub async fn photo_thumbnail(state: tauri::State<'_, AppState>, id: String) -> AppResult<Option<String>> {
    if id.is_empty() || id.len() > 200 || id.contains("..") {
        return Err(AppError::Invalid("invalid photo id".into()));
    }
    let cache = state.paths.data_dir.join("thumbnails/photos");
    tauri::async_runtime::spawn_blocking(move || -> AppResult<Option<String>> {
        std::fs::create_dir_all(&cache)?;
        let out = cache.join(format!("{}.jpg", &blake3::hash(id.as_bytes()).to_hex()[..24]));
        if !out.exists() {
            let mut child = std::process::Command::new(images::helper_path())
                .arg("photos-thumbnail")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .spawn()?;
            use std::io::Write;
            child.stdin.take().expect("stdin").write_all(serde_json::json!({ "id": id, "out": out }).to_string().as_bytes())?;
            child.wait()?;
        }
        Ok(std::fs::read(&out).ok().map(|b| format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(b))))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

/// Deletes Photos items. PhotoKit shows macOS's own confirmation; they go to Recently Deleted.
#[tauri::command]
pub async fn delete_photos(ids: Vec<String>) -> AppResult<bool> {
    if ids.is_empty() {
        return Ok(false);
    }
    tauri::async_runtime::spawn_blocking(move || images::delete_photos(&ids)).await.map_err(|e| AppError::Other(e.to_string()))?
}
