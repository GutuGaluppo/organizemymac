//! Menu bar item: CPU and memory in the title; disk, battery and the apps using the most memory in
//! the menu. Built to stay on all day: metrics every 5 s, processes every 30 s and only while the
//! menu bar item is enabled, on a background thread with the "background" QoS class.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::processes::{by_app, HealthMonitor};
use crate::state::AppState;

const TRAY_ID: &str = "organizemymac";
const TOP_APPS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TitleMode {
    /// Icon only.
    Icon,
    #[default]
    Cpu,
    Memory,
    Both,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuBarSettings {
    pub enabled: bool,
    pub title: TitleMode,
}

impl Default for MenuBarSettings {
    fn default() -> Self {
        MenuBarSettings { enabled: true, title: TitleMode::Cpu }
    }
}

struct Items {
    cpu: MenuItem<Wry>,
    memory: MenuItem<Wry>,
    disk: MenuItem<Wry>,
    battery: MenuItem<Wry>,
    uptime: MenuItem<Wry>,
    apps: Vec<MenuItem<Wry>>,
}

pub struct MenuBar {
    tray: Mutex<Option<(TrayIcon<Wry>, Arc<Items>)>>,
    settings: Mutex<MenuBarSettings>,
    running: Arc<AtomicBool>,
}

impl MenuBar {
    pub fn new(settings: MenuBarSettings) -> Self {
        MenuBar { tray: Mutex::new(None), settings: Mutex::new(settings), running: Arc::new(AtomicBool::new(false)) }
    }

    pub fn settings(&self) -> MenuBarSettings {
        *self.settings.lock().unwrap()
    }
}

fn gb(bytes: u64) -> String {
    let v = bytes as f64 / 1e9;
    if v >= 100.0 { format!("{v:.0} GB") } else { format!("{v:.1} GB").replace('.', ",") }
}

fn duration(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    if d > 0 { format!("{d} d {h} h") } else if h > 0 { format!("{h} h {m} min") } else { format!("{m} min") }
}

fn build(app: &AppHandle) -> tauri::Result<(TrayIcon<Wry>, Arc<Items>)> {
    let text = |id: &str| MenuItem::with_id(app, id, "…", false, None::<&str>);
    let items = Items {
        cpu: text("cpu")?,
        memory: text("memory")?,
        disk: text("disk")?,
        battery: text("battery")?,
        uptime: text("uptime")?,
        apps: (0..TOP_APPS).map(|i| text(&format!("app{i}"))).collect::<tauri::Result<_>>()?,
    };
    let header = MenuItem::with_id(app, "apps-header", "Mais memória", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Abrir OrganizeMyMac", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Encerrar OrganizeMyMac", true, Some("CmdOrCtrl+Q"))?;
    let sep = || PredefinedMenuItem::separator(app);
    let mut entries: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = vec![&items.cpu, &items.memory, &items.disk, &items.battery, &items.uptime];
    let s1 = sep()?;
    entries.push(&s1);
    entries.push(&header);
    for a in &items.apps {
        entries.push(a);
    }
    let s2 = sep()?;
    entries.push(&s2);
    entries.push(&open);
    entries.push(&quit);
    let menu = Menu::with_items(app, &entries)?;

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(include_bytes!("../icons/tray-template.png"))?)
        .icon_as_template(true)
        .tooltip("OrganizeMyMac")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "quit" => {
                app.state::<MenuBar>().running.store(false, Ordering::Relaxed);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    Ok((tray, Arc::new(items)))
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Creates or removes the menu bar item to match the settings, and starts the refresh loop.
pub fn apply(app: &AppHandle) -> tauri::Result<()> {
    let bar = app.state::<MenuBar>();
    let settings = bar.settings();
    let mut tray = bar.tray.lock().unwrap();
    if !settings.enabled {
        if let Some((t, _)) = tray.take() {
            let _ = t.set_visible(false);
            drop(t);
            let _ = app.remove_tray_by_id(TRAY_ID);
        }
        bar.running.store(false, Ordering::Relaxed);
        return Ok(());
    }
    if tray.is_none() {
        *tray = Some(build(app)?);
    }
    if !bar.running.swap(true, Ordering::Relaxed) {
        let app = app.clone();
        let running = bar.running.clone();
        std::thread::Builder::new().name("menubar".into()).spawn(move || refresh_loop(app, running))?;
    }
    Ok(())
}

fn refresh_loop(app: AppHandle, running: Arc<AtomicBool>) {
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_BACKGROUND, 0);
    }
    let monitor = HealthMonitor::default();
    let mut last_apps: Option<Instant> = None;
    while running.load(Ordering::Relaxed) {
        let bar = app.state::<MenuBar>();
        let Some((tray, items)) = bar.tray.lock().unwrap().as_ref().map(|(t, i)| (t.clone(), i.clone())) else { break };
        let h = monitor.health();
        let mem_pct = h.memory_used as f64 / h.memory_total.max(1) as f64 * 100.0;
        let title = match bar.settings().title {
            TitleMode::Icon => None,
            TitleMode::Cpu => Some(format!("{:.0}%", h.cpu)),
            TitleMode::Memory => Some(format!("{mem_pct:.0}%")),
            TitleMode::Both => Some(format!("{:.0}% · {mem_pct:.0}%", h.cpu)),
        };
        let _ = tray.set_title(title.as_deref());
        let _ = items.cpu.set_text(format!("CPU: {:.0}% ({} núcleos)", h.cpu, h.cores));
        let _ = items.memory.set_text(format!("Memória: {} de {} em uso", gb(h.memory_used), gb(h.memory_total)));
        let _ = items.disk.set_text(format!("Disco: {} livres de {}", gb(h.disk_free), gb(h.disk_total)));
        let _ = items.battery.set_text(match &h.battery {
            Some(b) => {
                let rest = b.minutes_remaining.map(|m| format!(" · {}", duration(m as u64 * 60))).unwrap_or_default();
                format!("Bateria: {}%{}{rest}", b.percent, if b.charging { " carregando" } else if b.on_ac { " na tomada" } else { "" })
            }
            None => "Bateria: sem bateria".into(),
        });
        let _ = items.uptime.set_text(format!("Ligado há {}", duration(h.uptime)));
        if last_apps.is_none_or(|t| t.elapsed() >= Duration::from_secs(30)) {
            last_apps = Some(Instant::now());
            let top = by_app(&monitor.processes());
            for (i, item) in items.apps.iter().enumerate() {
                let _ = item.set_text(top.get(i).map(|a| format!("{}  {}", a.name, gb(a.memory))).unwrap_or_default());
            }
        }
        // Sleep in short steps so disabling the item stops the loop quickly.
        for _ in 0..10 {
            if !running.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

pub fn load_settings(state: &AppState) -> MenuBarSettings {
    state
        .db
        .lock()
        .unwrap()
        .setting("menuBar")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn menu_bar_settings(bar: tauri::State<'_, MenuBar>) -> MenuBarSettings {
    bar.settings()
}

#[tauri::command]
pub fn set_menu_bar_settings(app: AppHandle, settings: MenuBarSettings) -> crate::error::AppResult<()> {
    *app.state::<MenuBar>().settings.lock().unwrap() = settings;
    app.state::<AppState>().db.lock().unwrap().set_setting("menuBar", &serde_json::to_string(&settings).unwrap_or_default())?;
    apply(&app).map_err(|e| crate::error::AppError::Other(e.to_string()))
}
