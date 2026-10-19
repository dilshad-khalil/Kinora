mod commands;
mod desktop;
mod monitors;
mod state;
mod tray;
mod wallpaper;
mod watcher;

use std::path::Path;

use tauri::{Emitter, Listener, Manager, RunEvent};
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

            // The control panel is created on demand and destroyed when closed, so it costs
            // no memory while Kinora sits in the tray. Login starts (--hidden) skip it.
            if !std::env::args().any(|a| a == "--hidden") {
                tray::create_main(&handle)?;
            }

            // Autostart stores this exe's path. Re-register so the entry follows the installed
            // app (and replaces any stale entry, e.g. one written by a dev build).
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                let autolaunch = handle.autolaunch();
                if autolaunch.is_enabled().unwrap_or(false) {
                    let _ = autolaunch.enable();
                }
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
        .build(tauri::generate_context!())
        .expect("error while building Kinora")
        .run(|_app, event| {
            // Closing the last window must not quit: Kinora lives in the tray.
            // Only an explicit exit (tray → Quit) carries an exit code.
            if let RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
