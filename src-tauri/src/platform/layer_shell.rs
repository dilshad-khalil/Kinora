//! Wayland compositors with wlr-layer-shell (Hyprland, sway, ...): the wallpaper window becomes
//! a layer-shell surface on the background layer, click-through, behind everything.
//!
//! Limits for now: a layer-shell surface covers one output, so only one monitor gets the
//! wallpaper (the primary, or the one named in `KINORA_MONITOR`). Debug overrides:
//! `KINORA_LAYER` (background, bottom or top) and `KINORA_MONITOR` (a monitor name).

use gtk::prelude::*;
use gtk_layer_shell::{Edge, Layer, LayerShell};
use tauri::WebviewWindow;

use super::{AudioMeter, Battery, MediaSource, Platform, Rect, linux_common};

pub struct LayerShellPlatform;

/// GTK is main-thread only; `attach` runs on a worker, so it hops over and waits.
fn on_main_thread(win: &WebviewWindow) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let w = win.clone();
    win.run_on_main_thread(move || {
        let _ = tx.send(configure(&w));
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

fn configure(win: &WebviewWindow) -> Result<(), String> {
    let gtk_win = win.gtk_window().map_err(|e| e.to_string())?;
    // Layer-shell has to be set up before the window is mapped. It isn't realized yet when
    // Tauri builds it hidden, but drop the native window if that ever changes.
    if gtk_win.is_realized() {
        gtk_win.hide();
        gtk_win.unrealize();
    }
    gtk_win.init_layer_shell();

    gtk_win.set_layer(match std::env::var("KINORA_LAYER").as_deref() {
        Ok("bottom") => Layer::Bottom,
        Ok("top") => Layer::Top,
        _ => Layer::Background,
    });
    gtk_win.set_namespace("kinora-wallpaper");
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        gtk_win.set_anchor(edge, true);
    }
    gtk_win.set_exclusive_zone(-1); // ignore panels; cover the whole output
    gtk_win.set_keyboard_interactivity(false);

    if let Some(monitor) = pick_monitor(&gtk_win.display()) {
        gtk_win.set_monitor(&monitor);
    }

    // An empty input region lets clicks through to whatever is on the desktop.
    gtk_win.input_shape_combine_region(Some(&gtk::cairo::Region::create()));
    gtk_win.show_all();
    Ok(())
}

/// The monitor named in `KINORA_MONITOR`, else the primary, else the first.
fn pick_monitor(display: &gtk::gdk::Display) -> Option<gtk::gdk::Monitor> {
    let named = std::env::var("KINORA_MONITOR").ok().and_then(|name| {
        (0..display.n_monitors())
            .filter_map(|i| display.monitor(i))
            .find(|m| m.model().is_some_and(|model| model.as_str() == name))
    });
    named
        .or_else(|| display.primary_monitor())
        .or_else(|| display.monitor(0))
}

impl Platform for LayerShellPlatform {
    fn name(&self) -> &'static str {
        "layer-shell"
    }

    fn prepare(&self) -> Result<(), String> {
        Ok(())
    }

    fn attach(&self, win: &WebviewWindow, _bounds: Rect) -> Result<(), String> {
        on_main_thread(win)
    }

    fn host_alive(&self) -> bool {
        true
    }

    fn detach(&self) {}

    fn static_wallpaper(&self) -> Option<String> {
        None
    }

    fn refresh_static_wallpaper(&self) {}

    fn fullscreen_app_open(&self) -> bool {
        false
    }

    fn on_battery(&self) -> bool {
        linux_common::on_battery()
    }

    fn battery(&self) -> Option<Battery> {
        linux_common::battery()
    }

    fn media_source(&self) -> Option<Box<dyn MediaSource>> {
        None
    }

    fn audio_meter(&self) -> Option<Box<dyn AudioMeter>> {
        None
    }
}
