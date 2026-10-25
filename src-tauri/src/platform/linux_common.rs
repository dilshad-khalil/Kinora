//! Pieces every Linux backend shares.

use std::{fs, path::Path};

use super::Battery;

const SUPPLIES: &str = "/sys/class/power_supply";

fn read(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(name)).ok().map(|s| s.trim().to_string())
}

/// Every power supply of type "Mains" is offline. Machines without a battery,
/// or whose adapter isn't listed, count as plugged in.
pub fn on_battery() -> bool {
    let Ok(entries) = fs::read_dir(SUPPLIES) else { return false };
    let mut mains = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| read(p, "type").as_deref() == Some("Mains"))
        .peekable();
    if mains.peek().is_none() {
        return false;
    }
    !mains.any(|p| read(&p, "online").as_deref() == Some("1"))
}

pub fn battery() -> Option<Battery> {
    let battery = fs::read_dir(SUPPLIES)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| read(p, "type").as_deref() == Some("Battery"))?;
    let status = read(&battery, "status").unwrap_or_default();
    Some(Battery {
        percent: read(&battery, "capacity").and_then(|c| c.parse().ok()),
        charging: status == "Charging",
        plugged_in: !on_battery(),
    })
}
