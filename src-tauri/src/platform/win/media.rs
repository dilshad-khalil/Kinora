//! Windows' "now playing", the same source as the volume flyout.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use ::windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager as SessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Storage::Streams::DataReader,
};

use crate::platform::{MediaInfo, MediaSource};

#[derive(Default)]
pub struct Media {
    manager: Option<SessionManager>,
    art_track: String,
    art: Option<String>,
    art_tries: u32,
}

impl MediaSource for Media {
    fn current(&mut self) -> Option<MediaInfo> {
        self.read().ok().flatten()
    }
}

impl Media {
    fn read(&mut self) -> ::windows::core::Result<Option<MediaInfo>> {
        if self.manager.is_none() {
            self.manager = Some(SessionManager::RequestAsync()?.join()?);
        }
        let manager = self.manager.as_ref().unwrap();
        let Ok(session) = manager.GetCurrentSession() else {
            return Ok(None);
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

        Ok(Some(MediaInfo {
            title,
            artist,
            album,
            app: session.SourceAppUserModelId()?.to_string(),
            status: status_name(status).into(),
            playing,
            position,
            duration,
            artwork: self.art.clone(),
        }))
    }
}

/// Read a thumbnail stream into a data URL (at most 2 MB).
fn read_image(r: &::windows::Storage::Streams::IRandomAccessStreamReference) -> ::windows::core::Result<String> {
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
