# Kinora

A small Windows app that plays a looping video behind your desktop icons. Named after the 1890s Kinora flip-card viewer.

- Pick an MP4 / WebM video and set it as the wallpaper on all monitors
- Library of recent videos with thumbnails
- Play, pause, mute, fit, stop (restores your normal wallpaper)
- Tray icon; closing the window keeps Kinora running
- Restores the last wallpaper on launch, optional launch at login
- Auto-pauses when a fullscreen app or game covers the desktop (and optionally on battery)

## Stack

Tauri 2 · React + Vite + TypeScript · Tailwind + shadcn/ui (zinc, dark) · Zustand. Video plays in an HTML `<video>` inside borderless windows that Rust pins behind the desktop icons via Win32 (`src-tauri/src/desktop.rs`). Supports both the classic desktop layout (Windows 10, Windows 11 ≤ 23H2) and the raised layout (Windows 11 24H2+).

## Develop

Prerequisites: Node 20+, Rust stable, Visual Studio C++ Build Tools, WebView2 (built into Windows 11).

```bash
npm install
npm run tauri dev
```

> In Git Bash, `/usr/bin/link.exe` shadows the MSVC linker. Run cargo / tauri from PowerShell or a Developer Prompt.

## Build

```bash
npm run tauri build
```

Produces an NSIS installer in `src-tauri/target/release/bundle/nsis/`.

## Tips

- H.264 or VP9 at your monitor's resolution, ~8–15 Mbps for 1080p, keeps GPU use low.
- MKV and some HEVC files won't play (WebView2 limitation).
