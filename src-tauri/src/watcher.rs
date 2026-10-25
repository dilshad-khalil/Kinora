//! One background thread: auto-pause, Explorer restarts, monitor changes.

use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};

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
    if wallpaper::is_building() {
        return;
    }

    // Only while the wallpaper window is up (a video or widgets are showing).
    let platform = state::platform(app);
    let (attached, last_monitors) = {
        let app_state = app.state::<AppState>();
        let runtime = app_state.runtime.lock().unwrap();
        (runtime.attached, runtime.monitors.clone())
    };
    if !attached {
        return;
    }
    if !platform.host_alive() || monitors::list(app) != last_monitors {
        eprintln!("kinora: desktop or monitors changed, rebuilding wallpaper windows");
        if let Err(e) = wallpaper::rebuild(app) {
            eprintln!("kinora: rebuild failed: {e}");
        }
        return;
    }

    let settings = state::settings(app);
    let auto = (settings.pause_on_fullscreen && platform.fullscreen_app_open())
        || (settings.pause_on_battery && platform.on_battery());
    if auto != s.auto_paused {
        let s = state::update(app, |s| s.auto_paused = auto);
        wallpaper::sync_playback(app, &s);
    }
}
