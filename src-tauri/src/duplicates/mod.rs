//! Duplicate Finder.
//!
//! ```text
//! files → size groups → sample hash (first/middle/last 64 KB) → full BLAKE3 → duplicate groups
//! ```
//!
//! Only files that survive each stage are read further, so most files are never hashed at all.
//! Hard links (same device and inode) are one file with several names, never "duplicates" of each
//! other. Cloud placeholders are skipped: reading them would download them.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::filesystem::metadata::FileMeta;
use crate::filesystem::scanner::{ScanEvent, ScanVisitor, VisitEntry};
use crate::filesystem::worker_pool;
use crate::storage::finder::inside_package;
use crate::types::FileEntry;

pub const SAMPLE: u64 = 64 * 1024;

/// A file and the other names it has (hard links).
type Named = (Candidate, Vec<String>);
/// Files of one group, each with the hash computed at the current stage.
type Hashed = Vec<([u8; 32], Named)>;

#[derive(Debug, Clone)]
struct Candidate {
    path: PathBuf,
    meta: FileMeta,
}

/// Collects files eligible for duplicate analysis during the scan.
pub struct DuplicateCandidates {
    min_size: u64,
    files: Vec<Candidate>,
}

impl DuplicateCandidates {
    pub fn new(min_size: u64) -> Self {
        DuplicateCandidates { min_size: min_size.max(1), files: Vec::new() }
    }
}

impl ScanVisitor for DuplicateCandidates {
    fn visit(&mut self, e: &VisitEntry) {
        let m = e.meta;
        if m.is_file() && !m.dataless && m.size_logical >= self.min_size && !inside_package(e.path) {
            self.files.push(Candidate { path: e.path.to_path_buf(), meta: m.clone() });
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    #[serde(flatten)]
    pub entry: FileEntry,
    /// Other names of the same file (hard links). Removing one name frees nothing.
    pub hard_links: Vec<String>,
    /// Default selection: true for every copy except the one to keep.
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub hash: String,
    pub size: u64,
    pub files: Vec<DuplicateFile>,
    /// Space freed by keeping one copy.
    pub wasted: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashStats {
    pub candidates: u64,
    pub after_size: u64,
    pub sample_hashed: u64,
    pub full_hashed: u64,
    pub bytes_hashed: u64,
    pub hash_ms: u64,
}

fn read_at(file: &mut File, offset: u64, len: u64, buf: &mut Vec<u8>) -> std::io::Result<()> {
    buf.clear();
    file.seek(SeekFrom::Start(offset))?;
    file.take(len).read_to_end(buf)?;
    Ok(())
}

/// Hash of the first, middle and last 64 KB (the whole file when it is small).
pub fn sample_hash(path: &Path, size: u64) -> std::io::Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = Vec::with_capacity(SAMPLE as usize);
    if size <= SAMPLE * 3 {
        file.read_to_end(&mut buf)?;
        hasher.update(&buf);
    } else {
        for offset in [0, size / 2 - SAMPLE / 2, size - SAMPLE] {
            read_at(&mut file, offset, SAMPLE, &mut buf)?;
            hasher.update(&buf);
        }
    }
    Ok(*hasher.finalize().as_bytes())
}

/// Full BLAKE3 hash, checking `cancel` between 1 MB reads.
pub fn full_hash(path: &Path, cancel: &AtomicBool, hashed: &AtomicU64) -> std::io::Result<Option<[u8; 32]>> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        hashed.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(Some(*hasher.finalize().as_bytes()))
}

/// Several names of one inode are collapsed into the first; the others become `hard_links`.
fn collapse_hard_links(files: Vec<Candidate>) -> Vec<(Candidate, Vec<String>)> {
    let mut by_inode: HashMap<(u64, u64), (Candidate, Vec<String>)> = HashMap::new();
    let mut order = Vec::new();
    for c in files {
        let key = (c.meta.dev, c.meta.ino);
        match by_inode.get_mut(&key) {
            Some((_, links)) => links.push(c.path.display().to_string()),
            None => {
                order.push(key);
                by_inode.insert(key, (c, Vec::new()));
            }
        }
    }
    order.into_iter().filter_map(|k| by_inode.remove(&k)).collect()
}

/// How likely a copy is the one to keep: lower is better.
fn keep_score(path: &Path, meta: &FileMeta, home: &Path) -> (u8, u8, usize, i64) {
    let p = path.display().to_string().to_lowercase();
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let transient = [home.join("Downloads"), home.join(".Trash"), home.join("Library/Caches"), PathBuf::from("/tmp")]
        .iter()
        .any(|d| path.starts_with(d)) as u8;
    let copy_name = ["copy", "cópia", "copia", "kopie", "(1)", "(2)", "(3)", " 2.", " 3."].iter().any(|m| name.contains(m)) as u8;
    let depth = p.matches('/').count();
    (transient, copy_name, depth, meta.created_ms.unwrap_or(i64::MAX))
}

pub struct DuplicateSearch<'a> {
    pub home: &'a Path,
    pub cancel: &'a Arc<AtomicBool>,
}

impl DuplicateSearch<'_> {
    pub fn run(&self, candidates: DuplicateCandidates, emit: &mut dyn FnMut(ScanEvent)) -> AppResult<(Vec<DuplicateGroup>, HashStats)> {
        let started = Instant::now();
        let mut stats = HashStats { candidates: candidates.files.len() as u64, ..Default::default() };

        // 1. Size groups (hard links collapsed first, so one inode never matches itself).
        let mut by_size: HashMap<u64, Vec<Candidate>> = HashMap::new();
        for c in candidates.files {
            by_size.entry(c.meta.size_logical).or_default().push(c);
        }
        let mut size_groups: Vec<Vec<(Candidate, Vec<String>)>> = by_size
            .into_values()
            .map(collapse_hard_links)
            .filter(|g| g.len() > 1)
            .collect();
        size_groups.sort_by_key(|g| std::cmp::Reverse(g[0].0.meta.size_logical));
        stats.after_size = size_groups.iter().map(|g| g.len() as u64).sum();

        // 2. Sample hash.
        let pool = worker_pool();
        let total = stats.after_size;
        let done = AtomicU64::new(0);
        let sampled: Vec<Hashed> = pool.install(|| {
            size_groups
                .into_par_iter()
                .map(|group| {
                    group
                        .into_iter()
                        .filter_map(|(c, links)| {
                            if self.cancel.load(Ordering::Relaxed) {
                                return None;
                            }
                            done.fetch_add(1, Ordering::Relaxed);
                            sample_hash(&c.path, c.meta.size_logical).ok().map(|h| (h, (c, links)))
                        })
                        .collect()
                })
                .collect()
        });
        stats.sample_hashed = done.load(Ordering::Relaxed);
        emit(ScanEvent::Stage { stage: "sample".into(), done: stats.sample_hashed, total });
        if self.cancel.load(Ordering::Relaxed) {
            return Err(AppError::Cancelled);
        }

        let mut full_groups: Vec<Vec<(Candidate, Vec<String>)>> = Vec::new();
        for group in sampled {
            let mut by_sample: HashMap<[u8; 32], Vec<(Candidate, Vec<String>)>> = HashMap::new();
            for (h, c) in group {
                by_sample.entry(h).or_default().push(c);
            }
            full_groups.extend(by_sample.into_values().filter(|g| g.len() > 1));
        }

        // 3. Full hash, only where the samples matched. Files small enough to be read whole by
        // the sample stage are already fully compared.
        let needs_full: u64 = full_groups.iter().filter(|g| g[0].0.meta.size_logical > SAMPLE * 3).map(|g| g.len() as u64).sum();
        let total_bytes: u64 = full_groups
            .iter()
            .filter(|g| g[0].0.meta.size_logical > SAMPLE * 3)
            .map(|g| g[0].0.meta.size_logical * g.len() as u64)
            .sum();
        let hashed = AtomicU64::new(0);
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let mut groups: Vec<DuplicateGroup> = Vec::new();
        std::thread::scope(|scope| -> AppResult<()> {
            let hashed_ref = &hashed;
            let worker = scope.spawn(move || {
                let r: Vec<Hashed> = pool.install(|| {
                    full_groups
                        .into_par_iter()
                        .map(|group| {
                            let small = group[0].0.meta.size_logical <= SAMPLE * 3;
                            group
                                .into_par_iter()
                                .filter_map(|(c, links)| {
                                    let h = if small {
                                        // The sample stage already read the whole file: re-hash in full (cheap).
                                        let mut buf = Vec::new();
                                        File::open(&c.path).and_then(|mut f| f.read_to_end(&mut buf)).ok()?;
                                        Some(*blake3::hash(&buf).as_bytes())
                                    } else {
                                        full_hash(&c.path, self.cancel, hashed_ref).ok().flatten()
                                    };
                                    h.map(|h| (h, (c, links)))
                                })
                                .collect()
                        })
                        .collect()
                });
                let _ = tx.send(());
                r
            });
            // Progress while hashing.
            loop {
                match rx.recv_timeout(std::time::Duration::from_millis(250)) {
                    Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(_) => emit(ScanEvent::Stage { stage: "full".into(), done: hashed.load(Ordering::Relaxed), total: total_bytes }),
                }
            }
            let hashed_groups = worker.join().map_err(|_| AppError::Other("hashing failed".into()))?;
            if self.cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
            for group in hashed_groups {
                let mut by_hash: HashMap<[u8; 32], Vec<(Candidate, Vec<String>)>> = HashMap::new();
                for (h, c) in group {
                    by_hash.entry(h).or_default().push(c);
                }
                for (hash, copies) in by_hash.into_iter().filter(|(_, g)| g.len() > 1) {
                    groups.push(self.build_group(hash, copies));
                }
            }
            Ok(())
        })?;

        stats.full_hashed = needs_full;
        stats.bytes_hashed = hashed.load(Ordering::Relaxed);
        stats.hash_ms = started.elapsed().as_millis() as u64;
        groups.sort_by_key(|g| std::cmp::Reverse(g.wasted));
        tracing::info!(?stats, groups = groups.len(), "duplicate search finished");
        Ok((groups, stats))
    }

    fn build_group(&self, hash: [u8; 32], copies: Vec<(Candidate, Vec<String>)>) -> DuplicateGroup {
        let size = copies[0].0.meta.size_logical;
        let keep = copies
            .iter()
            .enumerate()
            .min_by_key(|(_, (c, _))| keep_score(&c.path, &c.meta, self.home))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let files: Vec<DuplicateFile> = copies
            .into_iter()
            .enumerate()
            .map(|(i, (c, links))| DuplicateFile { entry: FileEntry::new(&c.path, &c.meta), hard_links: links, selected: i != keep })
            .collect();
        DuplicateGroup { hash: blake3::Hash::from_bytes(hash).to_hex().to_string(), size, wasted: size * (files.len() as u64 - 1), files }
    }
}

/// A removal request from the duplicates screen: per group, every copy the user saw and the ones
/// to remove. Validated so a group can never lose all its copies.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRemoval {
    pub all: Vec<String>,
    pub remove: Vec<RemoveCopy>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveCopy {
    pub path: String,
    pub size: u64,
    /// Modification time seen in the scan; the file must be unchanged to be removed.
    pub modified_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GroupRuleError {
    #[error("a duplicate group must keep at least one copy")]
    NoCopyKept,
    #[error("the copy to remove is not part of the group")]
    NotInGroup,
    #[error("no kept copy of this group exists any more")]
    KeptCopyMissing,
}

/// At least one copy that is not being removed must still exist on disk.
pub fn validate_group(g: &GroupRemoval) -> Result<(), GroupRuleError> {
    if g.remove.iter().any(|r| !g.all.contains(&r.path)) {
        return Err(GroupRuleError::NotInGroup);
    }
    let kept: Vec<&String> = g.all.iter().filter(|p| !g.remove.iter().any(|r| &r.path == *p)).collect();
    if kept.is_empty() {
        return Err(GroupRuleError::NoCopyKept);
    }
    if !kept.iter().any(|p| Path::new(p).exists()) {
        return Err(GroupRuleError::KeptCopyMissing);
    }
    Ok(())
}

/// The file still has the size and modification time seen in the scan.
pub fn unchanged(copy: &RemoveCopy) -> bool {
    FileMeta::read(Path::new(&copy.path)).is_ok_and(|m| m.size_logical == copy.size && m.modified_ms == copy.modified_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_prefers_originals() {
        let home = Path::new("/Users/x");
        let meta = FileMeta::read(Path::new("/bin/ls")).unwrap();
        let a = keep_score(Path::new("/Users/x/Pictures/photo.jpg"), &meta, home);
        let b = keep_score(Path::new("/Users/x/Downloads/photo.jpg"), &meta, home);
        let c = keep_score(Path::new("/Users/x/Pictures/photo copy.jpg"), &meta, home);
        assert!(a < b && a < c);
    }

    #[test]
    fn group_rules() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let s = |p: &PathBuf| p.display().to_string();
        let rm = |p: &PathBuf| RemoveCopy { path: s(p), size: 1, modified_at: None };
        let all = vec![s(&a), s(&b)];
        assert_eq!(validate_group(&GroupRemoval { all: all.clone(), remove: vec![rm(&a), rm(&b)] }), Err(GroupRuleError::NoCopyKept));
        assert!(validate_group(&GroupRemoval { all: all.clone(), remove: vec![rm(&a)] }).is_ok());
        let other = dir.path().join("c");
        assert_eq!(validate_group(&GroupRemoval { all: all.clone(), remove: vec![rm(&other)] }), Err(GroupRuleError::NotInGroup));
        std::fs::remove_file(&b).unwrap();
        assert_eq!(validate_group(&GroupRemoval { all, remove: vec![rm(&a)] }), Err(GroupRuleError::KeptCopyMissing));
    }
}
