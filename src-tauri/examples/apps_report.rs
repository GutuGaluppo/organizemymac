//! Read-only report of installed apps, one uninstall plan and leftovers of removed apps:
//! `cargo run --release --example apps_report -- [App name]`.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use organizemymac_lib::applications::leftovers::{find_for_app, find_orphans, installed_bundle_ids, AppIdentity};
use organizemymac_lib::applications::{list_apps, running_executables};

fn main() {
    let home = dirs::home_dir().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let t = Instant::now();
    let apps = list_apps(&home, &running_executables(), &cancel, &|_, _| {});
    println!("{} apps in {:.1} s, {:.1} GB", apps.len(), t.elapsed().as_secs_f64(), apps.iter().map(|a| a.size).sum::<u64>() as f64 / 1e9);
    for a in apps.iter().take(8) {
        println!("  {:<28} {:>8.2} GB  {:<40} running={} system={}", a.name, a.size as f64 / 1e9, a.bundle_id.clone().unwrap_or_default(), a.running, a.system_app);
    }
    let t = Instant::now();
    let installed = installed_bundle_ids(&home);
    println!("{} bundle ids indexed in {:.1} s", installed.len(), t.elapsed().as_secs_f64());

    let wanted = std::env::args().nth(1);
    if let Some(app) = apps.iter().find(|a| wanted.as_deref().is_some_and(|w| a.name == w)).or_else(|| apps.iter().find(|a| !a.system_app)) {
        let id = app.bundle_id.clone().unwrap_or_default();
        let found = find_for_app(&home.join("Library"), &AppIdentity { bundle_id: id.clone(), name: app.name.clone() }, &installed, false);
        println!("plan for {} ({id}):", app.name);
        for l in &found {
            println!("  {:?} {:?} {:>10} {}", l.confidence, l.rule, l.size, l.path.replace(home.to_str().unwrap(), "~"));
        }
    }
    let t = Instant::now();
    let orphans = find_orphans(&home.join("Library"), &installed, false);
    println!("{} leftover groups, {:.2} GB, in {:.1} s", orphans.len(), orphans.iter().map(|g| g.size).sum::<u64>() as f64 / 1e9, t.elapsed().as_secs_f64());
    for g in orphans.iter().take(8) {
        println!("  {:<45} {:>10} ({} items)", g.bundle_id, g.size, g.items.len());
    }
}
