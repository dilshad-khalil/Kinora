//! Everything that differs per operating system or desktop environment sits behind `Platform`.
//! Shared code asks for behavior (attach a window, is a fullscreen app open, what's playing)
//! and never names a platform type.

use std::sync::Arc;

use tauri::WebviewWindow;

use crate::state::Fit;

#[cfg(target_os = "linux")]
mod kde;
#[cfg(target_os = "linux")]
mod layer_shell;
#[cfg(not(windows))]
mod linux_common;
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

/// Everything a platform that plays the video itself needs to know.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerState {
    /// None means Kinora has nothing to show, so the desktop's own wallpaper comes back.
    pub video: Option<String>,
    pub fit: Fit,
    pub muted: bool,
    /// Paused by the user or by the watcher.
    pub paused: bool,
    pub pause_on_fullscreen: bool,
}

/// A desktop that plays the video itself (a Plasma wallpaper plugin) instead of Kinora pinning
/// a window to it.
pub trait RemotePlayer: Send + Sync {
    /// Safe to call often; backends skip a state they already applied.
    fn apply(&self, state: &PlayerState) -> Result<(), String>;
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
    /// Set on platforms where the desktop plays the video; then no wallpaper window is built
    /// and `prepare`/`attach` are never called.
    fn remote_player(&self) -> Option<&dyn RemotePlayer> {
        None
    }

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
    let platform: Arc<dyn Platform> = detect_linux();
    eprintln!("kinora: platform: {}", platform.name());
    platform
}

#[cfg(not(windows))]
fn detect_linux() -> Arc<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        // An explicit choice wins, for testing.
        match std::env::var("KINORA_BACKEND").as_deref() {
            Ok("kde") => return Arc::new(kde::KdePlatform::default()),
            Ok("layer-shell") => return Arc::new(layer_shell::LayerShellPlatform),
            Ok("stub") => return Arc::new(stub::StubPlatform),
            _ => {}
        }
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
        let desktops: Vec<&str> = desktop.split(':').collect();
        if desktops.contains(&"kde") {
            return Arc::new(kde::KdePlatform::default());
        }
        if desktops.contains(&"hyprland") || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            return Arc::new(layer_shell::LayerShellPlatform);
        }
    }
    Arc::new(stub::StubPlatform)
}
