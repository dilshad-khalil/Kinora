# Kinora on Linux

Kinora picks a backend from your desktop (`XDG_CURRENT_DESKTOP`). Force one with `KINORA_BACKEND=kde|layer-shell|stub`.

| Desktop                  | Backend       | Status                                              |
| ------------------------ | ------------- | --------------------------------------------------- |
| KDE Plasma               | `kde`         | Video wallpaper works. No widgets yet.              |
| Hyprland (Omarchy), sway | `layer-shell` | Experimental: window shows, video doesn't play yet. |
| Anything else            | `stub`        | A plain window, not a wallpaper.                    |

## Install

| Distro                  | Command                                                                                                            |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Zorin / Ubuntu / Debian | `npm run tauri build -- --bundles deb`, then `sudo apt install ./src-tauri/target/release/bundle/deb/Kinora_*.deb` |
| Arch / Omarchy          | `cd packaging/arch && makepkg -si`                                                                                 |
| Any                     | `npm run tauri build -- --no-bundle`, then run `src-tauri/target/release/kinora`                                   |

Building needs Node and Rust. Tagged releases on GitHub include the `.deb` and the Arch package.

## Run from source

```sh
sudo pacman -S webkit2gtk-4.1 gtk-layer-shell   # Arch / Omarchy
npm install
npm run tauri dev
```

Add an MP4 in the control panel, then click it to set the wallpaper. The log should say which backend was picked (`platform: kde`).

## KDE Plasma

Kinora installs a Plasma wallpaper plugin (`kde-plugin/`) and tells Plasma what to play.

- Plays on every monitor, under your icons and panels.
- Pauses while a window is fullscreen on that screen.
- Your previous wallpapers come back on Stop and Quit. They're remembered in `~/.local/state/kinora/kde-previous.json`.
- Codecs depend on your FFmpeg build; HEVC and AV1 work on most setups.
- Library thumbnails may be blank, because the app's webview can't decode video on Linux yet.

If a crash leaves the Kinora wallpaper stuck, restore the default with:

```sh
busctl --user call org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell evaluateScript s \
  'desktops().forEach(function (d) { d.wallpaperPlugin = "org.kde.image"; })'
```

## Hyprland (testing)

The wallpaper window becomes a click-through layer-shell surface on the background layer.

Known limitations:

- **No video yet.** WebKitGTK can't play video from Tauri's `asset://` protocol, so the window is black. A local media server is planned.
- **One monitor.** The primary one, or set `KINORA_MONITOR=DP-1` (a monitor name).
- No media, audio or fullscreen auto-pause data. Pause on battery works.

Debug override: `KINORA_LAYER=background|bottom|top` (default `background`).

When testing, check that:

- the log shows `platform: layer-shell`
- a black window sits behind your apps, on the right monitor
- clicks pass through to the desktop
