//! One borderless player window spanning all monitors, pinned behind the desktop icons.
//! A single window means a single WebView2 renderer, however many monitors there are;
//! the page lays out one <video> per monitor.

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
    monitors,
    platform::PlayerState,
    state::{self, AppState, Fit, KinoraState},
    widgets,
};

const PREFIX: &str = "wallpaper-";
static GENERATION: AtomicU32 = AtomicU32::new(0);
static BUILD: Mutex<()> = Mutex::new(());
static BUILDING: AtomicBool = AtomicBool::new(false);

pub fn is_building() -> bool {
    BUILDING.load(Ordering::Relaxed)
}

/// Tell whatever shows the wallpaper that something changed. A platform that plays the video
/// itself gets the whole state again; otherwise the event goes to the wallpaper windows only,
/// never the control panel.
fn emit<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if state::platform(app).remote_player().is_some() {
        push_remote(app);
        return;
    }
    let _ = app.emit_filter(event, payload, |t| {
        matches!(t, EventTarget::WebviewWindow { label } if label.starts_with(PREFIX))
    });
}

/// Send the current state to a platform that plays the video itself. No-op elsewhere.
pub fn push_remote(app: &AppHandle) {
    let platform = state::platform(app);
    let Some(remote) = platform.remote_player() else { return };
    let s = state::snapshot(app);
    let player = PlayerState {
        video: s.current.clone(),
        fit: s.fit,
        muted: s.muted,
        paused: s.paused || s.auto_paused,
        pause_on_fullscreen: state::settings(app).pause_on_fullscreen,
    };
    app.state::<AppState>().runtime.lock().unwrap().attached = player.video.is_some();
    if let Err(e) = remote.apply(&player) {
        eprintln!("kinora: couldn't update the desktop wallpaper: {e}");
    }
}

fn destroy_windows(app: &AppHandle) {
    for (label, win) in app.webview_windows() {
        if label.starts_with(PREFIX) {
            let _ = win.destroy();
        }
    }
}

/// (Re)create the player window and attach it to the desktop.
fn build(app: &AppHandle) -> Result<(), String> {
    if state::platform(app).remote_player().is_some() {
        return Ok(()); // the desktop plays the video; there is no window to build
    }
    let _guard = BUILD.lock().unwrap();
    BUILDING.store(true, Ordering::Relaxed);
    let result = build_inner(app);
    BUILDING.store(false, Ordering::Relaxed);
    result
}

fn build_inner(app: &AppHandle) -> Result<(), String> {
    destroy_windows(app);
    let platform = state::platform(app);
    platform.prepare()?;
    let monitors = monitors::list(app);
    // Bounding box of all monitors, in physical virtual-screen coordinates.
    let bounds = monitors.iter().map(|m| m.rect()).reduce(|a, b| a.union(b)).ok_or("No monitors found.")?;

    // Fresh label each build, so a window that is still being destroyed never collides.
    let label = format!("{PREFIX}{}", GENERATION.fetch_add(1, Ordering::Relaxed));
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
    if let Err(e) = platform.attach(&win, bounds) {
        let _ = win.destroy();
        return Err(e);
    }

    let app_state = app.state::<AppState>();
    let mut runtime = app_state.runtime.lock().unwrap();
    runtime.attached = true;
    runtime.monitors = monitors;
    Ok(())
}

fn has_windows(app: &AppHandle) -> bool {
    app.webview_windows().keys().any(|l| l.starts_with(PREFIX))
}

pub fn set_wallpaper(app: &AppHandle, path: String) -> Result<KinoraState, String> {
    if !Path::new(&path).is_file() {
        return Err(format!("File not found: {path}"));
    }
    if !has_windows(app) {
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
    let s = state::update(app, |s| {
        s.current = None;
        s.paused = false;
    });
    state::save(app, &s);
    if widgets::has_instances(app) {
        // Keep the window for the widgets; the player shows the static wallpaper instead.
        emit(app, "player:load", s.clone());
    } else {
        close(app);
    }
    s
}

/// Tear the window down and give the desktop back to Explorer.
fn close(app: &AppHandle) {
    destroy_windows(app);
    state::platform(app).detach();
    app.state::<AppState>().runtime.lock().unwrap().attached = false;
}

/// The window exists while there's a video or a widget to show. Call after widgets change.
pub fn sync_presence(app: &AppHandle) -> Result<(), String> {
    let needed = state::snapshot(app).current.is_some() || widgets::has_instances(app);
    if needed && !has_windows(app) {
        rebuild(app)?;
    } else if !needed && has_windows(app) {
        close(app);
    }
    Ok(())
}

/// Full shutdown (tray → Quit).
pub fn quit(app: &AppHandle) {
    close(app);
}

/// Push the effective play/pause state to the players.
pub fn sync_playback(app: &AppHandle, s: &KinoraState) {
    let event = if s.paused || s.auto_paused { "player:pause" } else { "player:play" };
    emit(app, event, ());
    // Widgets ignore the user's pause, but sleep while a fullscreen app is open.
    emit(app, "widgets:visible", !s.auto_paused);
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
