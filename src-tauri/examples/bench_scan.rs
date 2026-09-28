//! Read-only scan benchmark: `cargo run --release --example bench_scan -- <folder>`.
//! Prints files per second, peak memory (RSS) and cancellation latency.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use organizamymac_lib::filesystem::scanner::{prompting_locations, scan, ScanOptions, VisitEntry};
use organizamymac_lib::filesystem::{probe_full_disk_access, AccessStatus};
use organizamymac_lib::storage::largest::LargestFiles;
use organizamymac_lib::storage::tree::StorageTree;
use organizamymac_lib::storage::Fanout;

fn peak_rss_mb() -> f64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    usage.ru_maxrss as f64 / 1_048_576.0 // bytes on macOS
}

fn main() {
    let root = std::env::args().nth(1).map(Into::into).unwrap_or_else(|| dirs::home_dir().unwrap());
    let home = dirs::home_dir().unwrap();
    let fda = probe_full_disk_access(&home) == AccessStatus::Granted;
    let opts = ScanOptions { root: root.clone(), skipped: prompting_locations(&home, fda), ..Default::default() };

    let cancel = Arc::new(AtomicBool::new(false));
    let mut tree = StorageTree::new(&root, 1_000_000);
    let mut largest = LargestFiles::new(100);
    let stats = scan(&opts, &cancel, &mut Fanout(vec![&mut tree, &mut largest]), |_| {}).unwrap();
    tree.finish();
    let secs = stats.duration_ms as f64 / 1000.0;
    let entries = stats.files + stats.directories + stats.symlinks;
    println!("root        {}", root.display());
    println!("entries     {entries} ({} files, {} folders)", stats.files, stats.directories);
    println!("logical     {:.2} GB, allocated {:.2} GB", stats.bytes_scanned as f64 / 1e9, stats.bytes_allocated as f64 / 1e9);
    println!("time        {secs:.2} s  → {:.0} entries/s", entries as f64 / secs.max(0.001));
    println!("warnings    {}", stats.warnings);
    println!("peak RSS    {:.0} MB", peak_rss_mb());

    // Cancellation latency: cancel 300 ms into a second scan.
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        flag.store(true, Ordering::Relaxed);
    });
    let t = Instant::now();
    let s = scan(&opts, &cancel, &mut |_: &VisitEntry| {}, |_| {}).unwrap();
    println!("cancel      stopped {} ms after the request (cancelled: {})", t.elapsed().as_millis().saturating_sub(300), s.cancelled);
}
