//! The user's Trash (~/.Trash). Emptying it is the one place the app deletes permanently, so it
//! always shows the totals first and requires an explicit confirmation in the UI.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::Serialize;

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::filesystem::metadata::FileMeta;
use crate::filesystem::safety::{RemovalContext, SafetyPolicy};
use crate::filesystem::scanner::{scan, ScanOptions, VisitEntry};
use crate::types::{FileEntry, OperationOutcome};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashSummary {
    pub path: String,
    /// False when macOS does not let the app read the Trash (needs Full Disk Access).
    pub readable: bool,
    pub files: u64,
    pub folders: u64,
    pub bytes: u64,
    /// Top-level items, largest first (sizes include folder contents).
    pub items: Vec<FileEntry>,
}

pub fn summary(trash_dir: &Path) -> AppResult<TrashSummary> {
    let path = trash_dir.display().to_string();
    let top: Vec<PathBuf> = match std::fs::read_dir(trash_dir) {
        Ok(read) => read.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n != ".DS_Store")).collect(),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(libc::EPERM) => {
            return Ok(TrashSummary { path, readable: false, files: 0, folders: 0, bytes: 0, items: Vec::new() });
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TrashSummary { path, readable: true, files: 0, folders: 0, bytes: 0, items: Vec::new() });
        }
        Err(e) => return Err(AppError::Io(e)),
    };

    let (mut files, mut folders, mut bytes) = (0u64, 0u64, 0u64);
    let mut items = Vec::new();
    let never = Arc::new(AtomicBool::new(false));
    for item in top {
        let Ok(meta) = FileMeta::read(&item) else { continue };
        let mut entry = FileEntry::new(&item, &meta);
        if meta.is_dir() {
            let (mut size, mut count, mut dirs) = (0u64, 0u64, 0u64);
            let opts = ScanOptions { root: item.clone(), ..Default::default() };
            let _ = scan(&opts, &never, &mut |e: &VisitEntry| {
                if e.meta.is_dir() {
                    dirs += 1;
                } else if e.first_link {
                    size += e.meta.size_logical;
                    count += 1;
                }
            }, |_| {});
            entry.size_logical = size;
            files += count;
            folders += dirs;
        } else {
            files += 1;
        }
        bytes += entry.size_logical;
        items.push(entry);
    }
    items.sort_by_key(|e| std::cmp::Reverse(e.size_logical));
    Ok(TrashSummary { path, readable: true, files, folders, bytes, items })
}

/// Permanently deletes only the reviewed top-level items of the Trash (Smart Care: items moved to
/// the Trash after the review are never touched).
pub fn empty_only(policy: &SafetyPolicy, db: &Db, trash_dir: &Path, reviewed: &[String]) -> AppResult<Vec<OperationOutcome>> {
    let items = summary(trash_dir)?;
    if !items.readable {
        return Err(AppError::Invalid("the Trash cannot be read without Full Disk Access".into()));
    }
    let wanted: std::collections::HashSet<&str> = reviewed.iter().map(String::as_str).collect();
    Ok(delete_items(policy, db, trash_dir, items.items.iter().filter(|e| wanted.contains(e.path.as_str()))))
}

fn delete_items<'a>(policy: &SafetyPolicy, db: &Db, trash_dir: &Path, entries: impl Iterator<Item = &'a FileEntry>) -> Vec<OperationOutcome> {
    let ctx = RemovalContext { scan_root: Some(trash_dir.to_path_buf()), allow_system_library: false };
    entries
        .map(|entry| {
            let path = Path::new(&entry.path);
            let result = policy.check(path, &ctx).map_err(|v| v.to_string()).and_then(|resolved| {
                let meta = std::fs::symlink_metadata(&resolved).map_err(|e| e.to_string())?;
                if meta.is_dir() { std::fs::remove_dir_all(&resolved) } else { std::fs::remove_file(&resolved) }.map_err(|e| e.to_string())
            });
            let (ok, error) = match result {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e)),
            };
            let _ = db.log_operation("emptyTrash", &entry.path, entry.size_logical, ok, error.as_deref());
            OperationOutcome { path: entry.path.clone(), size: entry.size_logical, ok, error }
        })
        .collect()
}

/// Permanently deletes every top-level item in the Trash. Each one still passes the safety layer
/// (it must be inside the Trash folder after resolving symlinks, not the folder itself, etc.).
pub fn empty(policy: &SafetyPolicy, db: &Db, trash_dir: &Path) -> AppResult<Vec<OperationOutcome>> {
    let items = summary(trash_dir)?;
    if !items.readable {
        return Err(AppError::Invalid("the Trash cannot be read without Full Disk Access".into()));
    }
    Ok(delete_items(policy, db, trash_dir, items.items.iter()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn summary_and_empty() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let trash = root.join(".Trash");
        fs::create_dir_all(trash.join("folder/sub")).unwrap();
        fs::write(trash.join("folder/sub/a.bin"), vec![0u8; 3000]).unwrap();
        fs::write(trash.join("b.txt"), b"hello").unwrap();
        fs::write(root.join("keep.txt"), b"outside").unwrap();
        std::os::unix::fs::symlink(root.join("keep.txt"), trash.join("link")).unwrap();

        let s = summary(&trash).unwrap();
        assert!(s.readable);
        assert_eq!(s.bytes, 3005);
        assert_eq!(s.items[0].name, "folder");
        assert_eq!(s.files, 3); // a.bin, b.txt, link

        let policy = SafetyPolicy::custom(vec![], vec![trash.clone()], vec![]);
        let db = Db::open_in_memory().unwrap();
        let out = empty(&policy, &db, &trash).unwrap();
        assert!(out.iter().all(|o| o.ok), "{out:?}");
        assert_eq!(fs::read_dir(&trash).unwrap().count(), 0);
        // The symlink's target outside the Trash is untouched.
        assert_eq!(fs::read(root.join("keep.txt")).unwrap(), b"outside");
        assert!(trash.exists());
    }

    #[test]
    fn empty_only_touches_reviewed_items() {
        let dir = tempfile::tempdir().unwrap();
        let trash = fs::canonicalize(dir.path()).unwrap().join(".Trash");
        fs::create_dir_all(&trash).unwrap();
        fs::write(trash.join("reviewed.txt"), b"a").unwrap();
        fs::write(trash.join("added-later.txt"), b"b").unwrap();
        let policy = SafetyPolicy::custom(vec![], vec![trash.clone()], vec![]);
        let db = Db::open_in_memory().unwrap();
        let reviewed = vec![trash.join("reviewed.txt").display().to_string()];
        let out = empty_only(&policy, &db, &trash, &reviewed).unwrap();
        assert_eq!(out.len(), 1);
        assert!(!trash.join("reviewed.txt").exists());
        assert!(trash.join("added-later.txt").exists());
    }
}
