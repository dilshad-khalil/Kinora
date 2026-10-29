//! Optional wallpaper rotation: every few minutes, switch to the next video in the rotation list.
//! The list and interval are saved by the control panel; this thread only reads them.

use std::{
    path::Path,
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tauri::AppHandle;

use crate::{
    state::{self, Rotation},
    wallpaper,
};

static LAST_CHANGE: Mutex<Option<Instant>> = Mutex::new(None);

/// Restart the countdown (the wallpaper just changed, or the settings did).
pub fn mark_changed() {
    *LAST_CHANGE.lock().unwrap() = Some(Instant::now());
}

pub fn spawn(app: AppHandle) {
    mark_changed();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(5));
        tick(&app);
    });
}

/// Rotation list entries that still exist on disk.
pub fn usable(app: &AppHandle) -> Vec<String> {
    state::rotation(app).paths.into_iter().filter(|p| Path::new(p).is_file()).collect()
}

fn tick(app: &AppHandle) {
    let config = state::rotation(app);
    if !config.enabled || wallpaper::is_building() {
        return;
    }
    let s = state::snapshot(app);
    if s.current.is_none() || s.paused || s.auto_paused {
        return;
    }
    let due = LAST_CHANGE
        .lock()
        .unwrap()
        .is_none_or(|t| t.elapsed() >= Duration::from_secs(config.interval_min.max(1) * 60));
    if due {
        next(app);
    }
}

/// Switch to the next video in the rotation list, if there are at least two to pick from.
pub fn next(app: &AppHandle) {
    let Rotation { shuffle, .. } = state::rotation(app);
    let list = usable(app);
    let current = state::snapshot(app).current;
    let Some(path) = pick(&list, current.as_deref(), shuffle, random()) else {
        mark_changed(); // nothing to rotate to; check again after a full interval
        return;
    };
    if let Err(e) = wallpaper::set_wallpaper(app, path.to_string()) {
        eprintln!("kinora: rotation couldn't set the wallpaper: {e}");
        mark_changed();
    }
}

/// The video to show after `current`. `None` when the list has fewer than two entries.
fn pick<'a>(list: &'a [String], current: Option<&str>, shuffle: bool, random: usize) -> Option<&'a String> {
    if list.len() < 2 {
        return None;
    }
    let position = current.and_then(|c| list.iter().position(|p| p == c));
    if shuffle {
        let others: Vec<_> = list.iter().filter(|p| Some(p.as_str()) != current).collect();
        return others.get(random % others.len()).copied();
    }
    Some(&list[position.map_or(0, |i| (i + 1) % list.len())])
}

/// Good enough for shuffling a wallpaper list.
fn random() -> usize {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
    let mut x = nanos as u64 | 1;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x as usize
}

#[cfg(test)]
mod tests {
    use super::pick;

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn needs_two_videos() {
        assert_eq!(pick(&list(&[]), None, false, 0), None);
        assert_eq!(pick(&list(&["a"]), Some("a"), false, 0), None);
        assert_eq!(pick(&list(&["a"]), Some("a"), true, 0), None);
    }

    #[test]
    fn sequential_wraps() {
        let l = list(&["a", "b", "c"]);
        assert_eq!(pick(&l, Some("a"), false, 0).map(String::as_str), Some("b"));
        assert_eq!(pick(&l, Some("c"), false, 0).map(String::as_str), Some("a"));
    }

    #[test]
    fn starts_at_first_when_current_is_not_listed() {
        let l = list(&["a", "b"]);
        assert_eq!(pick(&l, Some("x"), false, 0).map(String::as_str), Some("a"));
        assert_eq!(pick(&l, None, false, 0).map(String::as_str), Some("a"));
    }

    #[test]
    fn shuffle_never_repeats_current() {
        let l = list(&["a", "b", "c"]);
        for r in 0..10 {
            assert_ne!(pick(&l, Some("b"), true, r).map(String::as_str), Some("b"));
        }
    }
}
