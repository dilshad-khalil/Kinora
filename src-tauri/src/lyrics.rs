//! Lyrics for the playing track, from lrclib.net (free, no key). Only used when a placed widget
//! has the "lyrics" permission, because it sends the track's title and artist to that service.
//! Each lookup runs on its own thread; results are cached for the session.

use std::{collections::HashMap, sync::Mutex, thread, time::Duration};

use serde_json::{json, Value};
use tauri::AppHandle;
use crate::providers;

const USER_AGENT: &str = "Kinora (https://github.com/dilshad-khalil/Kinora)";

/// Track key -> lyrics payload.
static CACHE: Mutex<Option<HashMap<String, Value>>> = Mutex::new(None);
/// The track lyrics should be shown for right now.
static CURRENT: Mutex<String> = Mutex::new(String::new());

/// Called whenever the media topic changes. Publishes "loading" and starts a lookup when the
/// track is new.
pub fn track_changed(app: &AppHandle, media: &Value) {
    let title = media["title"].as_str().unwrap_or("").trim().to_string();
    let artist = media["artist"].as_str().unwrap_or("").trim().to_string();
    if title.is_empty() {
        *CURRENT.lock().unwrap() = String::new();
        providers::publish(app, providers::LYRICS, Value::Null);
        return;
    }
    let album = media["album"].as_str().unwrap_or("").trim().to_string();
    let duration = media["duration"].as_f64().unwrap_or(0.0);
    let key = format!("{title}\u{1}{artist}");
    {
        let mut current = CURRENT.lock().unwrap();
        if *current == key {
            return; // same track (play/pause/seek)
        }
        *current = key.clone();
    }
    if let Some(hit) = CACHE.lock().unwrap().as_ref().and_then(|c| c.get(&key).cloned()) {
        providers::publish(app, providers::LYRICS, hit);
        return;
    }

    providers::publish(app, providers::LYRICS, payload(&title, &artist, "loading", None));
    let app = app.clone();
    thread::spawn(move || {
        let result = lookup(&title, &artist, &album, duration);
        let value = match result {
            Some(found) => payload(&title, &artist, "found", Some(found)),
            None => payload(&title, &artist, "none", None),
        };
        CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert(key.clone(), value.clone());
        // The user may have skipped to another song meanwhile.
        if *CURRENT.lock().unwrap() == key {
            providers::publish(&app, providers::LYRICS, value);
        }
    });
}

fn payload(title: &str, artist: &str, status: &str, found: Option<Value>) -> Value {
    let found = found.unwrap_or(Value::Null);
    json!({
        "title": title,
        "artist": artist,
        "status": status, // "loading" | "found" | "none"
        "instrumental": found["instrumentalFlag"].as_bool().unwrap_or(false),
        "synced": found.get("synced").cloned().unwrap_or(Value::Null),
        "plain": found.get("plain").cloned().unwrap_or(Value::Null),
        "source": "lrclib.net",
    })
}

/// Exact match first, then a search with a cleaned-up title ("Song - Remastered 2011" -> "Song").
fn lookup(title: &str, artist: &str, album: &str, duration: f64) -> Option<Value> {
    let mut url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        encode(title),
        encode(artist)
    );
    if !album.is_empty() {
        url += &format!("&album_name={}", encode(album));
    }
    if duration > 0.0 {
        url += &format!("&duration={}", duration.round());
    }
    if let Some(v) = get_json(&url).filter(Value::is_object) {
        return Some(parse(&v));
    }

    let clean = clean_title(title);
    let url = format!(
        "https://lrclib.net/api/search?track_name={}&artist_name={}",
        encode(&clean),
        encode(artist)
    );
    let results = get_json(&url)?;
    let list = results.as_array()?;
    // Prefer synced lyrics with a close duration.
    let close = |v: &&Value| duration <= 0.0 || (v["duration"].as_f64().unwrap_or(0.0) - duration).abs() < 4.0;
    let best = list
        .iter()
        .filter(close)
        .find(|v| v["syncedLyrics"].is_string())
        .or_else(|| list.iter().filter(close).find(|v| v["plainLyrics"].is_string()))
        .or_else(|| list.iter().find(|v| v["syncedLyrics"].is_string()))?;
    Some(parse(best))
}

fn parse(v: &Value) -> Value {
    json!({
        "instrumentalFlag": v["instrumental"].as_bool().unwrap_or(false),
        "synced": v["syncedLyrics"].as_str().map(parse_lrc),
        "plain": v["plainLyrics"].as_str(),
    })
}

/// "[01:02.34] line" -> [{ "time": 62.34, "text": "line" }], sorted. A line can carry several
/// timestamps ("[00:10.00][01:10.00] chorus").
fn parse_lrc(lrc: &str) -> Value {
    let mut lines: Vec<(f64, String)> = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut times = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            let stamp = &stripped[..end];
            if let Some((m, s)) = stamp.split_once(':') {
                if let (Ok(m), Ok(s)) = (m.parse::<f64>(), s.parse::<f64>()) {
                    times.push(m * 60.0 + s);
                }
            }
            rest = stripped[end + 1..].trim_start();
        }
        for t in times {
            lines.push((t, rest.to_string()));
        }
    }
    lines.sort_by(|a, b| a.0.total_cmp(&b.0));
    Value::Array(
        lines
            .into_iter()
            .map(|(t, text)| json!({ "time": (t * 100.0).round() / 100.0, "text": text }))
            .collect(),
    )
}

fn clean_title(title: &str) -> String {
    let mut t = title.to_string();
    for sep in [" - ", " (feat", " (with", " [", " (Remaster", " (Live"] {
        if let Some(i) = t.find(sep) {
            t.truncate(i);
        }
    }
    t.trim().to_string()
}

/// GET a JSON document. One retry, since lrclib sometimes answers 503 when busy. None on 404
/// or any failure.
fn get_json(url: &str) -> Option<Value> {
    for attempt in 0..2 {
        if attempt > 0 {
            thread::sleep(Duration::from_secs(3));
        }
        match get(url) {
            Ok((200, body)) => return serde_json::from_str(&body).ok(),
            Ok((404, _)) => return None,
            Ok(_) | Err(_) => continue,
        }
    }
    None
}

fn get(url: &str) -> Result<(u16, String), ureq::Error> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent(USER_AGENT)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut response = agent.get(url).call()?;
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string()?;
    Ok((status, body))
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
