use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use crate::db::Db;
use crate::error::AppResult;
use crate::filesystem::safety::SafetyPolicy;
use crate::jobs::Jobs;
use crate::storage::tree::StorageTree;

pub struct AppPaths {
    pub home: PathBuf,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
}

/// A finished scan kept in memory so views (Space Map, results) can query it without rescanning.
pub struct ScanSession {
    pub id: String,
    pub root: PathBuf,
    pub tree: StorageTree,
}

pub struct AppState {
    pub paths: AppPaths,
    pub db: Mutex<Db>,
    pub policy: RwLock<SafetyPolicy>,
    pub jobs: Jobs,
    sessions: Mutex<VecDeque<Arc<Mutex<ScanSession>>>>,
}

/// Only the most recent sessions are kept; each holds one node per folder.
const MAX_SESSIONS: usize = 3;

impl AppState {
    pub fn new(paths: AppPaths, db: Db, policy: SafetyPolicy) -> AppResult<Self> {
        let state = AppState { paths, db: Mutex::new(db), policy: RwLock::new(policy), jobs: Jobs::default(), sessions: Mutex::new(VecDeque::new()) };
        state.reload_ignore_list()?;
        Ok(state)
    }

    pub fn reload_ignore_list(&self) -> AppResult<()> {
        let ignored = self.db.lock().unwrap().ignore_list()?.into_iter().map(|e| PathBuf::from(e.path)).collect();
        self.policy.write().unwrap().set_ignored(ignored);
        Ok(())
    }

    pub fn ignored_paths(&self) -> Vec<PathBuf> {
        self.db
            .lock()
            .unwrap()
            .ignore_list()
            .map(|l| l.into_iter().map(|e| PathBuf::from(e.path)).collect())
            .unwrap_or_default()
    }

    pub fn store_session(&self, session: ScanSession) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.push_front(Arc::new(Mutex::new(session)));
        sessions.truncate(MAX_SESSIONS);
    }

    pub fn session(&self, id: &str) -> Option<Arc<Mutex<ScanSession>>> {
        self.sessions.lock().unwrap().iter().find(|s| s.lock().unwrap().id == id).cloned()
    }

    /// Keeps cached trees in sync after items are moved to the Trash.
    pub fn forget_paths(&self, paths: &[PathBuf]) {
        for s in self.sessions.lock().unwrap().iter() {
            let mut s = s.lock().unwrap();
            for p in paths {
                if p.starts_with(&s.root) {
                    s.tree.remove(p);
                }
            }
        }
    }
}
