//! Read-only evaluation of the cleanup rules: `cargo run --release --example rules_report`.
use organizemymac_lib::cleanup::rules::{builtin_rules, evaluate, running_apps};

fn main() {
    let home = dirs::home_dir().unwrap();
    let running = running_apps();
    println!("{} running apps", running.len());
    let t = std::time::Instant::now();
    for r in evaluate(&home, &builtin_rules(), &running, false) {
        let sel: u64 = r.items.iter().filter(|i| i.selected).map(|i| i.size).sum();
        let blocked = r.items.iter().filter(|i| i.blocked.is_some()).count();
        println!("{:<22} {:>9.2} GB  {:>4} items  selected {:>7.2} GB  blocked {:>3}  {}", r.rule.id, r.size as f64 / 1e9, r.items.len(), sel as f64 / 1e9, blocked, if r.blocked_by.is_empty() { String::new() } else { format!("(close {})", r.blocked_by.join(", ")) });
    }
    println!("evaluated in {:.1} s", t.elapsed().as_secs_f64());
}
