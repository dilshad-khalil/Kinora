//! One borderless player window per monitor, pinned behind the desktop icons.

use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Mutex,
    },
};

use serde::Serialize;
use tauri::{
    webview::Color, AppHandle, Emitter, EventTarget, Manager, WebviewUrl, WebviewWindowBuilder,
};

use crate::{
    desktop, monitors,
    state::{self, AppState, Fit, KinoraState},
};

const PREFIX: &str = "wallpaper-";
static GENERATION: AtomicU32 = AtomicU32::new(0);
static BUILD: Mutex<()> = Mutex::new(());
static BUILDING: AtomicBool = AtomicBool::new(false);

pub fn is_building() -> bool {
    BUILDING.load(Ordering::Relaxed)
}

/// Send an event to the wallpaper windows only, never the control panel.
fn emit<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    let _ = app.emit_filter(event, payload, |t| {
        matches!(t, EventTarget::WebviewWindow { label } if label.starts_with(PREFIX))
    });
}

fn destroy_windows(app: &AppHandle) {
    for (label, win) in app.webview_windows() {
        if label.starts_with(PREFIX) {
            let _ = win.destroy();
        }
    }
}

/// (Re)create one player window per monitor and attach them to the desktop.
fn build(app: &AppHandle) -> Result<(), String> {
    let _guard = BUILD.lock().unwrap();
    BUILDING.store(true, Ordering::Relaxed);
    let result = build_inner(app);
    BUILDING.store(false, Ordering::Relaxed);
    result
}

fn build_inner(app: &AppHandle) -> Result<(), String> {
    destroy_windows(app);
    let host = desktop::find_host()?;
    let monitors = monitors::list(app);
    // Fresh labels each build, so a window that is still being destroyed never collides.
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed);

    for (i, monitor) in monitors.iter().enumerate() {
        let label = format!("{PREFIX}{generation}-{i}");
        let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("wallpaper.html".into()))
            .title("Kinora wallpaper")
            .decorations(false)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .shadow(false)
            .resizable(false)
            .background_color(Color(0, 0, 0, 255))
            .build()
            .map_err(|e| e.to_string())?;
        let hwnd = win.hwnd().map_err(|e| e.to_string())?;
        desktop::attach(hwnd, &host, monitor.rect())?;
    }

    let app_state = app.state::<AppState>();
    let mut runtime = app_state.runtime.lock().unwrap();
    runtime.host = Some(host);
    runtime.monitors = monitors;
    Ok(())
}

pub fn set_wallpaper(app: &AppHandle, path: String) -> Result<KinoraState, String> {
    if !Path::new(&path).is_file() {
        return Err(format!("File not found: {path}"));
    }
    let has_windows = app.webview_windows().keys().any(|l| l.starts_with(PREFIX));
    if !has_windows {
        build(app)?;
    }
    let s = state::update(app, |s| {
        s.current = Some(path);
        s.paused = false;
    });
    state::save(app, &s);
    emit(app, "player:load", s.clone());
    Ok(s)
}

/// Rebuild the windows for the current video (Explorer restarted, monitors changed).
pub fn rebuild(app: &AppHandle) -> Result<(), String> {
    build(app)?;
    emit(app, "player:load", state::snapshot(app));
    Ok(())
}

pub fn stop(app: &AppHandle) -> KinoraState {
    destroy_windows(app);
    desktop::refresh_static_wallpaper();
    app.state::<AppState>().runtime.lock().unwrap().host = None;
    let s = state::update(app, |s| {
        s.current = None;
        s.paused = false;
    });
    state::save(app, &s);
    s
}

/// Push the effective play/pause state to the players.
pub fn sync_playback(app: &AppHandle, s: &KinoraState) {
    let event = if s.paused || s.auto_paused { "player:pause" } else { "player:play" };
    emit(app, event, ());
}

pub fn set_paused(app: &AppHandle, paused: bool) -> KinoraState {
    let s = state::update(app, |s| s.paused = paused);
    sync_playback(app, &s);
    s
}

pub fn set_muted(app: &AppHandle, muted: bool) -> KinoraState {
    let s = state::update(app, |s| s.muted = muted);
    emit(app, "player:mute", muted);
    s
}

pub fn set_fit(app: &AppHandle, fit: Fit) -> KinoraState {
    let s = state::update(app, |s| s.fit = fit);
    state::save(app, &s);
    emit(app, "player:fit", fit);
    s
}
