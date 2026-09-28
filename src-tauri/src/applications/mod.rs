//! Application Manager: installed apps, their metadata and size.

pub mod leftovers;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use rayon::prelude::*;
use serde::Serialize;

use crate::filesystem::metadata::FileMeta;
use crate::filesystem::scanner::{scan, ScanOptions, VisitEntry};
use crate::filesystem::worker_pool;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub path: String,
    pub name: String,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
    pub build: Option<String>,
    pub minimum_system: Option<String>,
    pub size: u64,
    pub modified_at: Option<i64>,
    /// From Spotlight (`kMDItemLastUsedDate`), when available.
    pub last_used_at: Option<i64>,
    pub from_app_store: bool,
    /// Apple apps that ship with macOS: protected by the system, never offered for removal.
    pub system_app: bool,
    pub running: bool,
    pub icon_file: Option<String>,
}

/// Info.plist fields.
#[derive(Debug, Default)]
pub struct BundleInfo {
    pub bundle_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub build: Option<String>,
    pub minimum_system: Option<String>,
    pub icon_file: Option<String>,
    pub executable: Option<String>,
}

pub fn read_bundle(app: &Path) -> BundleInfo {
    let Ok(value) = plist::Value::from_file(app.join("Contents/Info.plist")) else { return BundleInfo::default() };
    let Some(dict) = value.as_dictionary() else { return BundleInfo::default() };
    let s = |k: &str| dict.get(k).and_then(|v| v.as_string()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    BundleInfo {
        bundle_id: s("CFBundleIdentifier"),
        name: s("CFBundleDisplayName").or_else(|| s("CFBundleName")),
        version: s("CFBundleShortVersionString"),
        build: s("CFBundleVersion"),
        minimum_system: s("LSMinimumSystemVersion"),
        icon_file: s("CFBundleIconFile").or_else(|| s("CFBundleIconName")),
        executable: s("CFBundleExecutable"),
    }
}

/// `.app` bundles in a folder and one level of subfolders (e.g. /Applications/Utilities,
/// "Adobe Photoshop 2025/"), without entering the bundles.
pub fn find_bundles(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(read) = std::fs::read_dir(root) else { return out };
    for entry in read.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if !is_dir {
            continue;
        }
        if path.extension().is_some_and(|e| e == "app") {
            out.push(path);
        } else if let Ok(sub) = std::fs::read_dir(&path) {
            out.extend(
                sub.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "app") && p.is_dir()),
            );
        }
    }
    out
}

/// Last used dates for many bundles with a single `mdls` call.
fn last_used(paths: &[PathBuf]) -> HashMap<PathBuf, i64> {
    let mut out = HashMap::new();
    if paths.is_empty() {
        return out;
    }
    let Ok(output) = Command::new("/usr/bin/mdls").args(["-raw", "-name", "kMDItemLastUsedDate"]).args(paths).output() else {
        return out;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    for (path, value) in paths.iter().zip(text.split('\0')) {
        // "2026-09-27 21:14:03 +0000" or "(null)"
        if let Ok(t) = chrono::DateTime::parse_from_str(value.trim(), "%Y-%m-%d %H:%M:%S %z") {
            out.insert(path.clone(), t.timestamp_millis());
        }
    }
    out
}

pub fn bundle_size(app: &Path, cancel: &Arc<AtomicBool>) -> u64 {
    let mut size = 0u64;
    let _ = scan(&ScanOptions { root: app.to_path_buf(), ..Default::default() }, cancel, &mut |e: &VisitEntry| {
        if e.first_link {
            size += e.meta.size_logical;
        }
    }, |_| {});
    size
}

/// Every installed app in /Applications and ~/Applications with its size. `progress(done, total)`
/// is called as sizes are computed.
pub fn list_apps(home: &Path, running: &[PathBuf], cancel: &Arc<AtomicBool>, progress: &(dyn Fn(u64, u64) + Sync)) -> Vec<AppInfo> {
    let mut bundles = find_bundles(Path::new("/Applications"));
    bundles.extend(find_bundles(&home.join("Applications")));
    bundles.sort();
    bundles.dedup();
    let used = last_used(&bundles);
    let total = bundles.len() as u64;
    let done = AtomicU64::new(0);
    let mut apps: Vec<AppInfo> = worker_pool().install(|| {
        bundles
            .par_iter()
            .map(|path| {
                let info = read_bundle(path);
                let meta = FileMeta::read(path).ok();
                let size = if cancel.load(Ordering::Relaxed) { 0 } else { bundle_size(path, cancel) };
                progress(done.fetch_add(1, Ordering::Relaxed) + 1, total);
                let from_app_store = path.join("Contents/_MASReceipt").exists();
                let system_app = info.bundle_id.as_deref().is_some_and(|id| id.starts_with("com.apple.")) && !from_app_store;
                AppInfo {
                    name: info
                        .name
                        .clone()
                        .unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()),
                    path: path.display().to_string(),
                    bundle_id: info.bundle_id,
                    version: info.version,
                    build: info.build,
                    minimum_system: info.minimum_system,
                    size,
                    modified_at: meta.and_then(|m| m.modified_ms),
                    last_used_at: used.get(path).copied(),
                    from_app_store,
                    system_app,
                    running: running.iter().any(|exe| exe.starts_with(path)),
                    icon_file: info.icon_file,
                }
            })
            .collect()
    });
    apps.sort_by_key(|a| std::cmp::Reverse(a.size));
    apps
}

/// Executable paths of running processes.
pub fn running_executables() -> Vec<PathBuf> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_exe(UpdateKind::Always));
    sys.processes().values().filter_map(|p| p.exe().map(Path::to_path_buf)).collect()
}

pub fn is_running(app: &Path) -> bool {
    running_executables().iter().any(|exe| exe.starts_with(app))
}

/// App icon as a 64 px PNG, converted once with `sips` and cached by bundle path and mtime.
pub fn icon_png(app: &Path, cache_dir: &Path) -> Option<Vec<u8>> {
    let info = read_bundle(app);
    let resources = app.join("Contents/Resources");
    let icns = info
        .icon_file
        .map(|f| if f.ends_with(".icns") { f } else { format!("{f}.icns") })
        .map(|f| resources.join(f))
        .filter(|p| p.exists())
        .or_else(|| {
            std::fs::read_dir(&resources)
                .ok()?
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|e| e == "icns"))
        })?;
    let stamp = FileMeta::read(&icns).ok()?.modified_ms.unwrap_or(0);
    let key = format!("{:x}-{stamp}.png", blake3::hash(app.display().to_string().as_bytes()).as_bytes()[..8].iter().fold(0u64, |a, b| a << 8 | *b as u64));
    let out = cache_dir.join(key);
    if !out.exists() {
        std::fs::create_dir_all(cache_dir).ok()?;
        let status = Command::new("/usr/bin/sips")
            .args(["-s", "format", "png", "-Z", "64"])
            .arg(&icns)
            .arg("--out")
            .arg(&out)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()?;
        if !status.success() {
            return None;
        }
    }
    std::fs::read(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_system_bundle() {
        let info = read_bundle(Path::new("/System/Applications/Calculator.app"));
        assert_eq!(info.bundle_id.as_deref(), Some("com.apple.calculator"));
        assert!(info.version.is_some());
    }

    #[test]
    fn finds_bundles_one_level_deep() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("A.app/Contents")).unwrap();
        std::fs::create_dir_all(dir.path().join("Suite/B.app/Contents")).unwrap();
        std::fs::create_dir_all(dir.path().join("Suite/Deep/C.app")).unwrap();
        let mut found: Vec<String> = find_bundles(dir.path()).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        found.sort();
        assert_eq!(found, vec!["A.app", "B.app"]);
    }
}
