<p align="center">
  <img src="assets/kinora-icon.svg" alt="Kinora" width="96" height="96">
</p>

<h1 align="center">Kinora</h1>

<p align="center">
  <b>A video wallpaper app for Windows, with desktop widgets.</b><br>
  Play a looping video behind your desktop icons and place live HTML widgets on top of it.
</p>

<p align="center">
  <a href="https://github.com/dilshad-khalil/Kinora/releases/latest">Download</a> ·
  <a href="#widgets">Widgets</a> ·
  <a href="#making-your-own-widget">Make a widget</a> ·
  <a href="#development">Development</a>
</p>

---

Kinora is a small Windows app that does two things:

1. **Live wallpaper.** It plays a video (MP4 or WebM) as your desktop background, on every monitor, behind your icons.
2. **Desktop widgets.** It shows small widgets on your desktop: clocks, system stats, the song that's playing, synced lyrics and more. Widgets are plain HTML, so anyone can make one.

It's named after the Kinora, a flip-card viewer from the 1890s.

## Wallpaper

- Pick an MP4 or WebM video and set it as the wallpaper on **all monitors**.
- A library of your recent videos, with thumbnails.
- Play, pause, mute, change how the video fits (fill, fit, stretch) and stop. Stopping restores your normal wallpaper.
- **Saves power:** playback pauses on its own when a fullscreen app or game covers the desktop, and optionally when you're on battery.
- Lives in the **system tray**. Closing the window keeps the wallpaper running.
- Remembers your last wallpaper and restores it at launch. You can also launch Kinora at login.
- Works with both Windows desktop layouts: the classic one (Windows 10, Windows 11 up to 23H2) and the newer one (Windows 11 24H2 and later).

**Tips for smooth playback**

- H.264 or VP9 at your monitor's resolution is easiest on the GPU. About 8–15 Mbps is plenty for 1080p.
- MKV files and some HEVC files won't play. This is a WebView2 limitation.

## Widgets

Widgets sit on your desktop behind the icons, on top of your wallpaper. Open the **Widgets** tab, add the ones you want, then use **Arrange on desktop** to drag them into place and resize them. Each widget has its own settings (colors, text, dates and so on), and every copy you place keeps its own values.

Kinora comes with these widgets:

| Widget               | What it shows                                                                         |
| -------------------- | ------------------------------------------------------------------------------------- |
| **Clock**            | A digital clock in the style of macOS widgets                                         |
| **Analog Clock**     | A classic clock face with a sweeping second hand                                      |
| **Calendar**         | This month at a glance, with today highlighted                                        |
| **Countdown**        | Days, hours and minutes until a date, with a notification when it arrives             |
| **Break Reminder**   | Counts down to your next break and sends a Windows notification                       |
| **Sticky Note**      | A handwritten note for your desktop                                                   |
| **Quote**            | A rotating quote, or one you write yourself                                           |
| **System**           | CPU, memory and disk, plus network speed and battery                                  |
| **Activity Graph**   | A live chart of CPU, memory or network activity over the last few minutes             |
| **Now Playing**      | The song or video playing in Spotify, your browser or any media app, with artwork     |
| **Lyrics**           | Time-synced lyrics for the song that's playing, from [lrclib.net](https://lrclib.net) |
| **Audio Visualizer** | Bars that move with whatever your PC is playing                                       |

You can also import a widget from a folder or a single `.html` file, and remove any widget you don't want.

### Safe by design

Widgets run in a locked-down sandbox, so a widget you download can't harm your PC. Widgets can't use the network, read your files, open popups or reach Kinora itself. They can only use the live data they ask for. A widget lists what it uses (system stats, media, lyrics, audio levels, notifications) and you can see this on its card before you add it.

## Making your own widget

A widget is a folder with an `index.html` and an optional `widget.json` for its name, size and settings:

```
my-widget/
├─ index.html      required
├─ widget.json     optional: name, size, settings, permissions
└─ ...             your CSS, JS, images and fonts
```

Drop the folder into the widgets folder (the Widgets tab has a button that opens it), press **Reload**, and it appears in the list. The `clock` and `quote` widgets are good starting points.

```html
<script src="/kinora.js"></script>
<script>
  Kinora.onSettings((s) => {
    document.body.style.color = s.color;
  });
  Kinora.on("system", (d) => {
    /* d.cpu.usage, d.memory.usage, ... */
  });
</script>
```

Don't want to write the code yourself? Press **Copy AI guide** in the Widgets tab, give it to an AI assistant, and describe the widget you want.

The full reference is in [`src-tauri/widgets/README.md`](src-tauri/widgets/README.md), and the spec written for AI assistants is [`src-tauri/widgets/AI-GUIDE.md`](src-tauri/widgets/AI-GUIDE.md).

## Install

Download the latest installer from the [Releases page](https://github.com/dilshad-khalil/Kinora/releases/latest) and run it. Kinora needs Windows 10 or 11 and WebView2 (included with Windows 11).

## Development

Kinora is built with [Tauri 2](https://tauri.app), React, Vite and TypeScript, with Tailwind and shadcn/ui for the interface and Zustand for state. The video plays in an HTML `<video>` inside borderless windows that the Rust side pins behind the desktop icons using Win32 (`src-tauri/src/desktop.rs`). Widgets are shown in sandboxed frames, and live data (system, media, lyrics, audio) comes from the Rust side.

**Prerequisites:** Node 20+, Rust (stable), Visual Studio C++ Build Tools and WebView2.

```bash
npm install
npm run tauri dev      # run the app in development
npm run tauri build    # build the installer
```

The installer is created in `src-tauri/target/release/bundle/nsis/`.

> In Git Bash, `/usr/bin/link.exe` shadows the MSVC linker. Run cargo and tauri from PowerShell or a Developer Prompt.

### Project layout

```
src/               React interface (control panel, wallpaper player, widget layer, widget editor)
src-tauri/src/     Rust backend (desktop pinning, monitors, widgets, system/media/lyrics/audio data, tray)
src-tauri/widgets/ The built-in widgets, plus the widget guides and kinora.js
```

### Releases

Every push to `main` builds the Windows installer and publishes it as a new GitHub release (see `.github/workflows/release.yml`). The version is a patch bump on the latest tag. Put `[minor]` or `[major]` in the commit message to bump those instead. Changes to Markdown files don't trigger a release.
