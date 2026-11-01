//! Hyprland: one `mpvpaper` per monitor plays the video on a wlr-layer-shell surface, the same
//! way linux-wallpaperengine gives every output its own surface. Kinora only starts, stops and
//! steers those processes; there is no wallpaper window.
//!
//! The surface sits on the `bottom` layer: above whatever the desktop draws as its background
//! (swaybg, hyprpaper), which stays untouched, and below every window. Override the layer with
//! `KINORA_LAYER=background|bottom|top`.

use std::{
    collections::BTreeSet,
    io::Write,
    os::unix::{net::UnixStream, process::CommandExt},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

use serde_json::Value;
use tauri::WebviewWindow;

use super::{
    AudioMeter, Battery, MediaSource, Platform, PlayerState, Rect, RemotePlayer, linux_common,
};
use crate::state::Fit;

/// A player that dies sooner than this didn't like the file; don't restart it in a loop.
const MIN_LIFETIME: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct HyprlandPlatform {
    inner: Mutex<Players>,
}

#[derive(Default)]
struct Players {
    slots: Vec<Slot>,
    /// Monitors whose player exited right away, kept until the state changes.
    failed: BTreeSet<String>,
    applied: Option<PlayerState>,
    /// The last error reported, so the watcher's repeated calls don't repeat it.
    last_error: Option<String>,
}

struct Slot {
    monitor: String,
    child: Child,
    started: Instant,
}

impl Slot {
    fn socket(&self) -> PathBuf {
        socket(&self.monitor)
    }
}

fn socket(monitor: &str) -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(format!("kinora-mpv-{monitor}.sock"))
}

fn hyprctl(what: &str) -> Option<Value> {
    let out = Command::new("hyprctl").args([what, "-j"]).output().ok()?;
    out.status
        .success()
        .then(|| serde_json::from_slice(&out.stdout).ok())
        .flatten()
}

/// Names of the enabled monitors.
fn monitors() -> Vec<String> {
    hyprctl("monitors")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter(|m| m["disabled"] != true)
        .filter_map(|m| m["name"].as_str().map(String::from))
        .collect()
}

fn mpv_options(state: &PlayerState, sock: &PathBuf) -> String {
    let mut opts = vec![
        "loop-file=inf".to_string(),
        "no-osc".into(),
        "no-input-default-bindings".into(),
        "hwdec=auto-safe".into(),
        format!("input-ipc-server={}", sock.display()),
        format!("mute={}", yes_no(state.muted)),
        format!("pause={}", yes_no(state.paused)),
    ];
    match state.fit {
        Fit::Cover => opts.push("panscan=1.0".into()),
        Fit::Contain => {}
        Fit::Fill => opts.push("keepaspect=no".into()),
    }
    opts.join(" ")
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn spawn(monitor: &str, video: &str, state: &PlayerState) -> Result<Slot, String> {
    let sock = socket(monitor);
    let _ = std::fs::remove_file(&sock);
    let layer = match std::env::var("KINORA_LAYER").as_deref() {
        Ok("background") => "background",
        Ok("top") => "top",
        _ => "bottom",
    };
    let mut cmd = Command::new("mpvpaper");
    cmd.args(["-l", layer, "-o", &mpv_options(state, &sock), monitor, video])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .envs(linux_common::discrete_gpu_env());
    // If Kinora dies without cleaning up, the players go with it.
    unsafe {
        cmd.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            Ok(())
        });
    }
    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "mpvpaper isn't installed. On Arch: paru -S mpvpaper (or yay -S mpvpaper), then pick the wallpaper again.".to_string()
        } else {
            format!("couldn't start mpvpaper: {e}")
        }
    })?;
    Ok(Slot { monitor: monitor.into(), child, started: Instant::now() })
}

fn kill(mut slot: Slot) {
    let _ = slot.child.kill();
    let _ = slot.child.wait();
    let _ = std::fs::remove_file(slot.socket());
}

/// Set an mpv property on a running player.
fn set_property(slot: &Slot, name: &str, value: bool) {
    if let Ok(mut s) = UnixStream::connect(slot.socket()) {
        let cmd = serde_json::json!({ "command": ["set_property", name, value] });
        let _ = writeln!(s, "{cmd}");
    }
}

impl Players {
    fn stop_all(&mut self) {
        for slot in self.slots.drain(..) {
            kill(slot);
        }
        self.failed.clear();
        self.applied = None;
        self.last_error = None;
    }
}

impl Platform for HyprlandPlatform {
    fn name(&self) -> &'static str {
        "hyprland"
    }

    fn prepare(&self) -> Result<(), String> {
        Ok(())
    }

    fn attach(&self, _win: &WebviewWindow, _bounds: Rect) -> Result<(), String> {
        Err("mpvpaper plays the wallpaper itself; there is no window to attach.".into())
    }

    fn host_alive(&self) -> bool {
        true
    }

    fn detach(&self) {
        self.inner.lock().unwrap().stop_all();
    }

    fn remote_player(&self) -> Option<&dyn RemotePlayer> {
        Some(self)
    }

    fn static_wallpaper(&self) -> Option<String> {
        None
    }

    fn refresh_static_wallpaper(&self) {}

    /// A fullscreen window on a workspace that is showing on some monitor.
    fn fullscreen_app_open(&self) -> bool {
        let visible: BTreeSet<i64> = hyprctl("monitors")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|m| m["activeWorkspace"]["id"].as_i64())
            .collect();
        hyprctl("clients")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .any(|c| {
                c["mapped"] != false
                    && c["fullscreen"].as_i64().unwrap_or(0) > 0
                    && c["workspace"]["id"]
                        .as_i64()
                        .is_some_and(|id| visible.contains(&id))
            })
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

impl RemotePlayer for HyprlandPlatform {
    fn apply(&self, state: &PlayerState) -> Result<(), String> {
        let mut guard = self.inner.lock().unwrap();
        let players = &mut *guard;
        let Some(video) = &state.video else {
            players.stop_all();
            return Ok(());
        };

        // A different video or fit needs fresh players; pause and mute can be changed live.
        let restart = players.applied.as_ref().is_none_or(|a| {
            a.video != state.video || a.fit != state.fit
        });
        if restart {
            players.stop_all();
        } else if players.applied.as_ref() != Some(state) {
            let applied = players.applied.clone().unwrap();
            for slot in &players.slots {
                if applied.paused != state.paused {
                    set_property(slot, "pause", state.paused);
                }
                if applied.muted != state.muted {
                    set_property(slot, "mute", state.muted);
                }
            }
        }
        players.applied = Some(state.clone());

        // Drop players that exited (or whose monitor is gone) and start missing ones.
        let wanted = monitors();
        let mut error = None;
        let mut kept = Vec::new();
        for mut slot in players.slots.drain(..) {
            let exited = slot.child.try_wait().ok().flatten().is_some();
            if !exited && wanted.contains(&slot.monitor) {
                kept.push(slot);
            } else {
                if exited && slot.started.elapsed() < MIN_LIFETIME {
                    players.failed.insert(slot.monitor.clone());
                    error = Some(format!(
                        "mpvpaper exited right away on {}. Check the video plays in mpv.",
                        slot.monitor
                    ));
                }
                kill(slot);
            }
        }
        players.slots = kept;
        players.failed.retain(|m| wanted.contains(m));

        for monitor in &wanted {
            if players.failed.contains(monitor) || players.slots.iter().any(|s| &s.monitor == monitor) {
                continue;
            }
            match spawn(monitor, video, state) {
                Ok(slot) => players.slots.push(slot),
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }
        match error {
            Some(e) if players.last_error.as_ref() != Some(&e) => {
                players.last_error = Some(e.clone());
                Err(e)
            }
            Some(_) => Ok(()),
            None => {
                players.last_error = None;
                Ok(())
            }
        }
    }
}
