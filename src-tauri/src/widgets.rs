//! HTML widgets. Each widget is a folder in `<app data>/widgets/` with an `index.html` and an
//! optional `widget.json`. They run as sandboxed iframes inside the wallpaper window, served by
//! the `kwidget` protocol below, which only hands out files from the widget's own folder and
//! attaches a CSP that blocks the network, the asset protocol and Tauri's IPC.

use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::{
    http::{Request, Response, StatusCode},
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

use crate::{monitors, state::STORE};

pub const SCHEME: &str = "kwidget";
const ORIGIN: &str = "http://kwidget.localhost";
const EDITOR: &str = "widget-editor";
const SDK: &str = include_str!("../widgets/kinora.js");
const MAX_FILE: u64 = 10 * 1024 * 1024;
/// Bumped when widget files may have changed, so open pages reload their iframes.
static REVISION: AtomicU32 = AtomicU32::new(0);

/// Built-in widgets, written to the widgets folder when they're missing.
const BUILT_IN: &[(&str, &[u8])] = &[
    ("README.md", include_bytes!("../widgets/README.md")),
    ("AI-GUIDE.md", include_bytes!("../widgets/AI-GUIDE.md")),
    ("clock/widget.json", include_bytes!("../widgets/clock/widget.json")),
    ("clock/index.html", include_bytes!("../widgets/clock/index.html")),
    ("quote/widget.json", include_bytes!("../widgets/quote/widget.json")),
    ("quote/index.html", include_bytes!("../widgets/quote/index.html")),
    ("quote/gilda-display.woff2", include_bytes!("../widgets/quote/gilda-display.woff2")),
    ("quote/OFL.txt", include_bytes!("../widgets/quote/OFL.txt")),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Size {
    pub width: f64,
    pub height: f64,
    pub min_width: f64,
    pub min_height: f64,
    pub resizable: bool,
}

impl Default for Size {
    fn default() -> Self {
        Self { width: 320.0, height: 180.0, min_width: 80.0, min_height: 60.0, resizable: true }
    }
}

/// `widget.json`. Everything is optional, so a bare `index.html` is a valid widget.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub entry: String,
    pub size: Size,
    /// Blur whatever is behind the widget, with rounded corners of `radius` px.
    pub glass: bool,
    pub radius: f64,
    /// Settings shown in the control panel, passed to the widget as `{ key: value }`.
    pub settings: Vec<Value>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            author: String::new(),
            version: "1.0.0".into(),
            entry: "index.html".into(),
            size: Size::default(),
            glass: false,
            radius: 0.0,
            settings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String, // folder name
    #[serde(flatten)]
    pub manifest: Manifest,
    pub error: Option<String>,
}

/// One widget placed on the desktop. Position and size are CSS pixels, relative to the
/// monitor's top-left corner.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub widget: String,
    pub monitor: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub settings: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetsState {
    pub widgets: Vec<Widget>,
    pub instances: Vec<Instance>,
    pub revision: u32,
}

pub fn dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_default().join("widgets")
}

/// Write the built-in files: anything missing, docs that changed, and whole widgets whose bundled
/// version differs from the installed one. Users customise widgets through settings, or by
/// copying a folder.
pub fn install_built_ins(app: &AppHandle) {
    let root = dir(app);
    let version = |json: &[u8]| {
        serde_json::from_slice::<Manifest>(json).map(|m| m.version).unwrap_or_default()
    };
    let outdated: Vec<&str> = BUILT_IN
        .iter()
        .filter(|(rel, _)| rel.ends_with("/widget.json"))
        .filter(|(rel, bytes)| {
            fs::read(root.join(rel)).is_ok_and(|installed| version(&installed) != version(bytes))
        })
        .filter_map(|(rel, _)| rel.split('/').next())
        .collect();

    for (rel, bytes) in BUILT_IN {
        let path = root.join(rel);
        let update = match rel.split_once('/') {
            Some((id, _)) => outdated.contains(&id),
            // Top-level docs belong to Kinora: keep them current.
            None => fs::read(&path).is_ok_and(|installed| installed != *bytes),
        };
        if path.exists() && !update {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&path, bytes) {
            eprintln!("kinora: couldn't install {rel}: {e}");
        }
    }
}

pub fn list(app: &AppHandle) -> Vec<Widget> {
    let Ok(entries) = fs::read_dir(dir(app)) else { return Vec::new() };
    let mut widgets: Vec<Widget> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| read_widget(&e.path()))
        .collect();
    widgets.sort_by(|a, b| a.manifest.name.to_lowercase().cmp(&b.manifest.name.to_lowercase()));
    widgets
}

fn read_widget(folder: &Path) -> Option<Widget> {
    let id = folder.file_name()?.to_str()?.to_string();
    if !valid_id(&id) {
        return None;
    }
    let (mut manifest, mut error) = match fs::read_to_string(folder.join("widget.json")) {
        Ok(text) => match serde_json::from_str::<Manifest>(&text) {
            Ok(m) => (m, None),
            Err(e) => (Manifest::default(), Some(format!("widget.json: {e}"))),
        },
        Err(_) => (Manifest::default(), None),
    };
    if manifest.name.is_empty() {
        manifest.name = id.clone();
    }
    if error.is_none() && !folder.join(&manifest.entry).is_file() {
        error = Some(format!("{} is missing", manifest.entry));
    }
    Some(Widget { id, manifest, error })
}

/// Folder names become URL segments, so keep them boring.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn instances(app: &AppHandle) -> Vec<Instance> {
    app.store(STORE)
        .ok()
        .and_then(|s| s.get("widgetInstances"))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn has_instances(app: &AppHandle) -> bool {
    !instances(app).is_empty()
}

pub fn snapshot(app: &AppHandle) -> WidgetsState {
    WidgetsState {
        widgets: list(app),
        instances: instances(app),
        revision: REVISION.load(Ordering::Relaxed),
    }
}

/// Save the placed widgets and tell every window.
fn save_instances(app: &AppHandle, instances: Vec<Instance>) -> WidgetsState {
    if let Ok(store) = app.store(STORE) {
        store.set("widgetInstances", json!(instances));
        let _ = store.save();
    }
    let s = WidgetsState { instances, ..snapshot(app) };
    let _ = app.emit("widgets:changed", &s);
    s
}

/// Rescan the folder and make every page reload its widgets.
pub fn notify(app: &AppHandle) -> WidgetsState {
    REVISION.fetch_add(1, Ordering::Relaxed);
    let s = snapshot(app);
    let _ = app.emit("widgets:changed", &s);
    s
}

/// Place a new copy of `widget` near the top-right of the primary monitor.
pub fn add(app: &AppHandle, widget: &str) -> Result<WidgetsState, String> {
    let w = list(app).into_iter().find(|w| w.id == widget).ok_or("Widget not found.")?;
    let monitors = monitors::list(app);
    let m = monitors.iter().find(|m| m.primary).or(monitors.first()).ok_or("No monitors found.")?;
    let mut all = instances(app);
    let cascade = 24.0 * (all.len() % 8) as f64;
    let (width, height) = (w.manifest.size.width, w.manifest.size.height);
    let monitor_width = m.width as f64 / m.scale;
    all.push(Instance {
        id: format!("{:x}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()),
        widget: w.id,
        monitor: m.name.clone(),
        x: (monitor_width - width - 48.0 - cascade).max(0.0),
        y: 48.0 + cascade,
        width,
        height,
        settings: Map::new(),
    });
    Ok(save_instances(app, all))
}

pub fn update(app: &AppHandle, instance: Instance) -> WidgetsState {
    let all = instances(app)
        .into_iter()
        .map(|i| if i.id == instance.id { instance.clone() } else { i })
        .collect();
    save_instances(app, all)
}

pub fn remove(app: &AppHandle, id: &str) -> WidgetsState {
    let all = instances(app).into_iter().filter(|i| i.id != id).collect();
    save_instances(app, all)
}

/// Copy a widget folder, or a single .html file, into the widgets folder.
pub fn import(app: &AppHandle, source: &str) -> Result<WidgetsState, String> {
    let source = Path::new(source);
    let stem = source.file_stem().and_then(|s| s.to_str()).ok_or("Invalid path.")?;
    let base: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .to_lowercase();
    let base = if base.trim_matches('-').is_empty() { "widget".into() } else { base };
    let root = dir(app);
    let mut id = base.clone();
    let mut n = 2;
    while root.join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    let target = root.join(&id);

    if source.is_dir() {
        if !source.join("index.html").is_file() && !source.join("widget.json").is_file() {
            return Err("That folder has no index.html or widget.json.".into());
        }
        copy_dir(source, &target, 0).map_err(|e| e.to_string())?;
    } else if source.extension().is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm")) {
        fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        fs::copy(source, target.join("index.html")).map_err(|e| e.to_string())?;
    } else {
        return Err("Pick a widget folder or an .html file.".into());
    }
    Ok(notify(app))
}

fn copy_dir(from: &Path, to: &Path, depth: u32) -> std::io::Result<()> {
    if depth > 8 {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            copy_dir(&entry.path(), &dest, depth + 1)?;
        } else if kind.is_file() && entry.metadata()?.len() <= MAX_FILE {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

/// Delete an imported widget's folder and any placed copies of it.
pub fn uninstall(app: &AppHandle, widget: &str) -> Result<WidgetsState, String> {
    if !valid_id(widget) {
        return Err("Invalid widget.".into());
    }
    let folder = dir(app).join(widget);
    fs::remove_dir_all(&folder).map_err(|e| e.to_string())?;
    let all = instances(app).into_iter().filter(|i| i.widget != widget).collect();
    Ok(save_instances(app, all))
}

// ---- kwidget:// protocol ----

pub fn serve(app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = percent_decode(request.uri().path());
    let path = path.trim_start_matches('/');

    if path == "kinora.js" {
        return respond(StatusCode::OK, "text/javascript", SDK.as_bytes().to_vec(), None);
    }
    let Some((id, rest)) = path.split_once('/') else {
        return not_found();
    };
    if !valid_id(id) || rest.is_empty() {
        return not_found();
    }
    match resolve(&dir(app).join(id), rest) {
        Some(file) => match fs::read(&file) {
            Ok(bytes) => respond(StatusCode::OK, mime(&file), bytes, Some(id)),
            Err(_) => not_found(),
        },
        None => not_found(),
    }
}

/// The file inside `root` that `rel` points to, or None if it escapes the folder.
fn resolve(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.split(['/', '\\']).any(|seg| seg == ".." || seg.contains(':')) {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let file = root.join(rel).canonicalize().ok()?;
    let ok = file.starts_with(&root) && file.is_file() && file.metadata().ok()?.len() <= MAX_FILE;
    ok.then_some(file)
}

fn respond(
    status: StatusCode,
    mime: &str,
    body: Vec<u8>,
    widget: Option<&str>,
) -> Response<Cow<'static, [u8]>> {
    let own = widget.map(|id| format!("{ORIGIN}/{id}/")).unwrap_or_default();
    let csp = format!(
        "default-src 'none'; \
         script-src 'unsafe-inline' {own} {ORIGIN}/kinora.js; \
         style-src 'unsafe-inline' {own}; \
         img-src {own} data: blob:; \
         font-src {own} data:; \
         media-src {own} blob:; \
         connect-src 'none'; form-action 'none'; base-uri 'none'"
    );
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Content-Security-Policy", csp)
        // Sandboxed widgets have an opaque origin; fonts are loaded with CORS.
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-cache")
        .body(Cow::Owned(body))
        .unwrap()
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    respond(StatusCode::NOT_FOUND, "text/plain", b"Not found".to_vec(), None)
}

fn mime(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ---- Arrange overlay ----

/// A transparent, top-most window over every monitor where widgets are dragged into place.
/// It only exists while arranging.
pub fn open_editor(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(EDITOR) {
        let _ = win.set_focus();
        return Ok(());
    }
    let monitors = monitors::list(app);
    let left = monitors.iter().map(|m| m.x).min().ok_or("No monitors found.")?;
    let top = monitors.iter().map(|m| m.y).min().unwrap_or(0);
    let right = monitors.iter().map(|m| m.x + m.width as i32).max().unwrap_or(0);
    let bottom = monitors.iter().map(|m| m.y + m.height as i32).max().unwrap_or(0);

    let win = WebviewWindowBuilder::new(app, EDITOR, WebviewUrl::App("editor.html".into()))
        .title("Arrange widgets")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = win.set_position(PhysicalPosition::new(left, top));
    let _ = win.set_size(PhysicalSize::new((right - left) as u32, (bottom - top) as u32));
    win.show().map_err(|e| e.to_string())?;
    let _ = win.set_focus();
    Ok(())
}

pub fn close_editor(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(EDITOR) {
        let _ = win.destroy();
    }
}
