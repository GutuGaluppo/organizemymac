//! Mac Health: CPU, memory, swap, disk, battery, uptime and processes.
//!
//! One `sysinfo::System` is kept and refreshed on demand, so CPU usage is measured between two
//! consecutive refreshes. Only what is asked for is refreshed: the menu bar reads CPU and memory
//! (cheap); the process list is refreshed only when a screen needs it.

pub mod battery;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind, Users};

use crate::storage::volumes;
pub use battery::Battery;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessCategory {
    /// A user application (a process inside a .app bundle, owned by the user).
    Application,
    /// Other processes of the user (helpers, agents, command-line tools).
    Background,
    /// Processes of root and system accounts.
    System,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory: u64,
    pub cpu: f32,
    pub category: ProcessCategory,
    /// The .app bundle this process belongs to (helpers included).
    pub app_path: Option<String>,
    pub user: Option<String>,
}

/// Processes of one app summed together (Chrome and its helpers count as "Google Chrome").
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUsage {
    pub name: String,
    pub app_path: Option<String>,
    pub memory: u64,
    pub cpu: f32,
    pub processes: u32,
    /// The user can quit it (a regular app of this user).
    pub can_quit: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub cpu: f32,
    pub cores: usize,
    pub load_average: [f64; 3],
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disk_total: u64,
    pub disk_free: u64,
    pub uptime: u64,
    pub battery: Option<Battery>,
}

pub struct HealthMonitor {
    sys: Mutex<System>,
    users: Mutex<Users>,
    battery: Mutex<(Option<Instant>, Option<Battery>)>,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        let sys = System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage()).with_memory(MemoryRefreshKind::everything()));
        HealthMonitor { sys: Mutex::new(sys), users: Mutex::new(Users::new_with_refreshed_list()), battery: Mutex::new((None, None)) }
    }
}

/// The outermost .app bundle in a path ("/Applications/Google Chrome.app/…/Helper.app/…").
pub fn app_bundle_of(exe: &Path) -> Option<PathBuf> {
    exe.ancestors().filter(|p| p.extension().is_some_and(|e| e == "app")).last().map(Path::to_path_buf)
}

impl HealthMonitor {
    pub fn health(&self) -> Health {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let root = volumes::list().into_iter().find(|v| v.is_root);
        let load = System::load_average();
        Health {
            cpu: sys.global_cpu_usage(),
            cores: sys.cpus().len(),
            load_average: [load.one, load.five, load.fifteen],
            memory_total: sys.total_memory(),
            memory_used: sys.used_memory(),
            memory_available: sys.available_memory(),
            swap_total: sys.total_swap(),
            swap_used: sys.used_swap(),
            disk_total: root.as_ref().map(|v| v.total).unwrap_or(0),
            disk_free: root.as_ref().map(|v| v.free).unwrap_or(0),
            uptime: System::uptime(),
            battery: self.battery(),
        }
    }

    /// Battery status is read at most every 20 s (`pmset`).
    fn battery(&self) -> Option<Battery> {
        let mut cached = self.battery.lock().unwrap();
        if cached.0.is_none_or(|t| t.elapsed() > Duration::from_secs(20)) {
            *cached = (Some(Instant::now()), battery::read());
        }
        cached.1.clone()
    }

    pub fn processes(&self) -> Vec<ProcessInfo> {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory().with_exe(UpdateKind::OnlyIfNotSet).with_user(UpdateKind::OnlyIfNotSet),
        );
        let me = unsafe { libc::getuid() };
        let users = self.users.lock().unwrap();
        sys.processes()
            .values()
            .filter(|p| p.thread_kind().is_none())
            .map(|p| {
                let uid = p.user_id().map(|u| **u);
                let app = p.exe().and_then(app_bundle_of);
                let category = if uid != Some(me) {
                    ProcessCategory::System
                } else if app.is_some() {
                    ProcessCategory::Application
                } else {
                    ProcessCategory::Background
                };
                ProcessInfo {
                    pid: p.pid().as_u32(),
                    name: p.name().to_string_lossy().into_owned(),
                    memory: p.memory(),
                    cpu: p.cpu_usage(),
                    category,
                    app_path: app.map(|a| a.display().to_string()),
                    user: p.user_id().and_then(|u| users.get_user_by_id(u)).map(|u| u.name().to_string()),
                }
            })
            .collect()
    }
}

/// Groups processes by app bundle; processes outside bundles stay on their own.
pub fn by_app(processes: &[ProcessInfo]) -> Vec<AppUsage> {
    let mut groups: HashMap<String, AppUsage> = HashMap::new();
    for p in processes {
        let (key, name) = match &p.app_path {
            Some(app) => (app.clone(), Path::new(app).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| p.name.clone())),
            None => (format!("pid:{}", p.pid), p.name.clone()),
        };
        let e = groups.entry(key).or_insert(AppUsage {
            name,
            app_path: p.app_path.clone(),
            memory: 0,
            cpu: 0.0,
            processes: 0,
            can_quit: p.category == ProcessCategory::Application && p.app_path.as_deref().is_some_and(|a| !a.starts_with("/System/")),
        });
        e.memory += p.memory;
        e.cpu += p.cpu;
        e.processes += 1;
    }
    let mut out: Vec<AppUsage> = groups.into_values().collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.memory));
    out
}

/// Asks a regular app to quit, like choosing Quit in its menu (it can ask to save documents).
/// Never force-quits.
pub fn quit_app(app_path: &Path) -> Result<u32, String> {
    use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_exe(UpdateKind::Always));
    let me = unsafe { libc::getuid() };
    let mut asked = 0;
    for p in sys.processes().values() {
        if p.exe().and_then(app_bundle_of).as_deref() != Some(app_path) {
            continue;
        }
        let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(p.pid().as_u32() as libc::pid_t) else { continue };
        // Only the app's main process with a Dock presence, owned by this user.
        if app.activationPolicy() != NSApplicationActivationPolicy::Regular || p.user_id().map(|u| **u) != Some(me) {
            continue;
        }
        if app.terminate() {
            asked += 1;
        }
    }
    if asked == 0 { Err("the app is not running as a regular app of this user".into()) } else { Ok(asked) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_of_helpers() {
        let exe = Path::new("/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper");
        assert_eq!(app_bundle_of(exe).unwrap(), Path::new("/Applications/Google Chrome.app"));
        assert!(app_bundle_of(Path::new("/usr/bin/zsh")).is_none());
    }

    #[test]
    fn health_and_processes() {
        let m = HealthMonitor::default();
        let h = m.health();
        assert!(h.memory_total > 0 && h.cores > 0 && h.disk_total > 0);
        let procs = m.processes();
        assert!(procs.iter().any(|p| p.category == ProcessCategory::System));
        let apps = by_app(&procs);
        assert!(apps.windows(2).all(|w| w[0].memory >= w[1].memory));
    }
}
