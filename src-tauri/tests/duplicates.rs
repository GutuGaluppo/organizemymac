mod common;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use organizemymac_lib::duplicates::{DuplicateCandidates, DuplicateSearch};
use organizemymac_lib::filesystem::scanner::{scan, ScanOptions};

fn find(root: &std::path::Path, min_size: u64) -> (Vec<organizemymac_lib::duplicates::DuplicateGroup>, organizemymac_lib::duplicates::HashStats) {
    let cancel = Arc::new(AtomicBool::new(false));
    let mut candidates = DuplicateCandidates::new(min_size);
    scan(&ScanOptions { root: root.to_path_buf(), ..Default::default() }, &cancel, &mut candidates, |_| {}).unwrap();
    DuplicateSearch { home: root, cancel: &cancel }.run(candidates, &mut |_| {}).unwrap()
}

#[test]
fn finds_true_duplicates_only() {
    let f = common::build();
    let (groups, stats) = find(&f.root, 1);

    // The three identical photos form one group; near.jpg and tail.jpg differ.
    let photos = groups.iter().find(|g| g.size == 300_000).expect("photo group");
    let names: Vec<&str> = photos.files.iter().map(|d| d.entry.name.as_str()).collect();
    assert_eq!(photos.files.len(), 3, "{names:?}");
    assert!(!names.contains(&"near.jpg") && !names.contains(&"tail.jpg"));
    assert_eq!(photos.wasted, 600_000);

    // Never every copy selected: exactly one kept, and it is the original, not "photo copy.jpg".
    assert_eq!(photos.files.iter().filter(|d| !d.selected).count(), 1);
    let kept = photos.files.iter().find(|d| !d.selected).unwrap();
    assert_ne!(kept.entry.name, "photo copy.jpg");

    // Hard links are one file: no group for original.bin/hard1/hard2.
    assert!(groups.iter().all(|g| g.size != 100_000));
    // Zero-byte files are ignored.
    assert!(groups.iter().all(|g| g.size > 0));
    assert!(stats.candidates > 0);
}

#[test]
fn full_hash_only_when_samples_match() {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    let base = common::content(9, 1_000_000);
    // Same size, identical samples (first/middle/last 64 KB), different at byte 300 000.
    let mut sneaky = base.clone();
    sneaky[300_000] ^= 1;
    common::write(&root.join("a.bin"), &base);
    common::write(&root.join("b.bin"), &base);
    common::write(&root.join("c.bin"), &sneaky);
    // Same size but different first bytes: dropped after the sample stage.
    let mut other = base.clone();
    other[0] ^= 1;
    common::write(&root.join("d.bin"), &other);
    // Unique size: never read.
    common::write(&root.join("e.bin"), &common::content(3, 999_999));

    let (groups, stats) = find(&root, 1);
    assert_eq!(groups.len(), 1);
    let names: Vec<_> = groups[0].files.iter().map(|d| d.entry.name.clone()).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a.bin".to_string()) && names.contains(&"b.bin".to_string()));

    assert_eq!(stats.candidates, 5);
    assert_eq!(stats.after_size, 4); // e.bin dropped by size
    assert_eq!(stats.sample_hashed, 4);
    assert_eq!(stats.full_hashed, 3); // a, b, c (d dropped by sample)
    assert_eq!(stats.bytes_hashed, 3_000_000);
}

#[test]
fn min_size_filters_small_files() {
    let f = common::build();
    let (groups, _) = find(&f.root, 1_000_000);
    assert!(groups.is_empty());
}
