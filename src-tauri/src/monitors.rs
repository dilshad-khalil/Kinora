use serde::Serialize;
use tauri::{AppHandle, Monitor};

use crate::platform::Rect;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub name: String,
    /// Physical pixels, virtual-screen coordinates.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

impl MonitorInfo {
    pub fn rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, w: self.width as i32, h: self.height as i32 }
    }
}

pub fn list(app: &AppHandle) -> Vec<MonitorInfo> {
    let primary = app.primary_monitor().ok().flatten();
    let is_primary = |m: &Monitor| {
        primary.as_ref().is_some_and(|p| p.position() == m.position())
    };
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| MonitorInfo {
            name: m.name().cloned().unwrap_or_default(),
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
            scale: m.scale_factor(),
            primary: is_primary(m),
        })
        .collect()
}
