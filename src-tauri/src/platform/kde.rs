//! KDE Plasma: a wallpaper plugin plays the video, Kinora only tells plasmashell which one.
//! The plugin lives in `kde-plugin/` and is copied into the user's plugin folder from the copy
//! built into this binary. Settings reach it through plasmashell's scripting interface.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

use serde_json::Value;
use tauri::WebviewWindow;

use super::{
    AudioMeter, Battery, MediaSource, Platform, PlayerState, Rect, RemotePlayer, linux_common,
};
use crate::state::Fit;

const PLUGIN: &str = "com.kinora.wallpaper";

/// (path inside the plugin folder, contents)
const PLUGIN_FILES: [(&str, &str); 4] = [
    (
        "metadata.json",
        include_str!("../../../kde-plugin/com.kinora.wallpaper/metadata.json"),
    ),
    (
        "contents/config/main.xml",
        include_str!("../../../kde-plugin/com.kinora.wallpaper/contents/config/main.xml"),
    ),
    (
        "contents/ui/main.qml",
        include_str!("../../../kde-plugin/com.kinora.wallpaper/contents/ui/main.qml"),
    ),
    (
        "contents/ui/config.qml",
        include_str!("../../../kde-plugin/com.kinora.wallpaper/contents/ui/config.qml"),
    ),
];

#[derive(Default)]
pub struct KdePlatform {
    /// What plasmashell was last told, so unchanged states cost nothing. None while the
    /// desktop has its own wallpaper.
    applied: Mutex<Option<PlayerState>>,
}

impl Platform for KdePlatform {
    fn name(&self) -> &'static str {
        "kde"
    }

    fn prepare(&self) -> Result<(), String> {
        Ok(())
    }

    fn attach(&self, _win: &WebviewWindow, _bounds: Rect) -> Result<(), String> {
        Err("Plasma plays the wallpaper itself; there is no window to attach.".into())
    }

    fn host_alive(&self) -> bool {
        true
    }

    fn detach(&self) {
        let mut applied = self.applied.lock().unwrap();
        if let Err(e) = restore(&mut applied) {
            eprintln!("kinora: couldn't restore the desktop wallpaper: {e}");
        }
    }

    fn remote_player(&self) -> Option<&dyn RemotePlayer> {
        Some(self)
    }

    fn static_wallpaper(&self) -> Option<String> {
        None
    }

    fn refresh_static_wallpaper(&self) {}

    fn fullscreen_app_open(&self) -> bool {
        false // the plugin watches for fullscreen windows itself
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

impl RemotePlayer for KdePlatform {
    fn apply(&self, state: &PlayerState) -> Result<(), String> {
        let mut applied = self.applied.lock().unwrap();
        let Some(video) = &state.video else {
            return restore(&mut applied);
        };
        if applied.as_ref() == Some(state) {
            return Ok(());
        }
        install_plugin()?;

        let fit = match state.fit {
            Fit::Cover => 0,
            Fit::Contain => 1,
            Fit::Fill => 2,
        };
        // A desktop switches plugin only the first time, so later updates don't reload it.
        let script = format!(
            r#"var ID = "{PLUGIN}", previous = {{}};
desktops().forEach(function (d) {{
    if (d.wallpaperPlugin != ID) {{
        previous[d.id] = d.wallpaperPlugin;
        d.wallpaperPlugin = ID;
    }}
    d.currentConfigGroup = ["Wallpaper", ID, "General"];
    d.writeConfig("Video", {video});
    d.writeConfig("Fit", {fit});
    d.writeConfig("Muted", {muted});
    d.writeConfig("Paused", {paused});
    d.writeConfig("PauseOnFullscreen", {pause_on_fullscreen});
}});
print(JSON.stringify(previous));"#,
            video = js_string(video),
            muted = state.muted,
            paused = state.paused,
            pause_on_fullscreen = state.pause_on_fullscreen,
        );
        let switched: BTreeMap<String, String> =
            serde_json::from_str(&run_script(&script)?).map_err(|e| e.to_string())?;
        if !switched.is_empty() {
            let mut previous = load_previous();
            previous.extend(switched);
            save_previous(&previous)?;
        }
        *applied = Some(state.clone());
        Ok(())
    }
}

/// Give every desktop Kinora took over its old wallpaper back.
fn restore(applied: &mut Option<PlayerState>) -> Result<(), String> {
    let previous = load_previous();
    if applied.is_none() && previous.is_empty() {
        return Ok(()); // never took anything over
    }
    let script = format!(
        r#"var ID = "{PLUGIN}", previous = {};
desktops().forEach(function (d) {{
    if (d.wallpaperPlugin == ID) {{
        d.wallpaperPlugin = previous[d.id] || "org.kde.image";
    }}
}});"#,
        serde_json::to_string(&previous).map_err(|e| e.to_string())?,
    );
    run_script(&script)?;
    *applied = None;
    let _ = fs::remove_file(previous_path());
    Ok(())
}

/// Run a script inside plasmashell and return what it printed.
fn run_script(script: &str) -> Result<String, String> {
    let out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "call",
            "org.kde.plasmashell",
            "/PlasmaShell",
        ])
        .args(["org.kde.PlasmaShell", "evaluateScript", "s", script])
        .output()
        .map_err(|e| format!("couldn't run busctl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "plasmashell didn't answer: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let reply: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    Ok(reply["data"][0].as_str().unwrap_or_default().to_string())
}

/// A JavaScript string literal (JSON strings are valid JavaScript).
fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

fn dir_from(env: &str, fallback: &str) -> PathBuf {
    std::env::var_os(env)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(fallback))
}

/// Copy the plugin into the user's plugin folder when it's missing or out of date.
fn install_plugin() -> Result<(), String> {
    let root = dir_from("XDG_DATA_HOME", ".local/share")
        .join("plasma/wallpapers")
        .join(PLUGIN);
    for (path, contents) in PLUGIN_FILES {
        let target = root.join(path);
        if fs::read_to_string(&target).is_ok_and(|current| current == contents) {
            continue;
        }
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("couldn't create {}: {e}", dir.display()))?;
        }
        fs::write(&target, contents)
            .map_err(|e| format!("couldn't write {}: {e}", target.display()))?;
    }
    Ok(())
}

/// Which wallpaper plugin each desktop had before Kinora took it over (desktop id -> plugin).
/// Kept on disk so a crash doesn't lose it.
fn previous_path() -> PathBuf {
    dir_from("XDG_STATE_HOME", ".local/state").join("kinora/kde-previous.json")
}

fn load_previous() -> BTreeMap<String, String> {
    fs::read_to_string(previous_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_previous(previous: &BTreeMap<String, String>) -> Result<(), String> {
    let path = previous_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string(previous).map_err(|e| e.to_string())?;
    fs::write(Path::new(&path), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each desktop's current wallpaper plugin.
    fn plugins() -> Vec<String> {
        let script = "var o = []; desktops().forEach(function (d) { o.push(d.wallpaperPlugin); }); print(JSON.stringify(o));";
        serde_json::from_str(&run_script(script).unwrap()).unwrap()
    }

    /// Needs a running Plasma session and switches every desktop's wallpaper for a few seconds:
    /// `KINORA_TEST_VIDEO=/path/to/video.mp4 cargo test kde_roundtrip -- --ignored`
    #[test]
    #[ignore = "switches the wallpaper of the running Plasma session"]
    fn kde_roundtrip() {
        let before = plugins();
        let kde = KdePlatform::default();
        let state = PlayerState {
            video: Some(std::env::var("KINORA_TEST_VIDEO").expect("set KINORA_TEST_VIDEO")),
            fit: Fit::Cover,
            muted: true,
            paused: false,
            pause_on_fullscreen: true,
        };

        kde.apply(&state).unwrap();
        assert!(
            plugins().iter().all(|p| p == PLUGIN),
            "every desktop should use the Kinora plugin"
        );
        kde.apply(&state).unwrap(); // unchanged state is skipped

        std::thread::sleep(std::time::Duration::from_secs(3));
        kde.detach();
        assert_eq!(
            plugins(),
            before,
            "detach should put every desktop's wallpaper back"
        );
    }
}
