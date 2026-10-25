//! Foreground-window and power queries.

use ::windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
    UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetShellWindow, GetWindowRect, GetWindowThreadProcessId,
        IsZoomed,
    },
};

use crate::platform::Battery;

/// The foreground window covers its whole monitor and isn't the desktop itself.
pub fn fullscreen_app_open() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        // Maximized windows aren't fullscreen, even frameless ones whose rect fills the monitor.
        if fg.is_invalid() || fg == GetShellWindow() || IsZoomed(fg).as_bool() {
            return false;
        }
        // Kinora's own windows (the widget arrange overlay covers every monitor).
        let mut pid = 0;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == std::process::id() {
            return false;
        }
        let mut class = [0u16; 64];
        let len = GetClassNameW(fg, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..len]);
        if class == "Progman" || class == "WorkerW" {
            return false;
        }

        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST), &mut mi).as_bool() {
            return false;
        }
        let m = mi.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

pub fn on_battery() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status).is_ok() && status.ACLineStatus == 0 }
}

pub fn battery() -> Option<Battery> {
    let mut s = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut s) }.is_err() || s.BatteryFlag & 128 != 0 || s.BatteryFlag == 255 {
        return None; // no battery (desktop) or unknown
    }
    Some(Battery {
        percent: (s.BatteryLifePercent != 255).then_some(s.BatteryLifePercent),
        charging: s.BatteryFlag & 8 != 0,
        plugged_in: s.ACLineStatus == 1,
    })
}
