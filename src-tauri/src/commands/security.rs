use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;

use super::{start_job, JobEvent, JobOutcome};
use crate::applications::find_bundles;
use crate::error::{AppError, AppResult};
use crate::filesystem::scanner::ScanEvent;
use crate::security::{audit_apps, persistence, AppAudit, PersistenceItem};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAudit {
    pub persistence: Vec<PersistenceItem>,
    pub apps: Vec<AppAudit>,
}

/// Read-only audit of startup items and app signatures. Changes nothing.
#[tauri::command]
pub fn start_security_audit(app: AppHandle, on_event: Channel<JobEvent<SecurityAudit>>) -> AppResult<String> {
    Ok(start_job(&app, on_event, "security", move |state, _cancel, emit| {
        emit(ScanEvent::Start { root: "/Library".into() });
        emit(ScanEvent::Stage { stage: "persistence".into(), done: 0, total: 1 });
        let items = persistence(&state.paths.home);
        let mut bundles = find_bundles(std::path::Path::new("/Applications"));
        bundles.extend(find_bundles(&state.paths.home.join("Applications")));
        let (tx, rx) = std::sync::mpsc::channel::<(u64, u64)>();
        let worker = std::thread::spawn(move || audit_apps(&bundles, &move |d, t| {
            let _ = tx.send((d, t));
        }));
        for (done, total) in rx {
            emit(ScanEvent::Stage { stage: "apps".into(), done, total });
        }
        let apps = worker.join().map_err(|_| AppError::Other("app audit failed".into()))?;
        Ok(JobOutcome { cancelled: false, result: SecurityAudit { persistence: items, apps } })
    }))
}
