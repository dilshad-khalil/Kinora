mod commands;
mod desktop;
mod monitors;
mod state;
mod tray;
mod wallpaper;
mod watcher;

use std::path::Path;

use tauri::{Emitter, Listener, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch just focuses the running one.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();

            // Clear any frame left behind by a crash last time.
            desktop::refresh_static_wallpaper();

            let (current, fit) = state::load_saved(&handle);
            {
                let app_state = app.state::<AppState>();
                let mut s = app_state.state.lock().unwrap();
                s.fit = fit;
                s.muted = state::settings(&handle).start_muted;
            }

            tray::build(&handle)?;

            // Closing the window hides it to the tray; only tray → Quit exits.
            let main = app.get_webview_window("main").unwrap();
            let main_clone = main.clone();
            main.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = main_clone.hide();
                }
            });
            if !std::env::args().any(|a| a == "--hidden") {
                main.show()?;
            }

            let h = handle.clone();
            app.listen_any("player:error", move |_| {
                let _ = h.emit_to(
                    "main",
                    "kinora:error",
                    "This video couldn't be played. Try MP4 (H.264) or WebM.",
                );
            });

            // Restore the last wallpaper.
            if let Some(path) = current.filter(|p| Path::new(p).is_file()) {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = wallpaper::set_wallpaper(&h, path) {
                        eprintln!("kinora: restore failed: {e}");
                    }
                });
            }

            watcher::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_wallpaper,
            commands::stop_wallpaper,
            commands::set_paused,
            commands::set_muted,
            commands::set_fit,
            commands::get_state,
            commands::list_monitors,
            commands::missing_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kinora");
}
