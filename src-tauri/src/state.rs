use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

use crate::{desktop::Host, monitors::MonitorInfo};

pub const STORE: &str = "kinora.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    #[default]
    Cover,
    Contain,
    Fill,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KinoraState {
    pub current: Option<String>, // video path
    pub paused: bool,            // user pause
    pub auto_paused: bool,       // paused by fullscreen/battery watcher
    pub muted: bool,
    pub fit: Fit,
}

/// Written by the settings sheet through the store plugin's JS API.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub pause_on_fullscreen: bool,
    pub pause_on_battery: bool,
    pub start_muted: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { pause_on_fullscreen: true, pause_on_battery: false, start_muted: true }
    }
}

/// Native bits that only Rust cares about.
#[derive(Default)]
pub struct Runtime {
    pub host: Option<Host>,
    pub monitors: Vec<MonitorInfo>,
}

#[derive(Default)]
pub struct AppState {
    pub state: Mutex<KinoraState>,
    pub runtime: Mutex<Runtime>,
}

pub fn snapshot(app: &AppHandle) -> KinoraState {
    app.state::<AppState>().state.lock().unwrap().clone()
}

/// Mutate the state, then tell the control panel and tray about it.
pub fn update(app: &AppHandle, f: impl FnOnce(&mut KinoraState)) -> KinoraState {
    let s = {
        let app_state = app.state::<AppState>();
        let mut s = app_state.state.lock().unwrap();
        f(&mut s);
        s.clone()
    };
    let _ = app.emit_to("main", "kinora:state", &s);
    crate::tray::refresh(app, &s);
    s
}

pub fn settings(app: &AppHandle) -> Settings {
    app.store(STORE)
        .ok()
        .and_then(|s| s.get("settings"))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn load_saved(app: &AppHandle) -> (Option<String>, Fit) {
    let Ok(store) = app.store(STORE) else { return (None, Fit::default()) };
    let current = store.get("current").and_then(|v| v.as_str().map(String::from));
    let fit = store.get("fit").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    (current, fit)
}

pub fn save(app: &AppHandle, s: &KinoraState) {
    if let Ok(store) = app.store(STORE) {
        store.set("current", json!(s.current));
        store.set("fit", json!(s.fit));
        let _ = store.save();
    }
}
