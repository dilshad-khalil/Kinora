//! Live data for widgets: system stats and the media that's playing. One thread samples a
//! topic only while a placed widget has declared the matching permission, and stops while the
//! widgets are hidden. Data goes out as `widgets:data` events; the pages hosting the widgets
//! relay it into the iframes that are allowed to see it.

use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use serde_json::{json, Value};
use sysinfo::{Disks, Networks, System};
use tauri::{AppHandle, Emitter, Manager};
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager as SessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Storage::Streams::DataReader,
    Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
};

use crate::state::{self, AppState};

pub const SYSTEM: &str = "system";
pub const MEDIA: &str = "media";
pub const LYRICS: &str = "lyrics";
pub const AUDIO: &str = "audio";
pub const NOTIFICATIONS: &str = "notifications";
pub const PERMISSIONS: &[&str] = &[SYSTEM, MEDIA, LYRICS, AUDIO, NOTIFICATIONS];

/// Topics some placed widget is allowed to read.
static NEEDED: Mutex<Option<HashSet<String>>> = Mutex::new(None);
/// The last value sent per topic, for pages that open later.
static LATEST: Mutex<Option<HashMap<String, Value>>> = Mutex::new(None);

pub fn set_needed(topics: HashSet<String>) {
    *NEEDED.lock().unwrap() = Some(topics);
}

pub fn latest(topic: &str) -> Option<Value> {
    LATEST.lock().unwrap().as_ref()?.get(topic).cloned()
}

pub fn publish(app: &AppHandle, topic: &str, data: Value) {
    LATEST.lock().unwrap().get_or_insert_with(HashMap::new).insert(topic.into(), data.clone());
    let _ = app.emit("widgets:data", json!({ "topic": topic, "data": data }));
}

/// Whether the widgets are on screen.
fn shown(app: &AppHandle) -> bool {
    app.state::<AppState>().runtime.lock().unwrap().host.is_some() && !state::snapshot(app).auto_paused
}

/// A placed widget may read `topic` and the widgets are on screen.
pub fn wanted(app: &AppHandle, topic: &str) -> bool {
    NEEDED.lock().unwrap().as_ref().is_some_and(|n| n.contains(topic)) && shown(app)
}

pub fn spawn(app: AppHandle) {
    crate::audio::spawn(app.clone());
    thread::spawn(move || {
        let mut system = SystemSampler::new();
        let mut media = MediaSampler::default();
        loop {
            thread::sleep(Duration::from_secs(1));
            // Only while the widgets are on screen.
            let needed = NEEDED.lock().unwrap().clone().unwrap_or_default();
            if !shown(&app) {
                continue;
            }
            if needed.contains(SYSTEM) {
                publish(&app, SYSTEM, system.sample());
            }
            // Lyrics follow the media topic, so sample media for either.
            if needed.contains(MEDIA) || needed.contains(LYRICS) {
                if let Some(data) = media.sample() {
                    publish(&app, MEDIA, data);
                }
                if needed.contains(LYRICS) {
                    let now = latest(MEDIA).unwrap_or(Value::Null);
                    crate::lyrics::track_changed(&app, &now);
                }
            }
        }
    });
}

// ---- System ----

struct SystemSampler {
    sys: System,
    networks: Networks,
    disks: Disks,
    last_net: Instant,
    ticks: u32,
}

impl SystemSampler {
    fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        Self {
            sys,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last_net: Instant::now(),
            ticks: 0,
        }
    }

    fn sample(&mut self) -> Value {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.networks.refresh(true);
        // Disks change slowly.
        if self.ticks % 30 == 0 {
            self.disks.refresh(true);
        }
        self.ticks = self.ticks.wrapping_add(1);

        let secs = self.last_net.elapsed().as_secs_f64().max(0.001);
        self.last_net = Instant::now();
        let (down, up) = self
            .networks
            .values()
            .fold((0, 0), |(d, u), n| (d + n.received(), u + n.transmitted()));

        let (used, total) = (self.sys.used_memory(), self.sys.total_memory());
        let disks: Vec<Value> = self
            .disks
            .list()
            .iter()
            .filter(|d| !d.is_removable() && d.total_space() > 0)
            .map(|d| {
                let total = d.total_space();
                let used = total - d.available_space();
                json!({
                    "mount": d.mount_point().to_string_lossy(),
                    "used": used,
                    "total": total,
                    "usage": percent(used, total),
                })
            })
            .collect();
        let cpus = self.sys.cpus();

        json!({
            "cpu": {
                "usage": round1(self.sys.global_cpu_usage() as f64),
                "cores": cpus.len(),
                "name": cpus.first().map(|c| c.brand().trim()).unwrap_or(""),
            },
            "memory": { "used": used, "total": total, "usage": percent(used, total) },
            "disks": disks,
            "network": { "down": (down as f64 / secs).round(), "up": (up as f64 / secs).round() },
            "battery": battery(),
            "uptime": System::uptime(),
        })
    }
}

fn battery() -> Value {
    let mut s = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut s) }.is_err() || s.BatteryFlag & 128 != 0 || s.BatteryFlag == 255 {
        return Value::Null; // no battery (desktop) or unknown
    }
    json!({
        "percent": (s.BatteryLifePercent != 255).then_some(s.BatteryLifePercent),
        "charging": s.BatteryFlag & 8 != 0,
        "pluggedIn": s.ACLineStatus == 1,
    })
}

fn percent(part: u64, total: u64) -> f64 {
    if total == 0 { 0.0 } else { round1(part as f64 * 100.0 / total as f64) }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// ---- Media (Windows' "now playing", the same source as the volume flyout) ----

#[derive(Default)]
struct MediaSampler {
    manager: Option<SessionManager>,
    /// Identity of what was last sent, minus the position.
    sent: String,
    /// Position we last sent, when, and whether it was moving.
    sent_position: Option<(f64, Instant, bool)>,
    art_track: String,
    art: Option<String>,
    art_tries: u32,
}

impl MediaSampler {
    /// Some(data) when something changed; data is `null` when nothing is playing.
    fn sample(&mut self) -> Option<Value> {
        let now = self.read().unwrap_or(Value::Null);
        let key = json!([now["title"], now["artist"], now["album"], now["status"], now["duration"], now["app"], now["artwork"].is_string()])
            .to_string();
        let position = now["position"].as_f64();
        let playing = now["playing"].as_bool().unwrap_or(false);

        // A seek or a lagging timeline also counts as a change.
        let drifted = match (position, self.sent_position) {
            (Some(p), Some((last, at, moving))) => {
                let expected = last + if moving { at.elapsed().as_secs_f64() } else { 0.0 };
                (p - expected).abs() > 2.0
            }
            _ => false,
        };
        if key == self.sent && !drifted {
            return None;
        }
        self.sent = key;
        self.sent_position = position.map(|p| (p, Instant::now(), playing));
        Some(now)
    }

    fn read(&mut self) -> windows::core::Result<Value> {
        if self.manager.is_none() {
            self.manager = Some(SessionManager::RequestAsync()?.join()?);
        }
        let manager = self.manager.as_ref().unwrap();
        let Ok(session) = manager.GetCurrentSession() else {
            return Ok(Value::Null);
        };
        let props = session.TryGetMediaPropertiesAsync()?.join()?;
        let title = props.Title()?.to_string();
        let artist = props.Artist()?.to_string();
        let album = props.AlbumTitle()?.to_string();
        let status = session.GetPlaybackInfo()?.PlaybackStatus()?;
        let playing = status == Status::Playing;

        let timeline = session.GetTimelineProperties()?;
        let duration = ticks_to_secs(timeline.EndTime()?.Duration - timeline.StartTime()?.Duration);
        let mut position = ticks_to_secs(timeline.Position()?.Duration);
        if playing {
            // The timeline is a snapshot from LastUpdatedTime; move it forward to now.
            let updated_ms = (timeline.LastUpdatedTime()?.UniversalTime - 116_444_736_000_000_000) / 10_000;
            let since = (unix_ms() - updated_ms).max(0) as f64 / 1000.0;
            position += since;
        }
        if duration > 0.0 {
            position = position.min(duration);
        }

        // Artwork only when the track changes (apps sometimes add it a moment later).
        let track = format!("{title}\u{1}{artist}\u{1}{album}");
        if track != self.art_track {
            self.art_track = track;
            self.art = None;
            self.art_tries = 0;
        }
        if self.art.is_none() && self.art_tries < 5 {
            self.art_tries += 1;
            self.art = props.Thumbnail().ok().and_then(|t| read_image(&t).ok());
        }

        Ok(json!({
            "title": title,
            "artist": artist,
            "album": album,
            "app": session.SourceAppUserModelId()?.to_string(),
            "status": status_name(status),
            "playing": playing,
            "position": round1(position),
            "duration": round1(duration),
            "updatedAt": unix_ms(),
            "artwork": self.art,
        }))
    }
}

/// Read a thumbnail stream into a data URL (at most 2 MB).
fn read_image(r: &windows::Storage::Streams::IRandomAccessStreamReference) -> windows::core::Result<String> {
    let stream = r.OpenReadAsync()?.join()?;
    let size = stream.Size()?.min(2 * 1024 * 1024) as u32;
    let mime = stream.ContentType().map(|m| m.to_string()).unwrap_or_default();
    let reader = DataReader::CreateDataReader(&stream)?;
    reader.LoadAsync(size)?.join()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes)?;
    let mime = if mime.starts_with("image/") { mime } else { "image/jpeg".into() };
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Playing => "playing",
        Status::Paused => "paused",
        Status::Stopped => "stopped",
        Status::Changing => "changing",
        _ => "closed",
    }
}

fn ticks_to_secs(ticks: i64) -> f64 {
    ticks.max(0) as f64 / 10_000_000.0
}

fn unix_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
