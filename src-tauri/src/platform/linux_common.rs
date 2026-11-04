//! Pieces every Linux backend shares.

use std::{fs, path::Path};

use super::Battery;

const SUPPLIES: &str = "/sys/class/power_supply";

fn read(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
}

/// Every power supply of type "Mains" is offline. Machines without a battery,
/// or whose adapter isn't listed, count as plugged in.
pub fn on_battery() -> bool {
    let Ok(entries) = fs::read_dir(SUPPLIES) else {
        return false;
    };
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

/// Environment that makes a GL/Vulkan client render on the discrete GPU, or empty when there
/// isn't one (or `KINORA_GPU=integrated`). NVIDIA uses its render offload variables; Mesa
/// drivers (AMD, Intel Arc) use `DRI_PRIME`.
pub fn discrete_gpu_env() -> Vec<(&'static str, &'static str)> {
    if std::env::var("KINORA_GPU").as_deref() == Ok("integrated") {
        return Vec::new();
    }
    // (vendor id, is the boot GPU) for every GPU, skipping connector entries like card1-DP-1.
    let gpus: Vec<(String, bool)> = fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with("card") && n[4..].bytes().all(|b| b.is_ascii_digit())
        })
        .filter_map(|e| {
            let dev = e.path().join("device");
            Some((read(&dev, "vendor")?, read(&dev, "boot_vga").as_deref() == Some("1")))
        })
        .collect();
    if gpus.len() < 2 {
        return Vec::new();
    }
    if gpus.iter().any(|(vendor, _)| vendor == "0x10de") {
        return vec![
            ("__NV_PRIME_RENDER_OFFLOAD", "1"),
            ("__GLX_VENDOR_LIBRARY_NAME", "nvidia"),
            ("__VK_LAYER_NV_optimus", "NVIDIA_only"),
            ("__EGL_VENDOR_LIBRARY_FILENAMES", "/usr/share/glvnd/egl_vendor.d/10_nvidia.json"),
        ];
    }
    if gpus.iter().any(|(_, boot)| !boot) {
        return vec![("DRI_PRIME", "1")];
    }
    Vec::new()
}
