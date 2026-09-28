//! Read-only duplicate search benchmark: `cargo run --release --example bench_duplicates -- <folder> [min bytes]`.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use organizemymac_lib::duplicates::{DuplicateCandidates, DuplicateSearch};
use organizemymac_lib::filesystem::scanner::{prompting_locations, scan, ScanOptions};

fn main() {
    let root: std::path::PathBuf = std::env::args().nth(1).map(Into::into).unwrap_or_else(|| dirs::home_dir().unwrap());
    let min: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(1_000_000);
    let home = dirs::home_dir().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let t = Instant::now();
    let mut candidates = DuplicateCandidates::new(min);
    let opts = ScanOptions { root: root.clone(), skipped: prompting_locations(&home, false), ..Default::default() };
    let stats = scan(&opts, &cancel, &mut candidates, |_| {}).unwrap();
    let scan_s = t.elapsed().as_secs_f64();
    let (groups, h) = DuplicateSearch { home: &home, cancel: &cancel }.run(candidates, &mut |_| {}).unwrap();
    let wasted: u64 = groups.iter().map(|g| g.wasted).sum();
    println!("root         {} (files ≥ {min} bytes)", root.display());
    println!("scan         {} files in {scan_s:.2} s", stats.files);
    println!("candidates   {} → {} after size → {} full-hashed", h.candidates, h.after_size, h.full_hashed);
    println!("hashing      {:.2} GB in {:.2} s → {:.0} MB/s", h.bytes_hashed as f64 / 1e9, h.hash_ms as f64 / 1000.0, h.bytes_hashed as f64 / 1e6 / (h.hash_ms as f64 / 1000.0).max(0.001));
    println!("result       {} groups, {:.2} GB reclaimable", groups.len(), wasted as f64 / 1e9);
}
