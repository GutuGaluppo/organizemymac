//! Files an app leaves in ~/Library, classified by how sure we are they belong to it:
//!
//! ```text
//! exact bundle-id match   → safe
//! known app-owned path    → safe      (rules/apps/known-paths.json)
//! vendor-name match       → review
//! app-name / fuzzy match  → review    (never selected by default)
//! shared group container  → danger
//! ```
//!
//! Nothing outside ~/Library is considered, and a folder is never matched only because its name
//! loosely resembles the app's.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

use rayon::prelude::*;
use serde::Serialize;

use super::read_bundle;
use crate::filesystem::scanner::{scan, ScanOptions, VisitEntry};
use crate::filesystem::worker_pool;
use crate::types::Confidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LeftoverKind {
    ApplicationSupport,
    Caches,
    Preferences,
    Logs,
    SavedState,
    WebKit,
    HttpStorage,
    Cookies,
    Container,
    GroupContainer,
    LaunchAgent,
    ApplicationScripts,
}

pub const LOCATIONS: &[(&str, LeftoverKind)] = &[
    ("Application Support", LeftoverKind::ApplicationSupport),
    ("Caches", LeftoverKind::Caches),
    ("Preferences", LeftoverKind::Preferences),
    ("Logs", LeftoverKind::Logs),
    ("Saved Application State", LeftoverKind::SavedState),
    ("WebKit", LeftoverKind::WebKit),
    ("HTTPStorages", LeftoverKind::HttpStorage),
    ("Cookies", LeftoverKind::Cookies),
    ("Containers", LeftoverKind::Container),
    ("Group Containers", LeftoverKind::GroupContainer),
    ("LaunchAgents", LeftoverKind::LaunchAgent),
    ("Application Scripts", LeftoverKind::ApplicationScripts),
];

impl LeftoverKind {
    /// Reading inside other apps' containers makes macOS prompt unless the app has Full Disk Access.
    fn needs_full_disk_access(self) -> bool {
        matches!(self, LeftoverKind::Container | LeftoverKind::GroupContainer | LeftoverKind::ApplicationScripts)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchRule {
    BundleId,
    KnownPath,
    Vendor,
    AppName,
    Fuzzy,
    SharedGroup,
    /// Named after a bundle id that no installed app has.
    Orphan,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leftover {
    pub path: String,
    pub kind: LeftoverKind,
    pub size: u64,
    pub rule: MatchRule,
    pub confidence: Confidence,
    pub selected: bool,
}

pub struct AppIdentity {
    pub bundle_id: String,
    pub name: String,
}

const SUFFIXES: &[&str] = &[".plist", ".savedState", ".binarycookies", ".sfl2", ".sfl3", ".log"];

fn strip_suffix(name: &str) -> &str {
    SUFFIXES.iter().find_map(|s| name.strip_suffix(s)).unwrap_or(name)
}

fn normalize(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// "com.figma.Desktop" → "figma"; "notion.id" → "notion".
pub fn vendor_of(bundle_id: &str) -> Option<String> {
    const TLDS: &[&str] = &["com", "org", "net", "io", "dev", "app", "co", "me", "us", "de", "uk", "fr", "br", "jp", "ai", "ru", "cn", "nl", "se", "ch", "it", "es"];
    let parts: Vec<&str> = bundle_id.split('.').collect();
    let v = if parts.len() >= 3 && TLDS.contains(&parts[0].to_lowercase().as_str()) { parts[1] } else { parts[0] };
    let v = normalize(v);
    (v.len() >= 3).then_some(v)
}

/// A string that looks like a reverse-DNS bundle id with at least three components.
pub fn looks_like_bundle_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 3
        && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        && parts[0].chars().all(|c| c.is_ascii_lowercase())
        && (2..=6).contains(&parts[0].len())
}

pub type KnownPaths = HashMap<String, Vec<String>>;

pub fn known_paths() -> &'static KnownPaths {
    static KNOWN: OnceLock<KnownPaths> = OnceLock::new();
    KNOWN.get_or_init(|| {
        let raw: HashMap<String, serde_json::Value> = serde_json::from_str(include_str!("../../../rules/apps/known-paths.json")).unwrap_or_default();
        raw.into_iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .filter_map(|(k, v)| Some((k, v.as_array()?.iter().filter_map(|p| p.as_str().map(String::from)).collect())))
            .collect()
    })
}

fn confidence_for(rule: MatchRule) -> Confidence {
    match rule {
        MatchRule::BundleId | MatchRule::KnownPath => Confidence::Safe,
        MatchRule::SharedGroup => Confidence::Danger,
        _ => Confidence::Review,
    }
}

/// True when `name` (without suffix) belongs to another installed app whose id extends this one
/// (e.g. "com.foo.App.Helper" installed separately).
fn owned_by_other(stripped: &str, own_id: &str, installed: &HashSet<String>) -> bool {
    installed.iter().any(|other| other != own_id && other.len() > own_id.len() && other.starts_with(own_id) && (stripped == other || stripped.starts_with(&format!("{other}."))))
}

fn classify_for_app(name: &str, kind: LeftoverKind, app: &AppIdentity, vendor: Option<&str>, installed: &HashSet<String>) -> Option<MatchRule> {
    let id = app.bundle_id.as_str();
    let stripped = strip_suffix(name);
    let lname = name.to_lowercase();
    let lid = id.to_lowercase();
    if kind == LeftoverKind::GroupContainer {
        let hit = lname.contains(&lid) || vendor.is_some_and(|v| lname.contains(&format!(".{v}.")) || lname.contains(&format!(".{v}")));
        return hit.then_some(MatchRule::SharedGroup);
    }
    if (stripped.eq_ignore_ascii_case(id) || lname.starts_with(&format!("{lid}."))) && !owned_by_other(stripped, id, installed) {
        return Some(MatchRule::BundleId);
    }
    // Entries named after a bundle id belong to that id: never match them by name.
    if looks_like_bundle_id(stripped) {
        return None;
    }
    let n = normalize(name);
    if matches!(kind, LeftoverKind::ApplicationSupport | LeftoverKind::Caches | LeftoverKind::Logs) {
        if vendor.is_some_and(|v| n == v) {
            return Some(MatchRule::Vendor);
        }
        let app_name = normalize(&app.name);
        if !app_name.is_empty() && n == app_name {
            return Some(MatchRule::AppName);
        }
        if app_name.len() >= 5 && n.contains(&app_name) {
            return Some(MatchRule::Fuzzy);
        }
    }
    None
}

fn size_of(path: &Path) -> u64 {
    let never = Arc::new(AtomicBool::new(false));
    let meta = std::fs::symlink_metadata(path);
    if meta.as_ref().is_ok_and(|m| !m.is_dir()) {
        return meta.map(|m| m.len()).unwrap_or(0);
    }
    let mut size = 0;
    let _ = scan(&ScanOptions { root: path.to_path_buf(), ..Default::default() }, &never, &mut |e: &VisitEntry| {
        if e.first_link {
            size += e.meta.size_logical;
        }
    }, |_| {});
    size
}

fn with_sizes(found: Vec<(PathBuf, LeftoverKind, MatchRule)>) -> Vec<Leftover> {
    let mut out: Vec<Leftover> = worker_pool().install(|| {
        found
            .into_par_iter()
            .map(|(path, kind, rule)| {
                let confidence = confidence_for(rule);
                Leftover { size: size_of(&path), path: path.display().to_string(), kind, rule, confidence, selected: confidence == Confidence::Safe }
            })
            .collect()
    });
    out.sort_by(|a, b| a.kind.cmp_key().cmp(&b.kind.cmp_key()).then(b.size.cmp(&a.size)));
    out
}

impl LeftoverKind {
    fn cmp_key(self) -> usize {
        LOCATIONS.iter().position(|(_, k)| *k == self).unwrap_or(usize::MAX)
    }
}

/// Files in ~/Library that belong to `app`.
pub fn find_for_app(library: &Path, app: &AppIdentity, installed: &HashSet<String>, full_disk_access: bool) -> Vec<Leftover> {
    let vendor = vendor_of(&app.bundle_id);
    let mut found: Vec<(PathBuf, LeftoverKind, MatchRule)> = Vec::new();
    for (dir, kind) in LOCATIONS {
        if kind.needs_full_disk_access() && !full_disk_access {
            continue;
        }
        let Ok(read) = std::fs::read_dir(library.join(dir)) else { continue };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(rule) = classify_for_app(&name, *kind, app, vendor.as_deref(), installed) {
                found.push((entry.path(), *kind, rule));
            }
        }
    }
    if let Some(paths) = known_paths().get(&app.bundle_id) {
        for rel in paths {
            let p = library.join(rel);
            let kind = LOCATIONS.iter().find(|(d, _)| rel.starts_with(d)).map(|(_, k)| *k).unwrap_or(LeftoverKind::ApplicationSupport);
            if p.exists() {
                // A known path upgrades an earlier, weaker match of the same folder.
                found.retain(|(q, _, _)| *q != p);
                found.push((p, kind, MatchRule::KnownPath));
            }
        }
    }
    with_sizes(found)
}

/// Bundle ids of every app Spotlight knows on this Mac (all volumes it indexes), plus the ones in
/// the standard folders in case Spotlight is off.
pub fn installed_bundle_ids(home: &Path) -> HashSet<String> {
    let mut paths: Vec<PathBuf> = Command::new("/usr/bin/mdfind")
        .arg("kMDItemContentType == 'com.apple.application-bundle'")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(PathBuf::from).collect())
        .unwrap_or_default();
    for dir in ["/Applications", "/System/Applications", "/System/Applications/Utilities", "/System/Library/CoreServices"] {
        paths.extend(super::find_bundles(Path::new(dir)));
    }
    paths.extend(super::find_bundles(&home.join("Applications")));
    worker_pool().install(|| paths.par_iter().filter_map(|p| read_bundle(p).bundle_id).collect())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanGroup {
    pub bundle_id: String,
    /// Another installed app has the same vendor: the files may be a shared component of it.
    pub vendor_installed: bool,
    pub items: Vec<Leftover>,
    pub size: u64,
}

fn orphan_confidence(kind: LeftoverKind) -> Confidence {
    match kind {
        LeftoverKind::Caches | LeftoverKind::Preferences | LeftoverKind::Logs | LeftoverKind::SavedState | LeftoverKind::WebKit | LeftoverKind::HttpStorage | LeftoverKind::Cookies => Confidence::Safe,
        LeftoverKind::GroupContainer => Confidence::Danger,
        _ => Confidence::Review,
    }
}

fn is_installed(id: &str, installed: &HashSet<String>) -> bool {
    let lid = id.to_lowercase();
    installed.iter().any(|i| {
        let li = i.to_lowercase();
        li == lid || lid.starts_with(&format!("{li}.")) || li.starts_with(&format!("{lid}."))
    })
}

/// Leftovers of apps that are no longer installed: entries named after a bundle id that no app on
/// this Mac has. Apple's own ids are skipped (they belong to system components).
pub fn find_orphans(library: &Path, installed: &HashSet<String>, full_disk_access: bool) -> Vec<OrphanGroup> {
    let mut found: Vec<(String, PathBuf, LeftoverKind)> = Vec::new();
    for (dir, kind) in LOCATIONS {
        if kind.needs_full_disk_access() && !full_disk_access {
            continue;
        }
        let Ok(read) = std::fs::read_dir(library.join(dir)) else { continue };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let id = if *kind == LeftoverKind::GroupContainer {
                // "TEAMID.com.vendor.app" or "group.com.vendor.app"
                name.split_once('.').map(|(_, rest)| rest.to_string()).unwrap_or_default()
            } else {
                strip_suffix(&name).to_string()
            };
            if !looks_like_bundle_id(&id) || id.starts_with("com.apple.") || is_installed(&id, installed) {
                continue;
            }
            found.push((id, entry.path(), *kind));
        }
    }
    let installed_vendors: HashSet<String> = installed.iter().filter_map(|i| vendor_of(i)).collect();
    let items = with_sizes(found.iter().map(|(_, p, k)| (p.clone(), *k, MatchRule::Orphan)).collect());
    let mut groups: HashMap<String, OrphanGroup> = HashMap::new();
    for item in items {
        let id = found.iter().find(|(_, p, _)| p.display().to_string() == item.path).map(|(i, _, _)| i.clone()).unwrap_or_default();
        let vendor_installed = vendor_of(&id).is_some_and(|v| installed_vendors.contains(&v));
        let mut confidence = orphan_confidence(item.kind);
        if vendor_installed && confidence == Confidence::Safe {
            confidence = Confidence::Review;
        }
        let item = Leftover { confidence, selected: confidence == Confidence::Safe, ..item };
        let g = groups.entry(id.clone()).or_insert(OrphanGroup { bundle_id: id, vendor_installed, items: Vec::new(), size: 0 });
        g.size += item.size;
        g.items.push(item);
    }
    let mut out: Vec<OrphanGroup> = groups.into_values().collect();
    out.sort_by_key(|g| std::cmp::Reverse(g.size));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn app() -> AppIdentity {
        AppIdentity { bundle_id: "com.figma.Desktop".into(), name: "Figma".into() }
    }

    #[test]
    fn vendors_and_ids() {
        assert_eq!(vendor_of("com.figma.Desktop").as_deref(), Some("figma"));
        assert_eq!(vendor_of("notion.id").as_deref(), Some("notion"));
        assert_eq!(vendor_of("us.zoom.xos").as_deref(), Some("zoom"));
        assert!(looks_like_bundle_id("com.spotify.client"));
        assert!(!looks_like_bundle_id("Google"));
        assert!(!looks_like_bundle_id("com.spotify"));
        assert!(!looks_like_bundle_id("My.Folder.Name"));
    }

    #[test]
    fn classification_rules() {
        let installed: HashSet<String> = ["com.figma.Desktop".into(), "com.figma.Desktop.agent".into()].into_iter().collect();
        let v = Some("figma");
        let c = |name: &str, kind| classify_for_app(name, kind, &app(), v, &installed);
        assert_eq!(c("com.figma.Desktop.plist", LeftoverKind::Preferences), Some(MatchRule::BundleId));
        assert_eq!(c("com.figma.Desktop.savedState", LeftoverKind::SavedState), Some(MatchRule::BundleId));
        assert_eq!(c("com.figma.Desktop", LeftoverKind::Caches), Some(MatchRule::BundleId));
        // Another installed app with a longer id: not ours.
        assert_eq!(c("com.figma.Desktop.agent", LeftoverKind::Caches), None);
        assert_eq!(c("Figma", LeftoverKind::ApplicationSupport), Some(MatchRule::Vendor));
        assert_eq!(c("FigmaAgent", LeftoverKind::ApplicationSupport), Some(MatchRule::Fuzzy));
        assert_eq!(c("TEAM123.com.figma.shared", LeftoverKind::GroupContainer), Some(MatchRule::SharedGroup));
        // Loose resemblance in Preferences is not a match.
        assert_eq!(c("FigmaThing.plist", LeftoverKind::Preferences), None);
        assert_eq!(c("Google", LeftoverKind::ApplicationSupport), None);
        assert_eq!(confidence_for(MatchRule::SharedGroup), Confidence::Danger);
        assert_eq!(confidence_for(MatchRule::Vendor), Confidence::Review);
    }

    #[test]
    fn finds_app_files_and_orphans() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        for (p, bytes) in [
            ("Preferences/com.figma.Desktop.plist", 10),
            ("Caches/com.figma.Desktop/cache.bin", 2000),
            ("Application Support/Figma/data.db", 5000),
            ("Preferences/com.gone.OldApp.plist", 7),
            ("Application Support/com.gone.OldApp/state.json", 300),
            ("Preferences/com.apple.finder.plist", 5),
            ("Preferences/com.figma.helper-tool.plist", 3),
        ] {
            let path = lib.join(p);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, vec![0u8; bytes]).unwrap();
        }
        let installed: HashSet<String> = ["com.figma.Desktop".to_string()].into_iter().collect();

        let found = find_for_app(lib, &app(), &installed, false);
        let rules: Vec<_> = found.iter().map(|l| (l.path.rsplit('/').next().unwrap().to_string(), l.rule, l.selected)).collect();
        assert!(rules.contains(&("com.figma.Desktop.plist".into(), MatchRule::BundleId, true)));
        assert!(rules.contains(&("com.figma.Desktop".into(), MatchRule::BundleId, true)));
        // "Figma" is in the known-paths rules, so it is safe rather than a vendor match.
        assert!(rules.contains(&("Figma".into(), MatchRule::KnownPath, true)));
        assert_eq!(found.iter().find(|l| l.path.ends_with("com.figma.Desktop")).unwrap().size, 2000);

        let orphans = find_orphans(lib, &installed, false);
        assert_eq!(orphans.len(), 2);
        // Same vendor as an installed app: kept for review, never preselected.
        let shared = orphans.iter().find(|g| g.bundle_id == "com.figma.helper-tool").unwrap();
        assert!(shared.vendor_installed && shared.items.iter().all(|i| !i.selected && i.confidence == Confidence::Review));
        let orphans: Vec<_> = orphans.into_iter().filter(|g| g.bundle_id == "com.gone.OldApp").collect();
        assert!(!orphans[0].vendor_installed);
        let prefs = orphans[0].items.iter().find(|i| i.kind == LeftoverKind::Preferences).unwrap();
        assert!(prefs.selected && prefs.confidence == Confidence::Safe);
        let support = orphans[0].items.iter().find(|i| i.kind == LeftoverKind::ApplicationSupport).unwrap();
        assert!(!support.selected && support.confidence == Confidence::Review);
    }
}
