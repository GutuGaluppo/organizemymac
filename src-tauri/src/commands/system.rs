use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::db::{IgnoreEntry, OperationRecord, ScanRecord};
use crate::error::{AppError, AppResult};
use crate::filesystem::AccessStatus;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    /// macOS has no public API for this; it is inferred by listing folders that only apps with
    /// Full Disk Access can read. Shown as a hint, never used to bypass anything.
    pub full_disk_access: AccessStatus,
    pub home: String,
}

#[tauri::command]
pub fn permission_status(state: tauri::State<'_, AppState>) -> PermissionStatus {
    PermissionStatus {
        full_disk_access: crate::filesystem::probe_full_disk_access(&state.paths.home),
        home: state.paths.home.display().to_string(),
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsPane {
    FullDiskAccess,
    LoginItems,
    Storage,
    Photos,
}

/// Opens a System Settings pane. The user always makes the change there.
#[tauri::command]
pub fn open_system_settings(pane: SettingsPane) -> AppResult<()> {
    let url = match pane {
        SettingsPane::FullDiskAccess => "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles",
        SettingsPane::LoginItems => "x-apple.systempreferences:com.apple.LoginItems-Settings.extension",
        SettingsPane::Storage => "x-apple.systempreferences:com.apple.settings.Storage",
        SettingsPane::Photos => "x-apple.systempreferences:com.apple.preference.security?Privacy_Photos",
    };
    Command::new("/usr/bin/open").arg(url).status()?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: &'static str,
    pub path: String,
    pub exists: bool,
}

/// Suggested scan roots.
#[tauri::command]
pub fn suggested_locations(state: tauri::State<'_, AppState>) -> Vec<Location> {
    let home = &state.paths.home;
    [
        ("home", home.clone()),
        ("downloads", home.join("Downloads")),
        ("documents", home.join("Documents")),
        ("desktop", home.join("Desktop")),
        ("applications", "/Applications".into()),
        ("disk", "/".into()),
    ]
    .into_iter()
    .map(|(id, p)| Location { id, exists: p.exists(), path: p.display().to_string() })
    .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub log_dir: String,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.paths.data_dir.display().to_string(),
        log_dir: state.paths.log_dir.display().to_string(),
    }
}

#[tauri::command]
pub fn recent_scans(state: tauri::State<'_, AppState>, module: Option<String>, limit: Option<u32>) -> AppResult<Vec<ScanRecord>> {
    state.db.lock().unwrap().recent_scans(module.as_deref(), limit.unwrap_or(20))
}

#[tauri::command]
pub fn operation_log(state: tauri::State<'_, AppState>, limit: Option<u32>) -> AppResult<Vec<OperationRecord>> {
    state.db.lock().unwrap().operations(limit.unwrap_or(200))
}

#[tauri::command]
pub fn local_metrics(state: tauri::State<'_, AppState>) -> AppResult<std::collections::HashMap<String, i64>> {
    Ok(state.db.lock().unwrap().metrics()?.into_iter().collect())
}

#[tauri::command]
pub fn ignore_list(state: tauri::State<'_, AppState>) -> AppResult<Vec<IgnoreEntry>> {
    state.db.lock().unwrap().ignore_list()
}

#[tauri::command]
pub fn add_to_ignore_list(state: tauri::State<'_, AppState>, path: String, reason: Option<String>) -> AppResult<()> {
    if !Path::new(&path).is_absolute() {
        return Err(AppError::Invalid("path must be absolute".into()));
    }
    state.db.lock().unwrap().add_ignore(&path, reason.as_deref())?;
    state.reload_ignore_list()
}

#[tauri::command]
pub fn remove_from_ignore_list(state: tauri::State<'_, AppState>, path: String) -> AppResult<()> {
    state.db.lock().unwrap().remove_ignore(&path)?;
    state.reload_ignore_list()
}
