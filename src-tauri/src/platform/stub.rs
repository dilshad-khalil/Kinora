//! Placeholder for systems without a backend yet: wallpaper windows are plain windows and the
//! system reports nothing. Lets the whole app run while a real backend is written.

use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

use super::{AudioMeter, Battery, MediaSource, Platform, Rect};

pub struct StubPlatform;

impl Platform for StubPlatform {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn prepare(&self) -> Result<(), String> {
        Ok(())
    }

    fn attach(&self, win: &WebviewWindow, bounds: Rect) -> Result<(), String> {
        win.set_position(PhysicalPosition::new(bounds.x, bounds.y)).map_err(|e| e.to_string())?;
        win.set_size(PhysicalSize::new(bounds.w as u32, bounds.h as u32)).map_err(|e| e.to_string())?;
        win.show().map_err(|e| e.to_string())
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
