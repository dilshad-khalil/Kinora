use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewWindowBuilder, Wry,
};

use crate::{
    state::{self, KinoraState},
    wallpaper,
};

/// Menu items whose text/enabled state follows the playback state.
pub struct TrayItems {
    pause: MenuItem<Wry>,
    mute: MenuItem<Wry>,
    stop: MenuItem<Wry>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Kinora", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause", false, None::<&str>)?;
    let mute = MenuItem::with_id(app, "mute", "Unmute", false, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop wallpaper", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &PredefinedMenuItem::separator(app)?,
            &pause,
            &mute,
            &stop,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Kinora")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "pause" => {
                let paused = state::snapshot(app).paused;
                wallpaper::set_paused(app, !paused);
            }
            "mute" => {
                let muted = state::snapshot(app).muted;
                wallpaper::set_muted(app, !muted);
            }
            "stop" => {
                wallpaper::stop(app);
            }
            "quit" => {
                wallpaper::stop(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;

    app.manage(TrayItems { pause, mute, stop });
    Ok(())
}

pub fn refresh(app: &AppHandle, s: &KinoraState) {
    let Some(items) = app.try_state::<TrayItems>() else { return };
    let active = s.current.is_some();
    let _ = items.pause.set_text(if s.paused { "Resume" } else { "Pause" });
    let _ = items.mute.set_text(if s.muted { "Unmute" } else { "Mute" });
    let _ = items.pause.set_enabled(active);
    let _ = items.mute.set_enabled(active);
    let _ = items.stop.set_enabled(active);
}

pub fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return;
    }
    // Built off the main thread: creating a window from an event handler can deadlock on Windows.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = create_main(&app) {
            eprintln!("kinora: couldn't open the control panel: {e}");
        }
    });
}

/// Build the control panel from its `tauri.conf.json` entry (which has `create: false`).
pub fn create_main(app: &AppHandle) -> tauri::Result<()> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .expect("main window config")
        .clone();
    let win = WebviewWindowBuilder::from_config(app, &config)?.build()?;
    win.show()?;
    win.set_focus()
}
