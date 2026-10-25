//! Everything that differs per operating system or desktop environment sits behind `Platform`.
//! Shared code asks for behavior (attach a window, is a fullscreen app open, what's playing)
//! and never names a platform type.

use std::sync::Arc;

use tauri::WebviewWindow;

#[cfg(not(windows))]
mod stub;
#[cfg(windows)]
mod win;

/// Physical pixels, virtual-screen coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// The smallest rect containing both.
    pub fn union(self, o: Rect) -> Rect {
        let (x, y) = (self.x.min(o.x), self.y.min(o.y));
        Rect {
            x,
            y,
            w: (self.x + self.w).max(o.x + o.w) - x,
            h: (self.y + self.h).max(o.y + o.h) - y,
        }
    }
}

pub struct Battery {
    pub percent: Option<u8>,
    pub charging: bool,
    pub plugged_in: bool,
}

pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Identifier of the player app.
    pub app: String,
    /// "playing", "paused", "stopped", "changing" or "closed".
    pub status: String,
    pub playing: bool,
    /// Seconds, already moved forward to now when playing.
    pub position: f64,
    pub duration: f64,
    /// A data URL.
    pub artwork: Option<String>,
}

/// What's playing right now. Created and used on one thread.
pub trait MediaSource {
    /// None when nothing is playing.
    fn current(&mut self) -> Option<MediaInfo>;
}

/// Loudness of the default output. Created and used on one thread.
pub trait AudioMeter {
    /// (peak, per-channel peaks), each 0..1. None when the device is gone.
    fn levels(&mut self) -> Option<(f32, Vec<f32>)>;
}

pub trait Platform: Send + Sync {
    /// For logs.
    fn name(&self) -> &'static str;

    /// Find or set up whatever wallpaper windows get pinned to. Runs before the window is built,
    /// so a failure doesn't leave a stray window behind.
    fn prepare(&self) -> Result<(), String>;
    /// Pin a freshly built, still hidden window to the desktop covering `bounds`, then show it.
    fn attach(&self, win: &WebviewWindow, bounds: Rect) -> Result<(), String>;
    /// False once the desktop host is gone (Explorer restarted) and windows must be rebuilt.
    fn host_alive(&self) -> bool;
    /// Let go of the desktop host and restore the static wallpaper.
    fn detach(&self);

    /// The current static wallpaper image, if there is one (not a solid color).
    fn static_wallpaper(&self) -> Option<String>;
    /// Re-apply the static wallpaper so no frozen video frame is left behind.
    fn refresh_static_wallpaper(&self);

    /// A foreground app covers a whole monitor.
    fn fullscreen_app_open(&self) -> bool;
    fn on_battery(&self) -> bool;
    /// None on machines without a battery.
    fn battery(&self) -> Option<Battery>;

    /// Call from the thread that will use it.
    fn media_source(&self) -> Option<Box<dyn MediaSource>>;
    /// Call from the thread that will use it.
    fn audio_meter(&self) -> Option<Box<dyn AudioMeter>>;
}

/// Pick the backend for this machine. Desktop-environment detection for Linux goes here.
pub fn detect() -> Arc<dyn Platform> {
    #[cfg(windows)]
    let platform: Arc<dyn Platform> = Arc::new(win::WindowsPlatform::default());
    #[cfg(not(windows))]
    let platform: Arc<dyn Platform> = Arc::new(stub::StubPlatform);
    eprintln!("kinora: platform: {}", platform.name());
    platform
}
