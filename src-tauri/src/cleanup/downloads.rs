//! Downloads cleanup: classifies each top-level item of ~/Downloads. Old installers come first
//! (the app they installed is usually already in /Applications), then large files, archives,
//! screenshots and old files. Only old installers are selected by default.

use std::path::Path;

use serde::Serialize;

use crate::filesystem::metadata::{extension_of, FileCategory};
use crate::types::{Confidence, FileEntry};

const DAY_MS: i64 = 86_400_000;
pub const OLD_INSTALLER_DAYS: i64 = 30;
pub const OLD_FILE_DAYS: i64 = 180;
pub const LARGE_BYTES: u64 = 500_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadGroup {
    OldInstaller,
    LargeFile,
    Archive,
    Installer,
    Screenshot,
    OldFile,
    Recent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    #[serde(flatten)]
    pub entry: FileEntry,
    pub group: DownloadGroup,
    pub age_days: Option<i64>,
    /// An archive whose contents are already next to it (a folder with the same name).
    pub extracted: bool,
    pub confidence: Confidence,
    pub selected: bool,
}

fn is_installer(entry: &FileEntry) -> bool {
    matches!(entry.category, FileCategory::Installer | FileCategory::DiskImage)
        || entry.extension.as_deref() == Some("xip")
}

fn is_screenshot(entry: &FileEntry) -> bool {
    let name = entry.name.to_lowercase();
    entry.category == FileCategory::Image
        && ["screenshot", "screen shot", "captura de tela", "bildschirmfoto", "captura de pantalla", "capture d’écran", "capture d'écran"]
            .iter()
            .any(|p| name.starts_with(p))
}

/// Age from the most recent of modified/created: a download keeps the server's modification date,
/// but its creation date is when it arrived.
fn age_days(entry: &FileEntry, now_ms: i64) -> Option<i64> {
    let t = entry.created_at.into_iter().chain(entry.modified_at).max()?;
    Some(((now_ms - t) / DAY_MS).max(0))
}

pub fn classify(entry: FileEntry, extracted: bool, now_ms: i64) -> DownloadItem {
    let age = age_days(&entry, now_ms);
    let old = |days| age.is_some_and(|a| a >= days);
    let (group, confidence, selected) = if is_installer(&entry) && old(OLD_INSTALLER_DAYS) {
        (DownloadGroup::OldInstaller, Confidence::Safe, true)
    } else if entry.size_logical >= LARGE_BYTES {
        (DownloadGroup::LargeFile, Confidence::Review, false)
    } else if entry.category == FileCategory::Archive || entry.extension.as_deref() == Some("xip") {
        (DownloadGroup::Archive, if extracted { Confidence::Safe } else { Confidence::Review }, false)
    } else if is_installer(&entry) {
        (DownloadGroup::Installer, Confidence::Review, false)
    } else if is_screenshot(&entry) {
        (DownloadGroup::Screenshot, Confidence::Review, false)
    } else if old(OLD_FILE_DAYS) {
        (DownloadGroup::OldFile, Confidence::Review, false)
    } else {
        (DownloadGroup::Recent, Confidence::Review, false)
    };
    DownloadItem { entry, group, age_days: age, extracted, confidence, selected }
}

/// Whether `archive` sits next to a folder with the same stem ("x.zip" next to "x/").
pub fn looks_extracted(archive: &Path, sibling_dirs: &[String]) -> bool {
    if extension_of(archive).is_none() {
        return false;
    }
    let name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = name.trim_end_matches(".tar.gz").trim_end_matches(".tgz");
    let stem = stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(stem);
    sibling_dirs.iter().any(|d| d == stem)
}

/// Ranked for review: groups in order, largest first inside each group.
pub fn rank(items: &mut [DownloadItem]) {
    items.sort_by(|a, b| a.group.cmp(&b.group).then(b.entry.size_logical.cmp(&a.entry.size_logical)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::metadata::category_for;

    const NOW: i64 = 1_800_000_000_000;

    fn entry(name: &str, size: u64, age_days: i64) -> FileEntry {
        let path = format!("/Users/x/Downloads/{name}");
        let t = NOW - age_days * DAY_MS;
        FileEntry {
            name: name.into(),
            size_logical: size,
            size_allocated: size,
            created_at: Some(t),
            modified_at: Some(t - 400 * DAY_MS),
            extension: extension_of(Path::new(&path)),
            category: category_for(Path::new(&path)),
            path,
            is_directory: false,
            is_symlink: false,
            is_cloud_placeholder: false,
        }
    }

    #[test]
    fn old_installers_are_safe_and_selected() {
        let item = classify(entry("Figma.dmg", 200_000_000, 45), false, NOW);
        assert_eq!(item.group, DownloadGroup::OldInstaller);
        assert!(item.selected);
        assert_eq!(item.confidence, Confidence::Safe);
        // Uses the arrival date (created), not the server's modification date.
        let recent = classify(entry("Figma.dmg", 200_000_000, 3), false, NOW);
        assert_eq!(recent.group, DownloadGroup::Installer);
        assert!(!recent.selected);
    }

    #[test]
    fn groups_and_ranking() {
        let mut items = vec![
            classify(entry("notes.txt", 10, 2), false, NOW),
            classify(entry("movie.mov", 2_000_000_000, 2), false, NOW),
            classify(entry("Screenshot 2026-01-01 at 10.00.png", 300_000, 10), false, NOW),
            classify(entry("project.zip", 50_000_000, 10), true, NOW),
            classify(entry("report.pdf", 1_000_000, 400), false, NOW),
            classify(entry("Xcode_26.xip", 11_000_000_000, 90), false, NOW),
        ];
        rank(&mut items);
        let groups: Vec<_> = items.iter().map(|i| i.group).collect();
        assert_eq!(
            groups,
            vec![
                DownloadGroup::OldInstaller,
                DownloadGroup::LargeFile,
                DownloadGroup::Archive,
                DownloadGroup::Screenshot,
                DownloadGroup::OldFile,
                DownloadGroup::Recent
            ]
        );
        assert_eq!(items[2].confidence, Confidence::Safe); // extracted archive
        assert!(items.iter().filter(|i| i.selected).count() == 1);
    }

    #[test]
    fn extracted_archives() {
        let dirs = vec!["project".to_string(), "other".to_string()];
        assert!(looks_extracted(Path::new("/d/project.zip"), &dirs));
        assert!(looks_extracted(Path::new("/d/project.tar.gz"), &dirs));
        assert!(!looks_extracted(Path::new("/d/missing.zip"), &dirs));
    }
}
