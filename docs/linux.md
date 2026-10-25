# Kinora on Linux

Kinora picks how to show the wallpaper from the desktop it runs on (`XDG_CURRENT_DESKTOP`). Set `KINORA_BACKEND` to `kde`, `layer-shell` or `stub` to force one.

| Desktop | Backend | How it works |
|---|---|---|
| KDE Plasma | `kde` | A Plasma wallpaper plugin (`kde-plugin/com.kinora.wallpaper`) plays the video. Kinora installs it into `~/.local/share/plasma/wallpapers/` and tells plasmashell what to play. |
| Hyprland (Omarchy), sway | `layer-shell` | The wallpaper window becomes a wlr-layer-shell surface on the background layer, click-through. |
| anything else | `stub` | A plain window, so the app still runs. Not a wallpaper. |

## Running from source

You need Node, Rust, `webkit2gtk-4.1`, and for the layer-shell backend `gtk-layer-shell`.

```sh
sudo pacman -S webkit2gtk-4.1 gtk-layer-shell   # Arch / Omarchy
npm install
npm run tauri dev
```

The first build takes a few minutes. Add an MP4 in the control panel, then click it to set the wallpaper.

## KDE Plasma

- Works on every monitor, with desktop icons and widgets above it, and pauses while a window is fullscreen on that screen.
- Kinora remembers each desktop's previous wallpaper plugin (in `~/.local/state/kinora/kde-previous.json`) and puts it back on Stop and Quit.
- Videos play through Qt Multimedia, so codecs depend on your FFmpeg build (HEVC and AV1 are fine on most setups). Library thumbnails may be missing, because the app's own webview can't decode them on Linux yet.
- Not done yet on KDE: widgets.

If a crash ever leaves the Kinora plugin on a desktop, put the stock wallpaper back with:

```sh
busctl --user call org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell evaluateScript s \
  'desktops().forEach(function (d) { d.wallpaperPlugin = "org.kde.image"; })'
```

## Hyprland / layer-shell (for testing)

Known limitations right now:

- **Video does not play yet.** The app's webview can't play video from Tauri's `asset://` protocol on Linux (WebKitGTK), so the window comes up black. A local media server is planned. Widgets that don't need video should show.
- **One monitor only.** A layer-shell surface covers a single output. The primary monitor is used, or set `KINORA_MONITOR` to a monitor name (for example `DP-1`).
- No media, audio or fullscreen auto-pause data yet. Pause on battery works.

Debug overrides: `KINORA_LAYER=background|bottom|top` (default `background`) and `KINORA_MONITOR=<name>`.

Useful to report back: does a black window appear behind your other windows and on the right monitor, are clicks passing through to the desktop, and does the log (the terminal running `npm run tauri dev`) show `platform: layer-shell`.
