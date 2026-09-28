use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;

use super::{start_job, JobEvent, JobOutcome};
use crate::applications::{list_apps, running_executables};
use crate::cloud::{self, CloudFolder, CloudUsage, EvictOutcome};
use crate::error::{AppError, AppResult};
use crate::filesystem::scanner::ScanEvent;
use crate::state::AppState;
use crate::updates::{self, UpdateInfo};

#[tauri::command]
pub fn cloud_folders(state: tauri::State<'_, AppState>) -> Vec<CloudFolder> {
    cloud::folders(&state.paths.home)
}

/// Measures one synced folder (explicitly chosen by the user; general scans skip cloud storage).
#[tauri::command]
pub fn start_cloud_usage(app: AppHandle, path: String, on_event: Channel<JobEvent<CloudUsage>>) -> AppResult<String> {
    Ok(start_job(&app, on_event, "cloud", move |state, cancel, emit| {
        let folder = cloud::folders(&state.paths.home).into_iter().find(|f| f.path == path).ok_or_else(|| AppError::NotFound(path.clone()))?;
        let usage = cloud::usage(&folder, cancel, emit)?;
        Ok(JobOutcome { cancelled: usage.cancelled, result: usage })
    }))
}

#[tauri::command]
pub async fn evict_icloud(state: tauri::State<'_, AppState>, paths: Vec<String>) -> AppResult<Vec<EvictOutcome>> {
    let home = state.paths.home.clone();
    tauri::async_runtime::spawn_blocking(move || cloud::evict(&home, &paths)).await.map_err(|e| AppError::Other(e.to_string()))?
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatesResult {
    pub apps: Vec<UpdateInfo>,
    /// Whether the sources were contacted (false: only local detection).
    pub checked_online: bool,
}

/// Update sources of installed apps; with `online`, also asks the App Store and each app's
/// HTTPS appcast for the latest version. Nothing is downloaded or installed.
#[tauri::command]
pub fn start_update_check(app: AppHandle, online: bool, on_event: Channel<JobEvent<UpdatesResult>>) -> AppResult<String> {
    Ok(start_job(&app, on_event, "updates", move |state, cancel, emit| {
        emit(ScanEvent::Start { root: "/Applications".into() });
        let apps = list_apps(&state.paths.home, &running_executables(), cancel, &|_, _| {});
        let infos = updates::sources(&apps);
        let infos = if online {
            let (tx, rx) = std::sync::mpsc::channel::<(u64, u64)>();
            let worker = std::thread::spawn(move || updates::check(infos, &move |d, t| {
                let _ = tx.send((d, t));
            }));
            for (done, total) in rx {
                emit(ScanEvent::Stage { stage: "check".into(), done, total });
            }
            worker.join().map_err(|_| AppError::Other("update check failed".into()))?
        } else {
            infos
        };
        Ok(JobOutcome { cancelled: false, result: UpdatesResult { apps: infos, checked_online: online } })
    }))
}

/// Opens an App Store page (only apps.apple.com URLs).
#[tauri::command]
pub fn open_app_store_page(url: String) -> AppResult<()> {
    if !url.starts_with("https://apps.apple.com/") {
        return Err(AppError::Invalid("not an App Store page".into()));
    }
    std::process::Command::new("/usr/bin/open").arg(url.replacen("https://", "macappstore://", 1)).status()?;
    Ok(())
}

/// Opens an installed app so the user can use its own "Check for Updates…".
#[tauri::command]
pub fn open_application(path: String) -> AppResult<()> {
    let p = std::path::PathBuf::from(&path);
    if !p.is_absolute() || p.extension().is_none_or(|e| e != "app") || !p.is_dir() {
        return Err(AppError::Invalid("not an application".into()));
    }
    std::process::Command::new("/usr/bin/open").arg(&p).status()?;
    Ok(())
}
