//! App update detection (spec §26). v1: installed versions. v2: Sparkle-enabled apps (their
//! appcast feed from Info.plist). v3: trusted external version sources — the app's own HTTPS
//! appcast and Apple's public App Store lookup. OrganizeMyMac only reports; it never downloads or
//! installs updates (signatures, sources and architectures are the app's own updater's job).

use std::cmp::Ordering;
use std::path::Path;
use std::time::Duration;

use rayon::prelude::*;
use serde::Serialize;

use crate::applications::AppInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateSource {
    AppStore,
    Sparkle,
    /// No known source: only the installed version is shown.
    None,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub path: String,
    pub name: String,
    pub bundle_id: Option<String>,
    pub installed: Option<String>,
    pub source: UpdateSource,
    pub feed_url: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
    /// App Store page or release notes.
    pub url: Option<String>,
    pub error: Option<String>,
}

/// The Sparkle feed declared by the app, if it is HTTPS.
pub fn sparkle_feed(app: &Path) -> Option<String> {
    let v = plist::Value::from_file(app.join("Contents/Info.plist")).ok()?;
    let feed = v.as_dictionary()?.get("SUFeedURL")?.as_string()?.trim().to_string();
    feed.starts_with("https://").then_some(feed)
}

pub fn source_of(app: &AppInfo) -> (UpdateSource, Option<String>) {
    let path = Path::new(&app.path);
    if app.from_app_store {
        return (UpdateSource::AppStore, None);
    }
    match sparkle_feed(path) {
        Some(feed) => (UpdateSource::Sparkle, Some(feed)),
        None => (UpdateSource::None, None),
    }
}

/// Compares dotted versions numerically ("1.10" > "1.9"); non-numeric parts compare as text.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |s: &str| s.split(['.', '-', ' ', '_']).filter(|p| !p.is_empty()).map(String::from).collect::<Vec<_>>();
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).map(String::as_str).unwrap_or("0");
        let y = pb.get(i).map(String::as_str).unwrap_or("0");
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(m), Ok(n)) => m.cmp(&n),
            _ => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

/// "4.4.1b", "5.0b3", "2.0-beta", "3.1rc1": a pre-release.
pub fn is_prerelease(version: &str) -> bool {
    let v = version.to_lowercase();
    ["alpha", "beta", "rc", "preview", "dev"].iter().any(|t| v.contains(t))
        || v.split(['.', '-']).any(|part| {
            let digits = part.trim_end_matches(|c: char| c.is_ascii_digit());
            part.chars().next().is_some_and(|c| c.is_ascii_digit()) && digits.chars().last().is_some_and(|c| matches!(c, 'a' | 'b'))
        })
}

/// Latest stable version in a Sparkle appcast (items on a named channel and pre-releases are
/// skipped): (short version, release notes URL).
pub fn parse_appcast(xml: &str) -> Option<(String, Option<String>)> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    const SPARKLE: &str = "http://www.andymatuschak.org/xml-namespaces/sparkle";
    let mut best: Option<(String, Option<String>)> = None;
    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let child_text = |name: &str| item.children().find(|c| c.tag_name().name() == name && c.tag_name().namespace() == Some(SPARKLE)).and_then(|c| c.text()).map(str::trim).map(String::from);
        let enclosure = item.children().find(|c| c.has_tag_name("enclosure"));
        let version = child_text("shortVersionString")
            .or_else(|| enclosure.and_then(|e| e.attribute((SPARKLE, "shortVersionString"))).map(String::from))
            .or_else(|| child_text("version"))
            .or_else(|| enclosure.and_then(|e| e.attribute((SPARKLE, "version"))).map(String::from));
        let Some(version) = version else { continue };
        let on_channel = item.children().any(|c| c.tag_name().name() == "channel" && c.tag_name().namespace() == Some(SPARKLE));
        if on_channel || is_prerelease(&version) {
            continue;
        }
        let notes = child_text("releaseNotesLink").or_else(|| item.children().find(|c| c.has_tag_name("link")).and_then(|c| c.text()).map(String::from));
        if best.as_ref().is_none_or(|(b, _)| compare_versions(&version, b) == Ordering::Greater) {
            best = Some((version, notes));
        }
    }
    best
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .https_only(true)
        .redirects(3)
        .user_agent(concat!("OrganizeMyMac/", env!("CARGO_PKG_VERSION"), " (update check)"))
        .build()
}

fn app_store_lookup(agent: &ureq::Agent, bundle_id: &str) -> Result<(String, Option<String>), String> {
    let body = agent
        .get("https://itunes.apple.com/lookup")
        .query("bundleId", bundle_id)
        .query("entity", "macSoftware")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let r = v["results"].get(0).ok_or("not found in the App Store")?;
    let version = r["version"].as_str().ok_or("no version")?.to_string();
    Ok((version, r["trackViewUrl"].as_str().map(String::from)))
}

fn sparkle_lookup(agent: &ureq::Agent, feed: &str) -> Result<(String, Option<String>), String> {
    let body = agent.get(feed).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    parse_appcast(&body).ok_or_else(|| "the feed has no versions".into())
}

/// Lists update sources for every app without touching the network.
pub fn sources(apps: &[AppInfo]) -> Vec<UpdateInfo> {
    apps.iter()
        .filter(|a| !a.system_app)
        .map(|a| {
            let (source, feed_url) = source_of(a);
            UpdateInfo {
                path: a.path.clone(),
                name: a.name.clone(),
                bundle_id: a.bundle_id.clone(),
                installed: a.version.clone(),
                source,
                feed_url,
                latest: None,
                update_available: false,
                url: None,
                error: None,
            }
        })
        .collect()
}

/// Checks each source online (only when the user asks). At most 6 requests at a time.
pub fn check(mut infos: Vec<UpdateInfo>, progress: &(dyn Fn(u64, u64) + Sync)) -> Vec<UpdateInfo> {
    let agent = agent();
    let total = infos.iter().filter(|i| i.source != UpdateSource::None).count() as u64;
    let done = std::sync::atomic::AtomicU64::new(0);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(6).build().expect("update pool");
    pool.install(|| {
        infos.par_iter_mut().filter(|i| i.source != UpdateSource::None).for_each(|info| {
            let result = match (info.source, &info.feed_url, &info.bundle_id) {
                (UpdateSource::AppStore, _, Some(id)) => app_store_lookup(&agent, id),
                (UpdateSource::Sparkle, Some(feed), _) => sparkle_lookup(&agent, feed),
                _ => Err("no source".into()),
            };
            match result {
                Ok((latest, url)) => {
                    info.update_available = info.installed.as_deref().is_some_and(|i| compare_versions(&latest, i) == Ordering::Greater);
                    info.latest = Some(latest);
                    info.url = url;
                }
                Err(e) => info.error = Some(e),
            }
            progress(done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1, total);
        })
    });
    infos.sort_by_key(|i| (!i.update_available, i.source == UpdateSource::None, i.name.to_lowercase()));
    infos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare_versions("2.0", "2"), Ordering::Equal);
        assert_eq!(compare_versions("126.1", "126.1.2"), Ordering::Less);
        assert_eq!(compare_versions("3.4 (1234)", "3.4 (1200)"), Ordering::Greater);
    }

    #[test]
    fn appcast() {
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <item>
      <title>2.9</title>
      <sparkle:releaseNotesLink>https://example.com/2.9</sparkle:releaseNotesLink>
      <enclosure url="https://example.com/a.zip" sparkle:version="290" sparkle:shortVersionString="2.9" />
    </item>
    <item>
      <title>2.10</title>
      <sparkle:shortVersionString>2.10</sparkle:shortVersionString>
      <sparkle:releaseNotesLink>https://example.com/2.10</sparkle:releaseNotesLink>
      <enclosure url="https://example.com/b.zip" sparkle:version="2100" />
    </item>
  </channel>
</rss>"#;
        let (v, notes) = parse_appcast(xml).unwrap();
        assert_eq!(v, "2.10");
        assert_eq!(notes.as_deref(), Some("https://example.com/2.10"));
        assert!(parse_appcast("<rss></rss>").is_none());
        let beta = r#"<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"><channel>
            <item><sparkle:shortVersionString>4.3.0</sparkle:shortVersionString></item>
            <item><sparkle:shortVersionString>4.4.1b</sparkle:shortVersionString></item>
            <item><sparkle:channel>beta</sparkle:channel><sparkle:shortVersionString>4.5.0</sparkle:shortVersionString></item>
        </channel></rss>"#;
        assert_eq!(parse_appcast(beta).unwrap().0, "4.3.0");
        assert!(is_prerelease("5.0b3") && is_prerelease("2.0-beta") && is_prerelease("3.1rc1"));
        assert!(!is_prerelease("10.12.0") && !is_prerelease("26.3"));
        assert!(parse_appcast("not xml").is_none());
    }
}
