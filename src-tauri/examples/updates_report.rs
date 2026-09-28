//! Update detection report: `cargo run --release --example updates_report [--online]`.
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use organizamymac_lib::applications::{list_apps, running_executables};
use organizamymac_lib::updates::{check, sources, UpdateSource};

fn main() {
    let home = dirs::home_dir().unwrap();
    let apps = list_apps(&home, &running_executables(), &Arc::new(AtomicBool::new(false)), &|_, _| {});
    let mut infos = sources(&apps);
    let count = |s| infos.iter().filter(|i| i.source == s).count();
    println!("{} apps: {} App Store, {} Sparkle, {} without a source", infos.len(), count(UpdateSource::AppStore), count(UpdateSource::Sparkle), count(UpdateSource::None));
    if std::env::args().any(|a| a == "--online") {
        let t = std::time::Instant::now();
        infos = check(infos, &|_, _| {});
        println!("checked in {:.1} s", t.elapsed().as_secs_f64());
        for i in infos.iter().filter(|i| i.source != UpdateSource::None) {
            println!("  {:<26} {:<9?} installed {:<12} latest {:<12} {}", i.name, i.source, i.installed.clone().unwrap_or_default(), i.latest.clone().unwrap_or_default(), if i.update_available { "UPDATE" } else { i.error.as_deref().unwrap_or("") });
        }
    }
}
