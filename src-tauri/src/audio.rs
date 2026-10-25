//! Loudness of whatever is playing, from the platform's output meter. No audio data is
//! captured, only levels. Sampled about 30 times a second, and only while a placed, visible
//! widget has the "audio" permission.

use std::{
    thread,
    time::{Duration, Instant},
};

use serde_json::json;
use tauri::AppHandle;

use crate::{platform::AudioMeter, providers, state};

const FRAME: Duration = Duration::from_millis(33);

pub fn spawn(app: AppHandle) {
    let platform = state::platform(&app);
    thread::spawn(move || {
        let mut meter: Option<(Box<dyn AudioMeter>, Instant)> = None;
        let mut silent_frames = 0u32;
        loop {
            if !providers::wanted(&app, providers::AUDIO) {
                meter = None;
                thread::sleep(Duration::from_millis(500));
                continue;
            }
            // Re-open now and then, so a change of default device (headphones) is picked up.
            if meter.as_ref().is_none_or(|(_, at)| at.elapsed() > Duration::from_secs(5)) {
                meter = platform.audio_meter().map(|m| (m, Instant::now()));
            }
            let levels = meter.as_mut().and_then(|(m, _)| m.levels());
            let Some((peak, channels)) = levels else {
                meter = None;
                thread::sleep(Duration::from_millis(500));
                continue;
            };

            // During silence, send a single zero frame and then stop sending.
            silent_frames = if peak < 0.0005 { silent_frames + 1 } else { 0 };
            if silent_frames <= 2 {
                let left = channels.first().copied().unwrap_or(peak);
                let right = channels.get(1).copied().unwrap_or(left);
                providers::publish(
                    &app,
                    providers::AUDIO,
                    json!({ "peak": round3(peak), "left": round3(left), "right": round3(right) }),
                );
            }
            thread::sleep(FRAME);
        }
    });
}

fn round3(v: f32) -> f64 {
    (v as f64 * 1000.0).round() / 1000.0
}
