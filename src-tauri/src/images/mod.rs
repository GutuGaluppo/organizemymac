//! Similar Images: candidates are collected by the Rust scanner, analysed by the Swift helper
//! (thumbnail → perceptual hash pre-filter → Vision feature prints only for likely pairs →
//! clustering), and turned into review groups here.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::filesystem::metadata::{FileCategory, FileMeta};
use crate::filesystem::scanner::{ScanEvent, ScanVisitor, VisitEntry};
use crate::storage::finder::inside_package;
use crate::types::FileEntry;

/// Images smaller than this are icons and UI assets, not photos.
const MIN_IMAGE_BYTES: u64 = 30_000;
pub const MAX_IMAGES: usize = 30_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Sensitivity {
    /// Near-identical: resized, recompressed or converted copies.
    Strict,
    /// Also slightly cropped or edited versions and burst shots.
    Normal,
}

impl Sensitivity {
    /// Vision feature-print distance and dHash Hamming distance (calibrated on resized, recompressed
    /// and cropped copies vs. different pictures of the same style).
    pub fn thresholds(self) -> (f32, u32) {
        match self {
            Sensitivity::Strict => (0.2, 12),
            Sensitivity::Normal => (0.35, 18),
        }
    }
}

/// Path of the helper next to the app's executable (Tauri copies sidecars there), or the build
/// output when running tests and examples.
pub fn helper_path() -> PathBuf {
    let beside = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("organize-helper")));
    match beside {
        Some(p) if p.exists() => p,
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join(concat!("binaries/organize-helper-", env!("ORGANIZE_TARGET_TRIPLE"))),
    }
}

pub struct ImageCandidates {
    pub files: Vec<(PathBuf, FileMeta)>,
    pub truncated: bool,
}

impl ImageCandidates {
    pub fn new() -> Self {
        ImageCandidates { files: Vec::new(), truncated: false }
    }
}

impl Default for ImageCandidates {
    fn default() -> Self {
        Self::new()
    }
}

impl ScanVisitor for ImageCandidates {
    fn visit(&mut self, e: &VisitEntry) {
        let m = e.meta;
        if !m.is_file() || m.dataless || !e.first_link || m.size_logical < MIN_IMAGE_BYTES {
            return;
        }
        if crate::filesystem::metadata::category_for(e.path) != FileCategory::Image || inside_package(e.path) {
            return;
        }
        if self.files.len() >= MAX_IMAGES {
            self.truncated = true;
            return;
        }
        self.files.push((e.path.to_path_buf(), m.clone()));
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HelperMember {
    index: usize,
    id: String,
    width: u32,
    height: u32,
    distance: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HelperOutput {
    groups: Vec<Vec<HelperMember>>,
    analyzed: u64,
    candidates: u64,
    feature_prints: u64,
    failed: u64,
    elapsed_ms: u64,
}

#[derive(Debug, Deserialize)]
struct HelperProgress {
    stage: String,
    done: u64,
    total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarImage {
    #[serde(flatten)]
    pub entry: FileEntry,
    pub width: u32,
    pub height: u32,
    pub distance: f32,
    /// The copy to keep: highest resolution, then largest file, then oldest.
    pub keep: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarGroup {
    pub id: String,
    pub images: Vec<SimilarImage>,
    /// Space freed by keeping only the best image.
    pub reclaimable: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionStats {
    pub analyzed: u64,
    pub candidates: u64,
    pub feature_prints: u64,
    pub failed: u64,
    pub elapsed_ms: u64,
}

/// Runs the helper with `input` on stdin, forwarding its progress lines, and kills it if the job
/// is cancelled.
fn run_helper(args: &[&str], input: &serde_json::Value, cancel: &Arc<AtomicBool>, emit: &mut dyn FnMut(ScanEvent)) -> AppResult<Vec<u8>> {
    let mut child = Command::new(helper_path())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Other(format!("could not start the image helper: {e}")))?;
    let body = serde_json::to_vec(input).map_err(|e| AppError::Other(e.to_string()))?;
    let mut stdin = child.stdin.take().expect("stdin");
    std::thread::spawn(move || {
        let _ = stdin.write_all(&body);
    });
    let stdout = child.stdout.take().expect("stdout");
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = std::io::Read::read_to_end(&mut BufReader::new(stdout), &mut out);
        out
    });
    let (tx, rx) = std::sync::mpsc::channel::<HelperProgress>();
    let stderr = child.stderr.take().expect("stderr");
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Ok(p) = serde_json::from_str::<HelperProgress>(&line) {
                let _ = tx.send(p);
            } else if !line.is_empty() {
                tracing::warn!(helper = %line);
            }
        }
    });
    loop {
        while let Ok(p) = rx.try_recv() {
            emit(ScanEvent::Stage { stage: p.stage, done: p.done, total: p.total });
        }
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::Cancelled);
        }
        match child.try_wait()? {
            Some(status) if status.success() => break,
            Some(status) => return Err(AppError::Other(format!("the image helper failed ({status})"))),
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    Ok(reader.join().unwrap_or_default())
}

fn best_index(images: &[SimilarImage]) -> usize {
    images
        .iter()
        .enumerate()
        .max_by_key(|(_, i)| (i.width as u64 * i.height as u64, i.entry.size_logical, std::cmp::Reverse(i.entry.created_at.unwrap_or(i64::MAX))))
        .map(|(n, _)| n)
        .unwrap_or(0)
}

pub fn find_similar(candidates: &ImageCandidates, sensitivity: Sensitivity, cancel: &Arc<AtomicBool>, emit: &mut dyn FnMut(ScanEvent)) -> AppResult<(Vec<SimilarGroup>, VisionStats)> {
    if candidates.files.len() < 2 {
        return Ok((Vec::new(), VisionStats::default()));
    }
    let (threshold, hash_distance) = sensitivity.thresholds();
    let paths: Vec<String> = candidates.files.iter().map(|(p, _)| p.display().to_string()).collect();
    let input = serde_json::json!({ "paths": paths, "threshold": threshold, "hashDistance": hash_distance });
    let raw = run_helper(&["similar"], &input, cancel, emit)?;
    let out: HelperOutput = serde_json::from_slice(&raw).map_err(|e| AppError::Other(format!("unexpected helper output: {e}")))?;
    let mut groups: Vec<SimilarGroup> = out
        .groups
        .into_iter()
        .map(|members| {
            let mut images: Vec<SimilarImage> = members
                .into_iter()
                .filter_map(|m| {
                    let (path, meta) = candidates.files.get(m.index)?;
                    (path.display().to_string() == m.id).then(|| SimilarImage { entry: FileEntry::new(path, meta), width: m.width, height: m.height, distance: m.distance, keep: false })
                })
                .collect();
            let best = best_index(&images);
            if let Some(b) = images.get_mut(best) {
                b.keep = true;
            }
            images.sort_by_key(|i| (!i.keep, std::cmp::Reverse(i.width as u64 * i.height as u64)));
            let reclaimable = images.iter().filter(|i| !i.keep).map(|i| i.entry.size_logical).sum();
            SimilarGroup { id: images.first().map(|i| i.entry.path.clone()).unwrap_or_default(), images, reclaimable }
        })
        .filter(|g| g.images.len() > 1)
        .collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.reclaimable));
    let stats = VisionStats { analyzed: out.analyzed, candidates: out.candidates, feature_prints: out.feature_prints, failed: out.failed, elapsed_ms: out.elapsed_ms };
    tracing::info!(?stats, groups = groups.len(), "similar images");
    Ok((groups, stats))
}

// MARK: - Photos library (PhotoKit through the helper)

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoMember {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub distance: f32,
    pub keep: bool,
}

pub fn photos_status(request: bool) -> AppResult<String> {
    let mut cmd = Command::new(helper_path());
    cmd.arg("photos-status");
    if request {
        cmd.arg("--request");
    }
    let out = cmd.output()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| AppError::Other(e.to_string()))?;
    Ok(v["status"].as_str().unwrap_or("unknown").to_string())
}

pub fn similar_photos(sensitivity: Sensitivity, cancel: &Arc<AtomicBool>, emit: &mut dyn FnMut(ScanEvent)) -> AppResult<(Vec<Vec<PhotoMember>>, VisionStats)> {
    let (threshold, hash_distance) = sensitivity.thresholds();
    let raw = run_helper(&["photos-similar"], &serde_json::json!({ "threshold": threshold, "hashDistance": hash_distance, "limit": MAX_IMAGES }), cancel, emit)?;
    let out: HelperOutput = serde_json::from_slice(&raw).map_err(|e| AppError::Other(format!("unexpected helper output: {e}")))?;
    let groups = out
        .groups
        .into_iter()
        .map(|g| {
            let best = g.iter().enumerate().max_by_key(|(_, m)| m.width as u64 * m.height as u64).map(|(i, _)| i).unwrap_or(0);
            g.into_iter().enumerate().map(|(i, m)| PhotoMember { id: m.id, width: m.width, height: m.height, distance: m.distance, keep: i == best }).collect()
        })
        .collect();
    Ok((groups, VisionStats { analyzed: out.analyzed, candidates: out.candidates, feature_prints: out.feature_prints, failed: out.failed, elapsed_ms: out.elapsed_ms }))
}

/// Deletes Photos assets through PhotoKit: macOS shows its own confirmation and the photos go to
/// Recently Deleted.
pub fn delete_photos(ids: &[String]) -> AppResult<bool> {
    let never = Arc::new(AtomicBool::new(false));
    let raw = run_helper(&["photos-delete"], &serde_json::json!({ "ids": ids }), &never, &mut |_| {})?;
    let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| AppError::Other(e.to_string()))?;
    if v["ok"] == "true" {
        Ok(true)
    } else {
        Err(AppError::Other(v["error"].as_str().unwrap_or("Photos did not delete the items").to_string()))
    }
}

/// A 320 px JPEG preview, cached by path and modification time.
pub fn thumbnail_jpeg(path: &Path, cache_dir: &Path) -> Option<Vec<u8>> {
    let meta = FileMeta::read(path).ok()?;
    let key = blake3::hash(format!("{}:{}", path.display(), meta.modified_ms.unwrap_or(0)).as_bytes()).to_hex();
    let out = cache_dir.join(format!("{}.jpg", &key[..24]));
    if !out.exists() {
        std::fs::create_dir_all(cache_dir).ok()?;
        let ok = Command::new("/usr/bin/sips")
            .args(["-s", "format", "jpeg", "-s", "formatOptions", "70", "-Z", "320"])
            .arg(path)
            .arg("--out")
            .arg(&out)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?
            .success();
        if !ok {
            return None;
        }
    }
    std::fs::read(out).ok()
}
