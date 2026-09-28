pub mod metadata;
pub mod safety;
pub mod scanner;

use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessStatus {
    Granted,
    Denied,
    Unknown,
}

/// macOS has no public API for Full Disk Access: it is inferred by listing folders that only apps
/// with that permission can read. Used as a hint (and to avoid folders that would prompt), never
/// to bypass anything.
pub fn probe_full_disk_access(home: &Path) -> AccessStatus {
    let mut status = AccessStatus::Unknown;
    for dir in ["Library/Safari", "Library/Mail", "Library/Messages"] {
        match std::fs::read_dir(home.join(dir)) {
            Ok(_) => return AccessStatus::Granted,
            Err(e) if e.kind() == ErrorKind::PermissionDenied || e.raw_os_error() == Some(libc::EPERM) => {
                status = AccessStatus::Denied
            }
            Err(_) => {}
        }
    }
    status
}

/// Shared pool for scanning and hashing. Few threads and the "utility" QoS class, so a scan never
/// competes with what the user is doing (macOS also moves these threads to efficiency cores).
pub fn worker_pool() -> Arc<rayon::ThreadPool> {
    static POOL: OnceLock<Arc<rayon::ThreadPool>> = OnceLock::new();
    POOL.get_or_init(|| {
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 6);
        Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .thread_name(|i| format!("organize-worker-{i}"))
                .start_handler(|_| unsafe {
                    libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_UTILITY, 0);
                })
                .build()
                .expect("worker pool"),
        )
    })
    .clone()
}
