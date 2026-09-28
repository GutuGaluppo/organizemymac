//! Filesystem scanner: the foundation of every module.
//!
//! Folders are listed in parallel on a small, low-priority rayon pool; each listing (names plus
//! `lstat` metadata) is sent as one batch to the calling thread, which hands entries to a
//! [`ScanVisitor`]. A folder's batch is always sent before its subfolders are read, so visitors
//! see parents before children. Each module decides what to keep (aggregated tree, largest files,
//! duplicate candidates…), so the scanner never holds every entry in memory. Symlinks are never
//! followed.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::metadata::{EntryKind, FileMeta};
use super::worker_pool;
use crate::error::{AppError, AppResult};

/// Folders never entered while scanning. They hold other volumes, devices or firmlinked copies of
/// the data volume, which would be counted twice. Scanning inside them explicitly is still allowed.
const SKIP_DIRS: &[&str] = &["/System/Volumes", "/Volumes", "/dev", "/.vol", "/.nofollow", "/.resolve", "/Network"];

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub root: PathBuf,
    /// Paths skipped entirely (user exclusions, ignore list).
    pub exclusions: Vec<PathBuf>,
    /// Do not descend deeper than this (0 = only the root). `None` = unlimited.
    pub max_depth: Option<usize>,
    /// Folders not entered because reading them makes macOS ask for permission or wakes a cloud
    /// provider; each one is reported as a warning. See [`prompting_locations`].
    pub skipped: Vec<(PathBuf, SkipReason)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Other apps' containers: without Full Disk Access, macOS stops the read to ask the user
    /// ("would like to access data from other apps").
    NeedsFullDiskAccess,
    /// Cloud storage (File Provider): listing it can start the provider or download placeholders.
    CloudStorage,
}

/// Locations under the home folder that are skipped unless they are the scan root itself.
pub fn prompting_locations(home: &Path, full_disk_access: bool) -> Vec<(PathBuf, SkipReason)> {
    let mut out = vec![(home.join("Library/CloudStorage"), SkipReason::CloudStorage)];
    if !full_disk_access {
        out.push((home.join("Library/Containers"), SkipReason::NeedsFullDiskAccess));
        out.push((home.join("Library/Group Containers"), SkipReason::NeedsFullDiskAccess));
    }
    out
}

/// One entry handed to the visitor.
pub struct VisitEntry<'a> {
    pub path: &'a Path,
    pub depth: usize,
    pub meta: &'a FileMeta,
    /// False for the 2nd+ occurrence of a hard-linked file: count its size only once.
    pub first_link: bool,
}

pub trait ScanVisitor {
    fn visit(&mut self, entry: &VisitEntry);
}

impl<F: FnMut(&VisitEntry)> ScanVisitor for F {
    fn visit(&mut self, entry: &VisitEntry) {
        self(entry)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub files: u64,
    pub directories: u64,
    pub bytes_scanned: u64,
    pub current: String,
    pub elapsed_ms: u64,
    pub warnings: u64,
    /// No folder listing arrived for a while: macOS is holding a read, usually while a permission
    /// prompt is on screen.
    pub waiting: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub root: String,
    pub files: u64,
    pub directories: u64,
    /// Sum of logical sizes (hard links counted once).
    pub bytes_scanned: u64,
    /// Sum of allocated sizes (hard links counted once, cloud placeholders as 0).
    pub bytes_allocated: u64,
    pub symlinks: u64,
    pub cloud_placeholders: u64,
    pub warnings: u64,
    pub duration_ms: u64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ScanEvent {
    Start { root: String },
    Progress(ScanProgress),
    Warning { path: String, message: String },
    /// A later phase of a module (hashing, sizing apps…): `done` of `total` units.
    Stage { stage: String, done: u64, total: u64 },
}

/// The listing of one folder.
struct Batch {
    dir: PathBuf,
    depth: usize,
    entries: Vec<(OsString, Option<FileMeta>)>,
    error: Option<std::io::Error>,
}

/// At most this many warnings are sent as events; the rest are only counted.
const MAX_WARNING_EVENTS: u64 = 200;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);
const STALL_NOTICE: Duration = Duration::from_secs(3);

struct WalkContext {
    cancel: Arc<AtomicBool>,
    skip: Vec<PathBuf>,
    max_depth: Option<usize>,
}

fn read_dir_parallel<'s>(scope: &rayon::Scope<'s>, ctx: &'s WalkContext, tx: SyncSender<Batch>, dir: PathBuf, depth: usize) {
    if ctx.cancel.load(Ordering::Relaxed) {
        return;
    }
    let mut subdirs = Vec::new();
    let batch = match std::fs::read_dir(&dir) {
        Err(error) => Batch { dir, depth, entries: Vec::new(), error: Some(error) },
        Ok(read) => {
            let mut entries = Vec::new();
            for item in read.flatten() {
                let name = item.file_name();
                let path = dir.join(&name);
                if ctx.skip.contains(&path) {
                    continue;
                }
                let meta = FileMeta::read(&path).ok();
                if meta.as_ref().is_some_and(|m| m.kind == EntryKind::Dir) && ctx.max_depth.is_none_or(|m| depth < m) {
                    subdirs.push(path);
                }
                entries.push((name, meta));
            }
            Batch { dir, depth, entries, error: None }
        }
    };
    // The batch goes out before any subfolder is read: parents always arrive first.
    if tx.send(batch).is_err() {
        return; // the consumer stopped (cancelled)
    }
    for sub in subdirs {
        let tx = tx.clone();
        scope.spawn(move |s| read_dir_parallel(s, ctx, tx, sub, depth + 1));
    }
}

/// Serial walk for scans started from inside the worker pool (e.g. sizing many apps in parallel):
/// waiting on the pool from one of its own threads could leave no thread free to do the work.
fn read_dir_serial(ctx: &WalkContext, tx: SyncSender<Batch>, root: PathBuf) {
    let mut stack = vec![(root, 1usize)];
    while let Some((dir, depth)) = stack.pop() {
        if ctx.cancel.load(Ordering::Relaxed) {
            return;
        }
        let batch = match std::fs::read_dir(&dir) {
            Err(error) => Batch { dir, depth, entries: Vec::new(), error: Some(error) },
            Ok(read) => {
                let mut entries = Vec::new();
                for item in read.flatten() {
                    let name = item.file_name();
                    let path = dir.join(&name);
                    if ctx.skip.contains(&path) {
                        continue;
                    }
                    let meta = FileMeta::read(&path).ok();
                    if meta.as_ref().is_some_and(|m| m.kind == EntryKind::Dir) && ctx.max_depth.is_none_or(|m| depth < m) {
                        stack.push((path, depth + 1));
                    }
                    entries.push((name, meta));
                }
                Batch { dir, depth, entries, error: None }
            }
        };
        // Parents are sent before their subfolders are popped, so the order guarantee holds.
        if tx.send(batch).is_err() {
            return;
        }
    }
}

pub fn scan(
    options: &ScanOptions,
    cancel: &Arc<AtomicBool>,
    visitor: &mut dyn ScanVisitor,
    mut on_event: impl FnMut(ScanEvent),
) -> AppResult<ScanStats> {
    let started = Instant::now();
    let root = options.root.clone();
    let root_meta = FileMeta::read(&root).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => AppError::NotFound(root.display().to_string()),
        _ => AppError::Io(e),
    })?;
    if !root_meta.is_dir() {
        return Err(AppError::Invalid(format!("{} is not a folder", root.display())));
    }
    on_event(ScanEvent::Start { root: root.display().to_string() });

    let mut warnings_sent = 0u64;
    let skipped: Vec<PathBuf> = options
        .skipped
        .iter()
        .filter(|(p, _)| p.starts_with(&root) && *p != root)
        .map(|(p, reason)| {
            let message = match reason {
                SkipReason::NeedsFullDiskAccess => "not scanned: needs Full Disk Access",
                SkipReason::CloudStorage => "not scanned: cloud storage",
            };
            warnings_sent += 1;
            on_event(ScanEvent::Warning { path: p.display().to_string(), message: message.into() });
            p.clone()
        })
        .collect();

    let ctx = Arc::new(WalkContext {
        cancel: cancel.clone(),
        skip: SKIP_DIRS
            .iter()
            .map(PathBuf::from)
            .filter(|s| !root.starts_with(s))
            .chain(options.exclusions.iter().cloned())
            .chain(skipped.iter().cloned())
            .collect(),
        max_depth: options.max_depth,
    });

    let mut stats = ScanStats {
        root: root.display().to_string(),
        files: 0,
        directories: 1,
        bytes_scanned: 0,
        bytes_allocated: 0,
        symlinks: 0,
        cloud_placeholders: 0,
        warnings: warnings_sent,
        duration_ms: 0,
        cancelled: false,
    };
    visitor.visit(&VisitEntry { path: &root, depth: 0, meta: &root_meta, first_link: true });

    // Bounded channel: if the consumer is slower than the disk, workers wait instead of piling up
    // listings in memory.
    let (tx, rx) = sync_channel::<Batch>(512);
    let walker = {
        let ctx = ctx.clone();
        let root = root.clone();
        let descend = options.max_depth.is_none_or(|m| m > 0);
        let nested = rayon::current_thread_index().is_some();
        std::thread::spawn(move || {
            if !descend {
                return;
            }
            if nested {
                read_dir_serial(&ctx, tx, root);
            } else {
                worker_pool().install(|| rayon::scope(|s| read_dir_parallel(s, &ctx, tx, root, 1)));
            }
        })
    };

    let mut seen_links: HashSet<(u64, u64)> = HashSet::new();
    let mut last_progress = Instant::now();
    let mut path = PathBuf::new();

    let mut last_batch = Instant::now();
    let mut waiting = false;
    'batches: loop {
        let batch = match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(b) => b,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if !waiting && last_batch.elapsed() >= STALL_NOTICE {
                    waiting = true;
                    on_event(ScanEvent::Progress(ScanProgress {
                        files: stats.files,
                        directories: stats.directories,
                        bytes_scanned: stats.bytes_scanned,
                        current: String::new(),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                        warnings: stats.warnings,
                        waiting: true,
                    }));
                }
                continue;
            }
        };
        last_batch = Instant::now();
        if let Some(err) = batch.error {
            stats.warnings += 1;
            if stats.warnings <= MAX_WARNING_EVENTS {
                on_event(ScanEvent::Warning { path: batch.dir.display().to_string(), message: warning_message(&err) });
            }
            continue;
        }
        for (name, meta) in &batch.entries {
            if cancel.load(Ordering::Relaxed) {
                break 'batches;
            }
            path.clear();
            path.push(&batch.dir);
            path.push(name);
            let Some(meta) = meta else {
                stats.warnings += 1;
                continue;
            };
            let mut first_link = true;
            if meta.is_file() && meta.nlink > 1 {
                first_link = seen_links.insert((meta.dev, meta.ino));
            }
            match meta.kind {
                EntryKind::Dir => stats.directories += 1,
                EntryKind::File => stats.files += 1,
                EntryKind::Symlink => stats.symlinks += 1,
                EntryKind::Other => {}
            }
            if meta.dataless {
                stats.cloud_placeholders += 1;
            }
            if first_link {
                stats.bytes_scanned += meta.size_logical;
                stats.bytes_allocated += meta.size_allocated;
            }
            visitor.visit(&VisitEntry { path: &path, depth: batch.depth, meta, first_link });
        }
        if last_progress.elapsed() >= PROGRESS_INTERVAL {
            last_progress = Instant::now();
            on_event(ScanEvent::Progress(ScanProgress {
                files: stats.files,
                directories: stats.directories,
                bytes_scanned: stats.bytes_scanned,
                current: batch.dir.display().to_string(),
                elapsed_ms: started.elapsed().as_millis() as u64,
                warnings: stats.warnings,
                waiting: false,
            }));
            waiting = false;
        }
    }
    // Dropping the receiver makes pending sends fail, so workers stop after a cancel. A worker
    // stuck in the kernel (waiting on a permission prompt) cannot be interrupted: do not wait for
    // it after a cancel, it ends on its own once macOS answers.
    drop(rx);
    if !cancel.load(Ordering::Relaxed) {
        let _ = walker.join();
    }

    stats.cancelled = cancel.load(Ordering::Relaxed);
    stats.duration_ms = started.elapsed().as_millis() as u64;
    Ok(stats)
}

fn warning_message(err: &std::io::Error) -> String {
    match err.kind() {
        // EPERM from privacy protection (TCC) or EACCES from folder permissions.
        std::io::ErrorKind::PermissionDenied => "permission denied".into(),
        _ if err.raw_os_error() == Some(libc::EPERM) => "permission denied".into(),
        _ => err.to_string(),
    }
}
