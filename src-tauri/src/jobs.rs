//! Long-running jobs (scans, hashing, cleanup). Each job gets an id and a cancel flag; work runs
//! on its own low-priority thread so the UI thread and the IPC layer never block.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Jobs {
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Jobs {
    pub fn register(&self) -> (String, Arc<AtomicBool>) {
        let id = uuid::Uuid::new_v4().to_string();
        let flag = Arc::new(AtomicBool::new(false));
        self.running.lock().unwrap().insert(id.clone(), flag.clone());
        (id, flag)
    }

    /// Returns false when the job already finished.
    pub fn cancel(&self, id: &str) -> bool {
        match self.running.lock().unwrap().get(id) {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub fn cancel_all(&self) {
        for flag in self.running.lock().unwrap().values() {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn finish(&self, id: &str) {
        self.running.lock().unwrap().remove(id);
    }

    pub fn running_count(&self) -> usize {
        self.running.lock().unwrap().len()
    }
}

/// Spawns `work` on a background thread with the "utility" QoS class.
pub fn spawn_background(name: &str, work: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            unsafe {
                libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_UTILITY, 0);
            }
            work();
        })
        .expect("spawn job thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_sets_flag_until_finished() {
        let jobs = Jobs::default();
        let (id, flag) = jobs.register();
        assert!(!flag.load(Ordering::Relaxed));
        assert!(jobs.cancel(&id));
        assert!(flag.load(Ordering::Relaxed));
        jobs.finish(&id);
        assert!(!jobs.cancel(&id));
        assert_eq!(jobs.running_count(), 0);
    }
}
