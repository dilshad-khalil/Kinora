use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::{
    monitors::{self, MonitorInfo},
    providers, rotation,
    state::{self, Fit, KinoraState},
    wallpaper,
    widgets::{self, Instance, WidgetsState},
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
pub fn next_wallpaper(app: AppHandle) -> KinoraState {
    rotation::next(&app);
    state::snapshot(&app)
}

/// The rotation list or settings were saved: restart the countdown and refresh the tray.
#[tauri::command]
pub fn rotation_changed(app: AppHandle) {
    rotation::mark_changed();
    crate::tray::refresh(&app, &state::snapshot(&app));
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> KinoraState {
    state::snapshot(&app)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    name: &'static str,
    /// The desktop plays the video itself, so the webview never has to decode it.
    native_playback: bool,
}

#[tauri::command]
pub fn platform_info(app: AppHandle) -> PlatformInfo {
    let platform = state::platform(&app);
    PlatformInfo { name: platform.name(), native_playback: platform.remote_player().is_some() }
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

#[tauri::command]
pub fn static_wallpaper(app: AppHandle) -> Option<String> {
    state::platform(&app).static_wallpaper()
}

#[tauri::command]
pub fn get_widgets(app: AppHandle) -> WidgetsState {
    widgets::snapshot(&app)
}

/// Rescan the widgets folder and refresh every window (after the user edits a widget).
#[tauri::command]
pub fn reload_widgets(app: AppHandle) -> WidgetsState {
    widgets::notify(&app)
}

#[tauri::command]
pub async fn add_widget(app: AppHandle, widget: String) -> Result<WidgetsState, String> {
    let s = widgets::add(&app, &widget)?;
    wallpaper::sync_presence(&app)?;
    Ok(s)
}

#[tauri::command]
pub fn update_widget(app: AppHandle, instance: Instance) -> WidgetsState {
    widgets::update(&app, instance)
}

#[tauri::command]
pub async fn remove_widget(app: AppHandle, id: String) -> Result<WidgetsState, String> {
    let s = widgets::remove(&app, &id);
    wallpaper::sync_presence(&app)?;
    Ok(s)
}

#[tauri::command]
pub fn import_widget(app: AppHandle, path: String) -> Result<WidgetsState, String> {
    widgets::import(&app, &path)
}

#[tauri::command]
pub async fn uninstall_widget(app: AppHandle, widget: String) -> Result<WidgetsState, String> {
    let s = widgets::uninstall(&app, &widget)?;
    wallpaper::sync_presence(&app)?;
    Ok(s)
}

#[tauri::command]
pub fn open_widgets_folder(app: AppHandle) -> Result<(), String> {
    let dir = widgets::dir(&app);
    let _ = std::fs::create_dir_all(&dir);
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_widget_editor(app: AppHandle) -> Result<(), String> {
    widgets::open_editor(&app)
}

#[tauri::command]
pub fn close_widget_editor(app: AppHandle) {
    widgets::close_editor(&app);
}

/// The latest value of a live-data topic, for pages that open after it was sent.
#[tauri::command]
pub fn widget_data(topic: String) -> Option<serde_json::Value> {
    providers::latest(&topic)
}

/// Relayed by the page hosting the widget; the widget itself can't call commands.
#[tauri::command]
pub fn widget_notify(app: AppHandle, instance: String, title: String, body: String) -> Result<(), String> {
    widgets::notify_from(&app, &instance, &title, &body)
}
