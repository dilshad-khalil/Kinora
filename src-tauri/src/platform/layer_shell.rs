//! SPIKE: pin the wallpaper window to a wlr-layer-shell surface. Throwaway.

use gtk::prelude::*;
use gtk_layer_shell::{Edge, Layer, LayerShell};
use tauri::WebviewWindow;

use super::{AudioMeter, Battery, MediaSource, Platform, Rect};

pub struct LayerShellPlatform;

fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}

fn attach_on_main(win: &WebviewWindow) -> Result<(), String> {
    let gtk_win = win.gtk_window().map_err(|e| e.to_string())?;
    eprintln!("spike: realized before init = {}", gtk_win.is_realized());
    if gtk_win.is_realized() {
        gtk_win.hide();
        gtk_win.unrealize();
        eprintln!("spike: unrealized, realized now = {}", gtk_win.is_realized());
    }

    gtk_win.init_layer_shell();

    let layer = match env("KINORA_LAYER", "background").as_str() {
        "bottom" => Layer::Bottom,
        "top" => Layer::Top,
        _ => Layer::Background,
    };
    gtk_win.set_layer(layer);
    gtk_win.set_namespace("kinora-wallpaper");
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        gtk_win.set_anchor(edge, true);
    }
    gtk_win.set_exclusive_zone(-1);
    gtk_win.set_keyboard_interactivity(false);

    let x: i32 = env("KINORA_MON_X", "0").parse().unwrap_or(0);
    let display = gtk_win.display();
    match display.monitor_at_point(x + 1, 1) {
        Some(m) => {
            let g = m.geometry();
            eprintln!("spike: monitor {:?} at {},{} {}x{}", m.model(), g.x(), g.y(), g.width(), g.height());
            gtk_win.set_monitor(&m);
        }
        None => eprintln!("spike: no monitor at x={x}"),
    }

    if env("KINORA_CLICKTHROUGH", "1") == "1" {
        gtk_win.input_shape_combine_region(Some(&gtk::cairo::Region::create()));
    }

    gtk_win.show_all();
    eprintln!("spike: shown, mapped = {}", gtk_win.is_mapped());
    Ok(())
}

impl Platform for LayerShellPlatform {
    fn name(&self) -> &'static str {
        "layer-shell (spike)"
    }

    fn prepare(&self) -> Result<(), String> {
        Ok(())
    }

    fn attach(&self, win: &WebviewWindow, _bounds: Rect) -> Result<(), String> {
        // GTK is main-thread only; this runs on a worker.
        let (tx, rx) = std::sync::mpsc::channel();
        let w = win.clone();
        win.run_on_main_thread(move || {
            let _ = tx.send(attach_on_main(&w));
        })
        .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
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
        false
    }
    fn battery(&self) -> Option<Battery> {
        None
    }
    fn media_source(&self) -> Option<Box<dyn MediaSource>> {
        None
    }
    fn audio_meter(&self) -> Option<Box<dyn AudioMeter>> {
        None
    }
}
