//! Battery status from `pmset -g batt` (a documented Apple tool; no private APIs).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
    pub on_ac: bool,
    /// Minutes until empty (or full, when charging), when macOS has an estimate.
    pub minutes_remaining: Option<u32>,
    pub state: String,
}

pub fn parse(output: &str) -> Option<Battery> {
    let on_ac = output.contains("'AC Power'");
    let line = output.lines().find(|l| l.contains("InternalBattery"))?;
    let after_tab = line.split('\t').nth(1)?;
    let mut parts = after_tab.split(';').map(str::trim);
    let percent = parts.next()?.trim_end_matches('%').parse().ok()?;
    let state = parts.next().unwrap_or("").to_string();
    let remaining = parts.next().unwrap_or("");
    let minutes_remaining = remaining.split_whitespace().next().and_then(|t| {
        let (h, m) = t.split_once(':')?;
        let total = h.parse::<u32>().ok()? * 60 + m.parse::<u32>().ok()?;
        (total > 0).then_some(total)
    });
    Some(Battery { percent, charging: state == "charging", on_ac, minutes_remaining, state })
}

/// None on Macs without a battery.
pub fn read() -> Option<Battery> {
    let out = std::process::Command::new("/usr/bin/pmset").args(["-g", "batt"]).output().ok()?;
    parse(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pmset() {
        let charged = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=22741091)\t100%; charged; 0:00 remaining present: true\n";
        let b = parse(charged).unwrap();
        assert_eq!((b.percent, b.charging, b.on_ac, b.minutes_remaining), (100, false, true, None));
        let draining = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t87%; discharging; 5:12 remaining present: true\n";
        let b = parse(draining).unwrap();
        assert_eq!((b.percent, b.on_ac, b.minutes_remaining), (87, false, Some(312)));
        let charging = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t40%; charging; (no estimate) present: true\n";
        let b = parse(charging).unwrap();
        assert!(b.charging && b.minutes_remaining.is_none());
        assert!(parse("Now drawing from 'AC Power'\n").is_none());
    }
}
