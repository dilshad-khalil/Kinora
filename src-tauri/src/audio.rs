//! Loudness of whatever is playing, from the default output device's peak meter (the same meter
//! as the volume mixer). No audio data is captured, only levels. Sampled about 30 times a
//! second, and only while a placed, visible widget has the "audio" permission.

use std::{
    thread,
    time::{Duration, Instant},
};

use serde_json::json;
use tauri::AppHandle;
use windows::Win32::{
    Media::Audio::{eConsole, eRender, Endpoints::IAudioMeterInformation, IMMDeviceEnumerator, MMDeviceEnumerator},
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
};

use crate::providers;

const FRAME: Duration = Duration::from_millis(33);

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut meter: Option<(IAudioMeterInformation, Instant)> = None;
        let mut silent_frames = 0u32;
        loop {
            if !providers::wanted(&app, providers::AUDIO) {
                meter = None;
                thread::sleep(Duration::from_millis(500));
                continue;
            }
            // Re-open now and then, so a change of default device (headphones) is picked up.
            if meter.as_ref().is_none_or(|(_, at)| at.elapsed() > Duration::from_secs(5)) {
                meter = open().ok().map(|m| (m, Instant::now()));
            }
            let levels = meter.as_ref().and_then(|(m, _)| read(m).ok());
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

fn open() -> windows::core::Result<IAudioMeterInformation> {
    unsafe {
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eConsole)?;
        device.Activate(CLSCTX_ALL, None)
    }
}

fn read(meter: &IAudioMeterInformation) -> windows::core::Result<(f32, Vec<f32>)> {
    unsafe {
        let peak = meter.GetPeakValue()?;
        let count = meter.GetMeteringChannelCount()?.min(8) as usize;
        let mut channels = vec![0f32; count];
        if count > 0 {
            meter.GetChannelsPeakValues(&mut channels)?;
        }
        Ok((peak, channels))
    }
}

fn round3(v: f32) -> f64 {
    (v as f64 * 1000.0).round() / 1000.0
}
