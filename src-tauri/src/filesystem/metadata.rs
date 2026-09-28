use std::fs::Metadata;
use std::os::macos::fs::MetadataExt as MacMetadataExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

/// `SF_DATALESS`: the file is a cloud placeholder (iCloud Drive, File Provider). Its contents are not
/// on disk, so it takes no local space even though it reports a logical size.
const SF_DATALESS: u32 = 0x4000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
    Other,
}

/// Metadata gathered for every entry during a scan. Symlinks are never followed: a symlink is
/// reported as itself, with its own (tiny) size.
#[derive(Debug, Clone)]
pub struct FileMeta {
    pub kind: EntryKind,
    pub size_logical: u64,
    /// Bytes actually allocated on disk (`st_blocks * 512`). Smaller than the logical size for
    /// sparse files, zero for cloud placeholders. APFS clones are still counted in full.
    pub size_allocated: u64,
    pub modified_ms: Option<i64>,
    pub created_ms: Option<i64>,
    pub dev: u64,
    pub ino: u64,
    pub nlink: u64,
    pub dataless: bool,
}

impl FileMeta {
    pub fn from_metadata(m: &Metadata) -> Self {
        let ft = m.file_type();
        let kind = if ft.is_symlink() {
            EntryKind::Symlink
        } else if ft.is_dir() {
            EntryKind::Dir
        } else if ft.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        let dataless = m.st_flags() & SF_DATALESS != 0;
        FileMeta {
            kind,
            // Folders and symlinks hold no data of their own: a symlink's "size" is just the length
            // of its target path.
            size_logical: if matches!(kind, EntryKind::Dir | EntryKind::Symlink) { 0 } else { m.len() },
            size_allocated: if dataless || kind == EntryKind::Symlink { 0 } else { m.blocks() * 512 },
            modified_ms: to_ms(m.modified().ok()),
            created_ms: to_ms(m.created().ok()),
            dev: m.dev(),
            ino: m.ino(),
            nlink: m.nlink(),
            dataless,
        }
    }

    pub fn read(path: &Path) -> std::io::Result<Self> {
        std::fs::symlink_metadata(path).map(|m| Self::from_metadata(&m))
    }

    pub fn is_file(&self) -> bool {
        self.kind == EntryKind::File
    }

    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Dir
    }
}

fn to_ms(t: Option<std::time::SystemTime>) -> Option<i64> {
    let d = t?.duration_since(UNIX_EPOCH).ok()?;
    Some(d.as_millis() as i64)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileCategory {
    Image,
    Video,
    Audio,
    Document,
    Archive,
    DiskImage,
    Installer,
    Code,
    Application,
    Other,
}

pub fn extension_of(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_lowercase())
}

/// Coarse category from the extension. Used for filters and for the Downloads groups.
pub fn category_for(path: &Path) -> FileCategory {
    let Some(ext) = extension_of(path) else { return FileCategory::Other };
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "heic" | "heif" | "webp" | "tif" | "tiff" | "bmp" | "raw"
        | "cr2" | "cr3" | "nef" | "arw" | "dng" | "svg" | "psd" => FileCategory::Image,
        "mov" | "mp4" | "m4v" | "avi" | "mkv" | "webm" | "wmv" | "mpg" | "mpeg" => FileCategory::Video,
        "mp3" | "m4a" | "aac" | "wav" | "aiff" | "aif" | "flac" | "ogg" | "caf" => FileCategory::Audio,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "pages" | "numbers" | "key"
        | "txt" | "rtf" | "md" | "csv" | "epub" => FileCategory::Document,
        "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" => FileCategory::Archive,
        "dmg" | "iso" | "img" | "sparseimage" | "sparsebundle" => FileCategory::DiskImage,
        "pkg" | "mpkg" => FileCategory::Installer,
        "app" => FileCategory::Application,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "swift" | "py" | "go" | "java" | "kt" | "c" | "h"
        | "cpp" | "json" | "yml" | "yaml" | "toml" | "html" | "css" => FileCategory::Code,
        _ => FileCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_from_extension() {
        assert_eq!(category_for(Path::new("/a/b/Setup.DMG")), FileCategory::DiskImage);
        assert_eq!(category_for(Path::new("x.pkg")), FileCategory::Installer);
        assert_eq!(category_for(Path::new("x.tar.gz")), FileCategory::Archive);
        assert_eq!(category_for(Path::new("Screenshot 2026.png")), FileCategory::Image);
        assert_eq!(category_for(Path::new("noext")), FileCategory::Other);
    }
}
