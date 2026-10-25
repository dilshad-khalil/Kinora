//! Windows: wallpaper windows live inside Explorer's desktop, data comes from WinRT and WASAPI.

use std::sync::Mutex;

use ::windows::Win32::Foundation::{HWND, RECT};
use tauri::WebviewWindow;

use super::{AudioMeter, Battery, MediaSource, Platform, Rect};
use desktop::Host;

mod audio;
mod desktop;
mod media;
mod shell;

#[derive(Default)]
pub struct WindowsPlatform {
    host: Mutex<Option<Host>>,
}

impl Platform for WindowsPlatform {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn prepare(&self) -> Result<(), String> {
        *self.host.lock().unwrap() = Some(desktop::find_host()?);
        Ok(())
    }

    fn attach(&self, win: &WebviewWindow, bounds: Rect) -> Result<(), String> {
        let host = self.host.lock().unwrap().ok_or("The desktop host wasn't prepared.")?;
        let hwnd: HWND = win.hwnd().map_err(|e| e.to_string())?;
        let rect = RECT {
            left: bounds.x,
            top: bounds.y,
            right: bounds.x + bounds.w,
            bottom: bounds.y + bounds.h,
        };
        desktop::attach(hwnd, &host, rect)
    }

    fn host_alive(&self) -> bool {
        self.host.lock().unwrap().is_some_and(|h| h.is_alive())
    }

    fn detach(&self) {
        *self.host.lock().unwrap() = None;
        desktop::refresh_static_wallpaper();
    }

    fn static_wallpaper(&self) -> Option<String> {
        desktop::static_wallpaper_path()
    }

    fn refresh_static_wallpaper(&self) {
        desktop::refresh_static_wallpaper();
    }

    fn fullscreen_app_open(&self) -> bool {
        shell::fullscreen_app_open()
    }

    fn on_battery(&self) -> bool {
        shell::on_battery()
    }

    fn battery(&self) -> Option<Battery> {
        shell::battery()
    }

    fn media_source(&self) -> Option<Box<dyn MediaSource>> {
        Some(Box::new(media::Media::default()))
    }

    fn audio_meter(&self) -> Option<Box<dyn AudioMeter>> {
        audio::Meter::open().map(|m| Box::new(m) as Box<dyn AudioMeter>)
    }
}
