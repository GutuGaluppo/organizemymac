//! Runs the real Swift helper (Vision) on images derived from a macOS wallpaper.

use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use organizamymac_lib::filesystem::scanner::{scan, ScanOptions};
use organizamymac_lib::images::{find_similar, ImageCandidates, Sensitivity};

fn sips(args: &[&str]) {
    assert!(Command::new("/usr/bin/sips").args(args).output().unwrap().status.success(), "sips {args:?}");
}

#[test]
fn groups_resized_recompressed_and_cropped_copies() {
    let pictures = "/System/Library/Desktop Pictures";
    let blue = format!("{pictures}/iMac Blue.heic");
    let pink = format!("{pictures}/iMac Pink.heic");
    let radial = format!("{pictures}/Radial Sky Blue.heic");
    if !std::path::Path::new(&blue).exists() {
        eprintln!("wallpapers not available; skipping");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let d = std::fs::canonicalize(dir.path()).unwrap();
    let p = |n: &str| d.join(n).display().to_string();
    sips(&["-s", "format", "jpeg", "-s", "formatOptions", "90", "-Z", "3000", &blue, "--out", &p("blue.jpg")]);
    sips(&["-s", "format", "jpeg", "-s", "formatOptions", "40", "-Z", "1200", &blue, "--out", &p("blue-small.jpg")]);
    sips(&["-c", "2600", "2600", &p("blue.jpg"), "--out", &p("blue-crop.jpg")]);
    sips(&["-s", "format", "jpeg", "-Z", "2000", &pink, "--out", &p("pink.jpg")]);
    sips(&["-s", "format", "jpeg", "-Z", "2000", &radial, "--out", &p("radial.jpg")]);

    let cancel = Arc::new(AtomicBool::new(false));
    let mut candidates = ImageCandidates::new();
    scan(&ScanOptions { root: d.clone(), ..Default::default() }, &cancel, &mut candidates, |_| {}).unwrap();
    assert_eq!(candidates.files.len(), 5);

    let (groups, stats) = find_similar(&candidates, Sensitivity::Normal, &cancel, &mut |_| {}).unwrap();
    assert_eq!(groups.len(), 1, "{groups:#?}");
    let mut names: Vec<_> = groups[0].images.iter().map(|i| i.entry.name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["blue-crop.jpg", "blue-small.jpg", "blue.jpg"]);
    // The highest-resolution copy is kept.
    let kept: Vec<_> = groups[0].images.iter().filter(|i| i.keep).collect();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].entry.name, "blue.jpg");
    // Vision only ran on images that passed the perceptual pre-filter.
    assert!(stats.feature_prints <= stats.analyzed);

    let (strict, _) = find_similar(&candidates, Sensitivity::Strict, &cancel, &mut |_| {}).unwrap();
    assert!(strict.iter().all(|g| g.images.iter().all(|i| i.entry.name.starts_with("blue"))));
}
