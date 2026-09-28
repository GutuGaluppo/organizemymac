//! Cloud storage overview: how much of each synced folder is downloaded on this Mac, and which
//! files could be made cloud-only. Only iCloud Drive files are evicted by the app (public
//! `FileManager.evictUbiquitousItem` API, through the Swift helper); other providers are pointed to
//! their own "online-only" option.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::filesystem::scanner::{scan, ScanEvent, ScanOptions, VisitEntry};
use crate::types::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    ICloud,
    GoogleDrive,
    Dropbox,
    OneDrive,
    Box,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudFolder {
    pub provider: Provider,
    pub name: String,
    pub path: String,
    /// The app can evict downloaded files itself (iCloud Drive only).
    pub can_evict: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudUsage {
    #[serde(flatten)]
    pub folder: CloudFolder,
    pub files: u64,
    pub total_bytes: u64,
    /// Bytes stored on this Mac.
    pub local_bytes: u64,
    pub cloud_only_files: u64,
    /// Largest downloaded files (candidates for "Remove Download").
    pub largest_local: Vec<FileEntry>,
    pub cancelled: bool,
}

fn provider_of(name: &str) -> Provider {
    let n = name.to_lowercase();
    if n.starts_with("googledrive") {
        Provider::GoogleDrive
    } else if n.starts_with("dropbox") {
        Provider::Dropbox
    } else if n.starts_with("onedrive") {
        Provider::OneDrive
    } else if n.starts_with("box") {
        Provider::Box
    } else {
        Provider::Other
    }
}

/// Synced folders on this Mac (no provider is started: only the top-level folders are listed).
pub fn folders(home: &Path) -> Vec<CloudFolder> {
    let mut out = Vec::new();
    let icloud = home.join("Library/Mobile Documents/com~apple~CloudDocs");
    if icloud.is_dir() {
        out.push(CloudFolder { provider: Provider::ICloud, name: "iCloud Drive".into(), path: icloud.display().to_string(), can_evict: true });
    }
    if let Ok(read) = std::fs::read_dir(home.join("Library/CloudStorage")) {
        for e in read.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if e.path().is_dir() && !name.starts_with('.') {
                let label = name.replace('-', " · ");
                out.push(CloudFolder { provider: provider_of(&name), name: label, path: e.path().display().to_string(), can_evict: false });
            }
        }
    }
    let legacy = home.join("Dropbox");
    if legacy.is_dir() && !out.iter().any(|f| f.provider == Provider::Dropbox) {
        out.push(CloudFolder { provider: Provider::Dropbox, name: "Dropbox".into(), path: legacy.display().to_string(), can_evict: false });
    }
    out
}

pub fn usage(folder: &CloudFolder, cancel: &Arc<AtomicBool>, emit: &mut dyn FnMut(ScanEvent)) -> AppResult<CloudUsage> {
    let (mut files, mut total, mut local, mut cloud_only) = (0u64, 0u64, 0u64, 0u64);
    let mut heap: BinaryHeap<Reverse<(u64, String)>> = BinaryHeap::new();
    let mut metas = std::collections::HashMap::new();
    let stats = scan(&ScanOptions { root: PathBuf::from(&folder.path), ..Default::default() }, cancel, &mut |e: &VisitEntry| {
        if !e.meta.is_file() || !e.first_link {
            return;
        }
        files += 1;
        total += e.meta.size_logical;
        if e.meta.dataless {
            cloud_only += 1;
            return;
        }
        local += e.meta.size_allocated;
        if e.meta.size_logical >= 1_000_000 {
            let key = e.path.display().to_string();
            heap.push(Reverse((e.meta.size_logical, key.clone())));
            metas.insert(key, (e.path.to_path_buf(), e.meta.clone()));
            if heap.len() > 200 {
                if let Some(Reverse((_, k))) = heap.pop() {
                    metas.remove(&k);
                }
            }
        }
    }, emit)?;
    let mut largest: Vec<FileEntry> = heap.into_iter().filter_map(|Reverse((_, k))| metas.get(&k).map(|(p, m)| FileEntry::new(p, m))).collect();
    largest.sort_by_key(|f| Reverse(f.size_logical));
    Ok(CloudUsage { folder: folder.clone(), files, total_bytes: total, local_bytes: local, cloud_only_files: cloud_only, largest_local: largest, cancelled: stats.cancelled || cancel.load(Ordering::Relaxed) })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvictOutcome {
    pub path: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Asks iCloud to remove the local copies (the files stay in iCloud and download again on open).
/// Only files inside iCloud Drive are accepted.
pub fn evict(home: &Path, paths: &[String]) -> AppResult<Vec<EvictOutcome>> {
    let root = home.join("Library/Mobile Documents");
    let (ok, rejected): (Vec<&String>, Vec<&String>) = paths.iter().partition(|p| {
        let p = Path::new(p.as_str());
        p.is_absolute() && p.starts_with(&root) && !p.components().any(|c| matches!(c, std::path::Component::ParentDir)) && p.is_file()
    });
    let mut out: Vec<EvictOutcome> = rejected.into_iter().map(|p| EvictOutcome { path: p.clone(), ok: false, error: Some("only files in iCloud Drive can be evicted".into()) }).collect();
    if !ok.is_empty() {
        use std::io::Write;
        let mut child = std::process::Command::new(crate::images::helper_path())
            .arg("evict")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        child.stdin.take().expect("stdin").write_all(serde_json::json!({ "paths": ok }).to_string().as_bytes())?;
        let output = child.wait_with_output()?;
        let parsed: Vec<EvictOutcome> = serde_json::from_slice(&output.stdout).map_err(|e| AppError::Other(format!("unexpected helper output: {e}")))?;
        out.extend(parsed);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_and_folders() {
        assert_eq!(provider_of("GoogleDrive-me@example.com"), Provider::GoogleDrive);
        assert_eq!(provider_of("OneDrive-Personal"), Provider::OneDrive);
        assert_eq!(provider_of("Dropbox"), Provider::Dropbox);
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        std::fs::create_dir_all(home.join("Library/Mobile Documents/com~apple~CloudDocs")).unwrap();
        std::fs::create_dir_all(home.join("Library/CloudStorage/GoogleDrive-me@example.com")).unwrap();
        let f = folders(home);
        assert_eq!(f.len(), 2);
        assert!(f[0].can_evict && !f[1].can_evict);
    }

    #[test]
    fn eviction_only_inside_icloud() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        std::fs::write(home.join("outside.txt"), b"x").unwrap();
        let out = evict(home, &[home.join("outside.txt").display().to_string()]).unwrap();
        assert!(!out[0].ok);
    }
}
