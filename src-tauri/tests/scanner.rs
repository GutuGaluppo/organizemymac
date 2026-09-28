mod common;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use organizamymac_lib::filesystem::scanner::{scan, ScanEvent, ScanOptions};
use organizamymac_lib::storage::largest::LargestFiles;
use organizamymac_lib::storage::tree::StorageTree;
use organizamymac_lib::storage::Fanout;

fn run(opts: &ScanOptions) -> (organizamymac_lib::filesystem::scanner::ScanStats, StorageTree, Vec<organizamymac_lib::types::FileEntry>, Vec<ScanEvent>) {
    let cancel = Arc::new(AtomicBool::new(false));
    let mut tree = StorageTree::new(&opts.root, 1_000_000);
    let mut largest = LargestFiles::new(5);
    let mut events = Vec::new();
    let stats = {
        let mut fan = Fanout(vec![&mut tree, &mut largest]);
        scan(opts, &cancel, &mut fan, |e| events.push(e)).unwrap()
    };
    tree.finish();
    (stats, tree, largest.into_sorted(), events)
}

#[test]
fn scans_fixture_without_following_symlinks() {
    let f = common::build();
    let (stats, tree, largest, events) = run(&ScanOptions { root: f.root.clone(), ..Default::default() });

    assert!(!stats.cancelled);
    // 5 dups + 3 hard-link names + 3 media + 3 zero + 4 unicode/hidden + 1 deep leaf = 19 regular
    // files (the locked folder cannot be read).
    assert_eq!(stats.files, 19);
    assert_eq!(stats.symlinks, 3);
    // The symlink to /usr and the loop were not followed: nothing from /usr, and the scan ended.
    assert!(largest.iter().all(|e| e.path.starts_with(f.root.to_str().unwrap())));

    // Hard links counted once in the totals.
    let expected_logical = 5 * 300_000 + 100_000 + common::SPARSE_SIZE + 2_000_000 + 1_500_000 + 7 + 7 + 6 + 13 + 4;
    assert_eq!(stats.bytes_scanned, expected_logical as u64);
    // Sparse file: logical size is huge, allocated space is tiny.
    assert!(stats.bytes_allocated < 50_000_000, "allocated {}", stats.bytes_allocated);

    // Permission-denied folder becomes a warning, not a failure.
    assert!(stats.warnings >= 1);
    assert!(events.iter().any(|e| matches!(e, ScanEvent::Warning { message, .. } if message.contains("permission"))));
    assert!(matches!(events.first(), Some(ScanEvent::Start { .. })));

    // The tree adds up to the same total.
    let root = tree.view(tree.root_id().unwrap(), 1, 100).unwrap();
    assert_eq!(root.size, stats.bytes_scanned);
    assert_eq!(root.children.unwrap()[0].name, "big");

    // Largest first.
    assert_eq!(largest[0].name, "sparse.img");
    assert_eq!(largest[1].name, "video.mov");
}

#[test]
fn deep_trees_and_unicode_names() {
    let f = common::build();
    let (_, tree, _, _) = run(&ScanOptions { root: f.root.clone(), ..Default::default() });
    let mut deep = f.root.join("deep");
    for i in 0..common::DEEP_LEVELS {
        deep.push(format!("l{i}"));
    }
    assert!(tree.find(&deep).is_some());
    assert!(tree.find(&f.root.join("unicode/日本語")).is_some());
    assert!(tree.find(&f.root.join(".hidden-dir")).is_some());
}

#[test]
fn exclusions_are_skipped() {
    let f = common::build();
    let (stats, tree, _, _) = run(&ScanOptions { root: f.root.clone(), exclusions: vec![f.root.join("big")], ..Default::default() });
    assert!(tree.find(&f.root.join("big")).is_none());
    assert!(stats.bytes_scanned < common::SPARSE_SIZE);
}

#[test]
fn cancellation_stops_quickly() {
    let f = common::build();
    let cancel = Arc::new(AtomicBool::new(true));
    let mut count = 0;
    let stats = scan(&ScanOptions { root: f.root.clone(), ..Default::default() }, &cancel, &mut |_: &organizamymac_lib::filesystem::scanner::VisitEntry| count += 1, |_| {}).unwrap();
    assert!(stats.cancelled);
    assert!(count <= 1);
}

#[test]
fn missing_root_is_an_error() {
    let f = common::build();
    let cancel = Arc::new(AtomicBool::new(false));
    let res = scan(&ScanOptions { root: f.root.join("nope"), ..Default::default() }, &cancel, &mut |_: &organizamymac_lib::filesystem::scanner::VisitEntry| {}, |_| {});
    assert!(res.is_err());
    let file = scan(&ScanOptions { root: f.root.join("big/video.mov"), ..Default::default() }, &cancel, &mut |_: &organizamymac_lib::filesystem::scanner::VisitEntry| {}, |_| {});
    assert!(file.is_err());
}

#[test]
fn removing_from_tree_updates_totals() {
    let f = common::build();
    let (stats, mut tree, _, _) = run(&ScanOptions { root: f.root.clone(), ..Default::default() });
    tree.remove(&f.root.join("big/video.mov"));
    let root = tree.view(tree.root_id().unwrap(), 0, 10).unwrap();
    assert_eq!(root.size, stats.bytes_scanned - 2_000_000);
    assert!(tree.find(&f.root.join("big/video.mov")).is_none());
}

#[test]
fn scans_started_inside_the_pool_do_not_deadlock() {
    use rayon::prelude::*;
    let f = common::build();
    let roots: Vec<_> = (0..24).map(|_| f.root.clone()).collect();
    // More concurrent scans than pool threads, each started from a pool thread.
    let totals: Vec<u64> = organizamymac_lib::filesystem::worker_pool().install(|| {
        roots
            .par_iter()
            .map(|r| {
                let cancel = Arc::new(AtomicBool::new(false));
                scan(&ScanOptions { root: r.clone(), ..Default::default() }, &cancel, &mut |_: &organizamymac_lib::filesystem::scanner::VisitEntry| {}, |_| {})
                    .unwrap()
                    .bytes_scanned
            })
            .collect()
    });
    assert!(totals.windows(2).all(|w| w[0] == w[1]));
}
