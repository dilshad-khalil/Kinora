use std::path::Path;

use tauri::AppHandle;

use crate::{
    monitors::{self, MonitorInfo},
    state::{self, Fit, KinoraState},
    wallpaper,
};

// Async so window creation doesn't run on (and deadlock) the main thread.
#[tauri::command]
pub async fn set_wallpaper(app: AppHandle, path: String) -> Result<KinoraState, String> {
    wallpaper::set_wallpaper(&app, path)
}

#[tauri::command]
pub fn stop_wallpaper(app: AppHandle) -> KinoraState {
    wallpaper::stop(&app)
}

#[tauri::command]
pub fn set_paused(app: AppHandle, paused: bool) -> KinoraState {
    wallpaper::set_paused(&app, paused)
}

#[tauri::command]
pub fn set_muted(app: AppHandle, muted: bool) -> KinoraState {
    wallpaper::set_muted(&app, muted)
}

#[tauri::command]
pub fn set_fit(app: AppHandle, fit: Fit) -> KinoraState {
    wallpaper::set_fit(&app, fit)
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> KinoraState {
    state::snapshot(&app)
}

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    monitors::list(&app)
}

/// Library paths that no longer exist on disk.
#[tauri::command]
pub fn missing_files(paths: Vec<String>) -> Vec<String> {
    paths.into_iter().filter(|p| !Path::new(p).is_file()).collect()
}
