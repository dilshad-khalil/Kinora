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

use serde_json::{json, Value};
use sysinfo::{Disks, Networks, System};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    platform::{MediaInfo, MediaSource, Platform},
    state::{self, AppState},
};

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
    app.state::<AppState>().runtime.lock().unwrap().attached && !state::snapshot(app).auto_paused
}

/// A placed widget may read `topic` and the widgets are on screen.
pub fn wanted(app: &AppHandle, topic: &str) -> bool {
    NEEDED.lock().unwrap().as_ref().is_some_and(|n| n.contains(topic)) && shown(app)
}

pub fn spawn(app: AppHandle) {
    crate::audio::spawn(app.clone());
    let platform = state::platform(&app);
    thread::spawn(move || {
        let mut system = SystemSampler::new();
        let mut media = MediaSampler::new(platform.media_source());
        loop {
            thread::sleep(Duration::from_secs(1));
            // Only while the widgets are on screen.
            let needed = NEEDED.lock().unwrap().clone().unwrap_or_default();
            if !shown(&app) {
                continue;
            }
            if needed.contains(SYSTEM) {
                publish(&app, SYSTEM, system.sample(platform.as_ref()));
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

    fn sample(&mut self, platform: &dyn Platform) -> Value {
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
            "battery": battery(platform),
            "uptime": System::uptime(),
        })
    }
}

fn battery(platform: &dyn Platform) -> Value {
    let Some(b) = platform.battery() else {
        return Value::Null; // no battery (desktop) or unknown
    };
    json!({ "percent": b.percent, "charging": b.charging, "pluggedIn": b.plugged_in })
}

fn percent(part: u64, total: u64) -> f64 {
    if total == 0 { 0.0 } else { round1(part as f64 * 100.0 / total as f64) }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// ---- Media (what the system says is playing) ----

struct MediaSampler {
    source: Option<Box<dyn MediaSource>>,
    /// Identity of what was last sent, minus the position.
    sent: String,
    /// Position we last sent, when, and whether it was moving.
    sent_position: Option<(f64, Instant, bool)>,
}

impl MediaSampler {
    fn new(source: Option<Box<dyn MediaSource>>) -> Self {
        Self { source, sent: String::new(), sent_position: None }
    }

    /// Some(data) when something changed; data is `null` when nothing is playing.
    fn sample(&mut self) -> Option<Value> {
        let now = self.source.as_mut().and_then(|s| s.current()).map(to_json).unwrap_or(Value::Null);
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
}

fn to_json(m: MediaInfo) -> Value {
    json!({
        "title": m.title,
        "artist": m.artist,
        "album": m.album,
        "app": m.app,
        "status": m.status,
        "playing": m.playing,
        "position": round1(m.position),
        "duration": round1(m.duration),
        "updatedAt": unix_ms(),
        "artwork": m.artwork,
    })
}

fn unix_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
