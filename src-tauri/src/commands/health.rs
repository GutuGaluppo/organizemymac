use std::path::PathBuf;

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::processes::{by_app, quit_app, AppUsage, Health, HealthMonitor, ProcessInfo};

#[tauri::command]
pub async fn health(monitor: tauri::State<'_, HealthMonitor>) -> AppResult<Health> {
    Ok(monitor.health())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessList {
    pub processes: Vec<ProcessInfo>,
    pub apps: Vec<AppUsage>,
}

#[tauri::command]
pub async fn process_list(monitor: tauri::State<'_, HealthMonitor>) -> AppResult<ProcessList> {
    let processes = monitor.processes();
    let apps = by_app(&processes);
    Ok(ProcessList { processes, apps })
}

/// Asks an app to quit normally (it may ask to save documents). Only regular apps of this user.
#[tauri::command]
pub async fn quit_application(app_path: String) -> AppResult<u32> {
    let path = PathBuf::from(&app_path);
    if !path.is_absolute() || path.extension().is_none_or(|e| e != "app") || app_path.starts_with("/System/") {
        return Err(AppError::Invalid("only regular apps can be quit".into()));
    }
    let own = std::env::current_exe().ok().and_then(|e| crate::processes::app_bundle_of(&e));
    if own.as_deref() == Some(path.as_path()) {
        return Err(AppError::Invalid("use Quit in the menu to close OrganizaMyMac".into()));
    }
    tauri::async_runtime::spawn_blocking(move || quit_app(&path))
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
        .map_err(AppError::Other)
}
