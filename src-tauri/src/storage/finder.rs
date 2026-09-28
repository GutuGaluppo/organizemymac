//! Large & Old Files: collects files matching size, age, extension and type filters.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::filesystem::metadata::{category_for, extension_of, FileCategory};
use crate::filesystem::scanner::{ScanVisitor, VisitEntry};
use crate::types::FileEntry;

const DAY_MS: i64 = 86_400_000;

/// Bundles whose inner files are not listed one by one (removing a piece breaks the whole).
const PACKAGE_EXTENSIONS: &[&str] = &[
    "app", "photoslibrary", "musiclibrary", "tvlibrary", "fcpbundle", "logicx", "band", "framework",
    "bundle", "plugin", "kext", "xcarchive", "sparsebundle", "pvm", "vmwarevm", "utm", "photolibrary",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DateField {
    #[default]
    Modified,
    Created,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileFilter {
    pub min_size: u64,
    /// Only files not modified (or created) in this many days.
    pub older_than_days: Option<u32>,
    pub date_field: DateField,
    /// Lowercase, without the dot. Empty = any.
    pub extensions: Vec<String>,
    /// Empty = any.
    pub categories: Vec<FileCategory>,
    /// Skip ~/Library (app data, caches: better handled by other modules).
    pub exclude_library: bool,
    pub include_hidden: bool,
}

pub fn inside_package(path: &Path) -> bool {
    let mut comps = path.components().collect::<Vec<_>>();
    comps.pop(); // the file itself may be a package-like name
    comps.iter().any(|c| match c {
        Component::Normal(n) => {
            let n = n.to_string_lossy();
            n.rsplit_once('.').is_some_and(|(_, ext)| PACKAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        }
        _ => false,
    })
}

fn hidden(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .any(|c| matches!(c, Component::Normal(n) if n.to_string_lossy().starts_with('.')))
}

struct BySize(FileEntry);
impl PartialEq for BySize {
    fn eq(&self, o: &Self) -> bool {
        self.0.size_logical == o.0.size_logical
    }
}
impl Eq for BySize {}
impl PartialOrd for BySize {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for BySize {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.size_logical.cmp(&o.0.size_logical)
    }
}

pub struct FileFinder {
    filter: FileFilter,
    root: PathBuf,
    library: Option<PathBuf>,
    now_ms: i64,
    limit: usize,
    heap: BinaryHeap<Reverse<BySize>>,
    pub matched: u64,
    pub matched_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinderResult {
    /// Named `matches` (not `files`) because it is flattened next to the scan's file count.
    pub matches: Vec<FileEntry>,
    pub matched: u64,
    pub matched_bytes: u64,
    /// More files matched than are listed (the largest ones are kept).
    pub truncated: bool,
}

impl FileFinder {
    pub fn new(filter: FileFilter, root: &Path, home: &Path, now_ms: i64, limit: usize) -> Self {
        let mut filter = filter;
        filter.extensions = filter.extensions.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect();
        FileFinder {
            library: filter.exclude_library.then(|| home.join("Library")),
            filter,
            root: root.to_path_buf(),
            now_ms,
            limit,
            heap: BinaryHeap::new(),
            matched: 0,
            matched_bytes: 0,
        }
    }

    fn matches(&self, path: &Path, entry: &VisitEntry) -> bool {
        let m = entry.meta;
        if !m.is_file() || !entry.first_link || m.size_logical < self.filter.min_size {
            return false;
        }
        if let Some(days) = self.filter.older_than_days {
            let t = match self.filter.date_field {
                DateField::Modified => m.modified_ms,
                DateField::Created => m.created_ms,
            };
            match t {
                Some(t) if self.now_ms - t >= days as i64 * DAY_MS => {}
                _ => return false,
            }
        }
        if !self.filter.extensions.is_empty() && !extension_of(path).is_some_and(|e| self.filter.extensions.contains(&e)) {
            return false;
        }
        if !self.filter.categories.is_empty() && !self.filter.categories.contains(&category_for(path)) {
            return false;
        }
        if self.library.as_ref().is_some_and(|l| path.starts_with(l)) {
            return false;
        }
        if !self.filter.include_hidden && hidden(path, &self.root) {
            return false;
        }
        !inside_package(path)
    }

    pub fn finish(self) -> FinderResult {
        let truncated = self.matched as usize > self.heap.len();
        let mut files: Vec<FileEntry> = self.heap.into_iter().map(|Reverse(BySize(e))| e).collect();
        files.sort_by_key(|e| Reverse(e.size_logical));
        FinderResult { matches: files, matched: self.matched, matched_bytes: self.matched_bytes, truncated }
    }
}

impl ScanVisitor for FileFinder {
    fn visit(&mut self, e: &VisitEntry) {
        if !self.matches(e.path, e) {
            return;
        }
        self.matched += 1;
        self.matched_bytes += e.meta.size_logical;
        if self.heap.len() >= self.limit {
            if self.heap.peek().is_some_and(|Reverse(min)| e.meta.size_logical <= min.0.size_logical) {
                return;
            }
            self.heap.pop();
        }
        self.heap.push(Reverse(BySize(FileEntry::new(e.path, e.meta))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packages_are_not_split() {
        assert!(inside_package(Path::new("/Users/x/Pictures/Photos Library.photoslibrary/originals/a.heic")));
        assert!(inside_package(Path::new("/Applications/Xcode.app/Contents/Resources/x.dmg")));
        assert!(!inside_package(Path::new("/Users/x/Downloads/Setup.app.zip")));
        assert!(!inside_package(Path::new("/Users/x/Movies/film.mov")));
    }

    #[test]
    fn hidden_is_relative_to_root() {
        assert!(hidden(Path::new("/Users/x/.cache/big.bin"), Path::new("/Users/x")));
        assert!(!hidden(Path::new("/Users/x/.cache/big.bin"), Path::new("/Users/x/.cache")));
    }
}
