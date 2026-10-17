//! One background thread: auto-pause, Explorer restarts, monitor changes.

use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};
use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
    UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetShellWindow, GetWindowRect, IsZoomed},
};

use crate::{
    monitors,
    state::{self, AppState},
    wallpaper,
};

pub fn spawn(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(1500));
        tick(&app);
    });
}

fn tick(app: &AppHandle) {
    let s = state::snapshot(app);
    if s.current.is_none() || wallpaper::is_building() {
        return;
    }

    let (host_alive, last_monitors) = {
        let app_state = app.state::<AppState>();
        let runtime = app_state.runtime.lock().unwrap();
        (runtime.host.is_some_and(|h| h.is_alive()), runtime.monitors.clone())
    };
    if !host_alive || monitors::list(app) != last_monitors {
        eprintln!("kinora: desktop or monitors changed, rebuilding wallpaper windows");
        if let Err(e) = wallpaper::rebuild(app) {
            eprintln!("kinora: rebuild failed: {e}");
        }
        return;
    }

    let settings = state::settings(app);
    let auto = (settings.pause_on_fullscreen && fullscreen_app_open())
        || (settings.pause_on_battery && on_battery());
    if auto != s.auto_paused {
        let s = state::update(app, |s| s.auto_paused = auto);
        wallpaper::sync_playback(app, &s);
    }
}

/// The foreground window covers its whole monitor and isn't the desktop itself.
fn fullscreen_app_open() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        // Maximized windows aren't fullscreen, even frameless ones whose rect fills the monitor.
        if fg.is_invalid() || fg == GetShellWindow() || IsZoomed(fg).as_bool() {
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

fn on_battery() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status).is_ok() && status.ACLineStatus == 0 }
}
