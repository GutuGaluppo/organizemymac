//! Read-only security audit: `cargo run --release --example security_report`.
use organizamymac_lib::applications::find_bundles;
use organizamymac_lib::security::{audit_apps, persistence};

fn main() {
    let home = dirs::home_dir().unwrap();
    let t = std::time::Instant::now();
    let items = persistence(&home);
    println!("{} startup items in {:.1} s", items.len(), t.elapsed().as_secs_f64());
    for i in items.iter().filter(|i| !i.findings.is_empty()).take(12) {
        println!("  {:?} {:<50} {:?} {:?}", i.scope, i.label.clone().unwrap_or_default(), i.signature.as_ref().map(|s| s.kind), i.findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>());
    }
    let t = std::time::Instant::now();
    let apps = audit_apps(&find_bundles(std::path::Path::new("/Applications")), &|_, _| {});
    println!("{} apps in {:.1} s", apps.len(), t.elapsed().as_secs_f64());
    for a in apps.iter().filter(|a| !a.findings.is_empty()) {
        println!("  {:<30} {:?} gatekeeper={:?} quarantined={} {:?}", a.name, a.signature.kind, a.gatekeeper, a.quarantined, a.findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>());
    }
}
