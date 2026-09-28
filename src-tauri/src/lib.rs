pub mod commands;
pub mod db;
pub mod error;
pub mod filesystem;
pub mod jobs;
pub mod state;
pub mod storage;
pub mod types;

use std::path::PathBuf;
use std::sync::OnceLock;

use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;

use crate::db::Db;
use crate::filesystem::safety::SafetyPolicy;
use crate::state::{AppPaths, AppState};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Local log file, rotated daily, in ~/Library/Logs/<bundle id>.
fn init_logging(log_dir: &std::path::Path) {
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("organizamymac")
        .filename_suffix("log")
        .max_log_files(7)
        .build(log_dir);
    let Ok(appender) = appender else { return };
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let _ = LOG_GUARD.set(guard);
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_writer(writer).with_ansi(false).with_env_filter(filter).try_init();
}

/// Paths that belong to OrganizaMyMac itself and must never be removed while it runs.
fn own_paths(paths: &AppPaths) -> Vec<PathBuf> {
    let mut own = vec![paths.data_dir.clone(), paths.log_dir.clone()];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bundle) = exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")) {
            own.push(bundle.to_path_buf());
        }
    }
    own
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("home folder not found")?;
    let data_dir = app.path().app_data_dir()?;
    let log_dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&log_dir)?;
    init_logging(&log_dir);
    tracing::info!(version = %app.package_info().version, "starting");

    let paths = AppPaths { home: home.clone(), data_dir: data_dir.clone(), log_dir };
    let db = Db::open(&data_dir.join("organizamymac.sqlite"))?;
    let policy = SafetyPolicy::system(&home, own_paths(&paths));
    app.manage(AppState::new(paths, db, policy)?);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::cancel_job,
            commands::scan::start_scan,
            commands::scan::storage_node,
            commands::system::permission_status,
            commands::system::open_system_settings,
            commands::system::suggested_locations,
            commands::system::app_info,
            commands::system::recent_scans,
            commands::system::operation_log,
            commands::system::local_metrics,
            commands::system::ignore_list,
            commands::system::add_to_ignore_list,
            commands::system::remove_from_ignore_list,
        ])
        .build(tauri::generate_context!())
        .expect("error while building OrganizaMyMac")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                app.state::<AppState>().jobs.cancel_all();
            }
        });
}
