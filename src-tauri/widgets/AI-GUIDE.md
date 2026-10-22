# Kinora widget spec (for AI assistants)

You are building a **Kinora widget**. Kinora is a Windows app that plays video wallpapers and
shows small HTML widgets on the desktop, behind the desktop icons. Follow this spec exactly.
Widgets that break these rules fail silently: blank tiles, missing fonts, settings that never
arrive.

The user will describe the widget they want after this document. If something they ask for
is impossible under the rules below (for example live weather, which needs the internet), say
so and offer the closest thing that works. Kinora *does* provide system stats, the media
that's playing (with artwork), synced lyrics, live audio levels, and Windows notifications
(see section 5).

---

## 1. What to deliver

A folder containing:

```
<widget-id>/
├─ widget.json     manifest (always include it)
├─ index.html      entry point
└─ ...             optional: .css, .js, images, .woff2 fonts
```

- `<widget-id>` is the folder name. Use only `a-z`, `0-9`, `-`, `_`, at most 64 characters,
  for example `pomodoro-ring`. Folders with other characters are ignored.
- Give the full content of every file in separate code blocks, each labelled with its path.
- Prefer one self-contained `index.html` with inline `<style>` and `<script>`. Add other files
  only for real assets (fonts, images).
- Tell the user how to install it: put the folder in the widgets folder (Kinora → Widgets →
  folder icon) or use **Import → Widget folder…**, then press **Reload**, then hover the
  widget and choose **Add to desktop**.

## 2. widget.json

Every field is optional. These are the defaults:

```json
{
  "name": "<folder name>",
  "description": "",
  "author": "",
  "version": "1.0.0",
  "entry": "index.html",
  "size": { "width": 320, "height": 180, "minWidth": 80, "minHeight": 60, "resizable": true },
  "glass": false,
  "radius": 0,
  "permissions": [],
  "settings": []
}
```

| Field | Meaning |
| --- | --- |
| `name`, `description`, `author`, `version` | Shown in Kinora's Widgets tab. |
| `entry` | HTML file to load, relative to the folder. |
| `size.width` / `size.height` | Default size in CSS pixels when the user adds the widget. |
| `size.minWidth` / `size.minHeight` | The smallest size the user can resize to. |
| `size.resizable` | Whether the user can resize it. |
| `radius` | Kinora clips the widget to rounded corners of this many px. Use it for tile-style widgets (24 matches the built-ins). |
| `glass` | Kinora blurs the wallpaper behind the widget. **Leave this `false`**: over a video it costs about 200 MB of GPU memory. Paint a semi-transparent background instead. |
| `permissions` | Live data and actions the widget uses: `"system"`, `"media"`, `"lyrics"`, `"audio"`, `"notifications"` (see section 5). List only what you use; users see these before adding the widget. |
| `settings` | User-editable options (see section 3). |

The JSON must be strictly valid: no comments, no trailing commas. A broken `widget.json`
shows the widget as an error.

## 3. Settings (how users customise the widget)

Each entry in `settings` becomes a form field in Kinora. Every placed copy of the widget has
its own values.

```json
"settings": [
  { "key": "title",  "type": "text",   "label": "Title", "placeholder": "Shown when empty", "default": "" },
  { "key": "size",   "type": "number", "label": "Text size (%)", "min": 50, "max": 200, "step": 5, "default": 100 },
  { "key": "color",  "type": "color",  "label": "Text color", "default": "#ffffff" },
  { "key": "theme",  "type": "select", "label": "Theme", "options": ["dark", "light", "clear"], "default": "dark" },
  { "key": "seconds","type": "toggle", "label": "Show seconds", "default": false }
]
```

| `type` | Value your code receives | Form control | Extra fields |
| --- | --- | --- | --- |
| `text` | string | text box | `placeholder` |
| `number` | number | slider if `min` and `max` are set, otherwise a number box | `min`, `max`, `step` |
| `color` | `"#rrggbb"` string | colour picker | none |
| `select` | one of `options` (string) | row of buttons, shown capitalised | `options` (keep to 2–4 short words) |
| `toggle` | boolean | switch | none |

Rules:
- `key` must be unique. Always give a `default` of the right type.
- The reserved key `glass` (type `toggle`) lets the user turn on Kinora's background blur. Only
  add it if the user asks, and label it as costing GPU memory.
- Your code receives an object with every key: the defaults, overlaid with what the user
  changed. Still keep your own copy of the defaults in JavaScript, so the widget renders
  correctly before settings arrive.

## 4. The Kinora SDK

Include it with this exact tag, an absolute path that starts with `/`:

```html
<script src="/kinora.js"></script>
```

It defines `window.Kinora`:

| API | Behaviour |
| --- | --- |
| `Kinora.onSettings(fn)` | Calls `fn(settings)` with the settings object, right away if they've already arrived, and again every time the user changes something (live, with no reload). |
| `Kinora.onVisibility(fn)` | Calls `fn(visible)` right away and whenever it changes. `false` while a fullscreen app or game is open. The widget is hidden then, so stop timers and animations. |
| `Kinora.on(topic, fn)` | Live data. `topic` is `"system"`, `"media"`, `"lyrics"` or `"audio"`. Calls `fn(data)` right away if data has already arrived, then on every update. Needs the matching permission (section 5). |
| `Kinora.notify(title, body)` | Shows a Windows notification. Needs the `"notifications"` permission. |
| `Kinora.onError(fn)` | Calls `fn(message)` when Kinora refuses a request, for example a missing permission or a rate limit. |
| `Kinora.settings` | The latest settings object, or `null` before the first one arrives. |
| `Kinora.visible` | The latest visibility (boolean). |
| `Kinora.permissions` | The permissions Kinora granted (from `widget.json`). |
| `Kinora.data` | The latest value per topic, e.g. `Kinora.data.system` (`undefined` until it arrives). |

That's the whole API: no storage, no network, no clicks.

## 5. Live data and notifications

Add the permission to `widget.json`, for example `"permissions": ["system", "media"]`. Without
it, `Kinora.on` never fires and `Kinora.notify` is refused. Data only flows while the widget
is on the desktop and visible (it pauses during fullscreen apps), so render whatever you
have and handle `undefined`/`null`.

### `"system"`: updates every second

```js
Kinora.on("system", function (d) {
  d.cpu.usage;        // 0-100, one decimal (the first sample can be 0)
  d.cpu.cores;        // logical processors, e.g. 16
  d.cpu.name;         // "AMD Ryzen 9 5900HX with Radeon Graphics"
  d.memory.used;      // bytes
  d.memory.total;     // bytes
  d.memory.usage;     // 0-100
  d.disks;            // [{ mount: "C:\\", used, total, usage }], fixed drives only (bytes, 0-100)
  d.network.down;     // bytes per second, all adapters
  d.network.up;       // bytes per second
  d.battery;          // null on desktops, else { percent: 0-100 or null, charging: bool, pluggedIn: bool }
  d.uptime;           // seconds since boot
});
```

Use 1000 for KB/MB/GB to match Windows' own speed display. Disk mounts end in a backslash
(`"C:\\"` in JavaScript source).

### `"media"`: the song or video playing in any app

This is the same source as the Windows volume flyout (Spotify, browsers, the Media Player app,
and so on). It's sent only when something changes: a new track, play/pause, a seek. It is
**not** sent every second.

```js
Kinora.on("media", function (m) {
  if (!m) { /* nothing is playing, or no media app is open */ return; }
  m.title;      // "Dark Red"
  m.artist;     // "Steve Lacy" (may be "")
  m.album;      // may be ""
  m.app;        // app id, e.g. "Spotify.exe" or "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"
  m.status;     // "playing" | "paused" | "stopped" | "changing" | "closed"
  m.playing;    // boolean
  m.position;   // seconds into the track at the moment `updatedAt`
  m.duration;   // seconds, 0 if unknown (live streams)
  m.updatedAt;  // Date.now()-style ms timestamp for `position`
  m.artwork;    // "data:image/...;base64,..." or null. Use it as <img src> or a CSS url("...")
});
```

To show a moving progress bar, extrapolate the position yourself instead of waiting for
updates:
`var pos = m.position + (m.playing ? (Date.now() - m.updatedAt) / 1000 : 0);`
Update it with a 1-second timer while `m.playing`, and clamp it to `m.duration`.

Widgets can't control playback (no clicks), so this is read-only.

### `"lyrics"`: lyrics for the playing song

Kinora looks the song up on lrclib.net (it sends the title and artist there, which is why
this is its own permission) and sends the result when the track changes. The lyrics don't
include the playback position, so **also request `"media"`** and use its
`position`/`updatedAt` to find the current line.

```js
Kinora.on("lyrics", function (l) {
  if (!l) return;          // nothing playing
  l.title; l.artist;       // the song these lyrics belong to; compare with media.title
  l.status;                // "loading" | "found" | "none"
  l.instrumental;          // true for instrumental tracks
  l.synced;                // [{ time: 12.34, text: "line" }, ...] sorted by time (seconds), or null
  l.plain;                 // the whole text with "\n" line breaks, or null
  l.source;                // "lrclib.net"
});
```

- `synced` lines can have empty `text` (instrumental breaks). Show `♪` or a gap.
- Prefer `synced`. Fall back to `plain` (scroll it with the song's progress), then to a "No
  lyrics found" message.
- To highlight the current line: the last line with `time <= position`. Schedule your next
  update with `setTimeout` for when the next line starts. Don't poll every frame.

### `"audio"`: live audio levels

The loudness of everything playing through the default speakers or headphones (the same meter
as the Windows volume mixer). It's **levels only**: no frequencies or waveform, and no audio
is recorded.

```js
Kinora.on("audio", function (a) {
  a.peak;   // 0-1, overall
  a.left;   // 0-1, left channel
  a.right;  // 0-1, right channel
});
```

- Arrives about **30 times a second** while sound is playing. During silence, one frame of
  zeros is sent and then nothing until sound resumes.
- Animate with `requestAnimationFrame`, easing between values (rise fast, fall slowly). Stop
  the loop once everything has settled at zero, and when `onVisibility(false)` is called.
- For a spectrum-like look, give each bar its own fixed curve or wobble scaled by the level.
  The built-in Audio Visualizer does this.

### `"notifications"`: send a Windows notification

```js
Kinora.notify("Time for a break", "Stand up and stretch.");
Kinora.onError(function (message) { console.warn(message); });
```

- At most one notification every **10 seconds** per widget. Extra calls are refused through
  `onError`.
- The title is cut at 80 characters and the body at 300. Plain text only.
- Notifications are only sent from the widget on the desktop, not from previews in Kinora's
  window, so the user never gets duplicates.
- Use them for events the user actually wants (a timer finishing, a reminder), never on a
  loop. Windows Do Not Disturb may send them silently to the notification center.
- Widgets can **send** notifications, but they can't read other apps' notifications.

## 6. The sandbox: hard rules

The widget runs in `<iframe sandbox="allow-scripts">` with an opaque (`null`) origin, served
with this Content-Security-Policy:

```
default-src 'none';
script-src 'unsafe-inline' <own folder> /kinora.js;
style-src 'unsafe-inline' <own folder>;
img-src <own folder> data: blob:;
font-src <own folder> data:;
media-src <own folder> blob:;
connect-src 'none'; form-action 'none'; base-uri 'none'
```

So:

**Allowed**
- Inline `<script>` and `<style>`, and `style="..."` attributes.
- Files from the widget's own folder through **relative** paths (`app.js`, `img/bg.png`,
  `fonts/x.woff2`).
- `data:` and `blob:` images, `data:` fonts, `<canvas>`, inline `<svg>`, CSS animations and
  transitions.
- `Intl`, `Date`, `Math`, and other plain JavaScript.
- Fonts installed on Windows by name (`"Segoe UI Variable Display"`, `"Segoe UI"`,
  `Consolas`, `Georgia`, `system-ui`).

**Blocked: the widget breaks if it relies on any of these**
- **Anything from the internet**: `fetch`/`XMLHttpRequest`/WebSocket (Kinora's data comes through `Kinora.on`, not fetch), CDN scripts (no React,
  Tailwind CDN, jQuery CDN or Google Fonts links), remote images, remote fonts. Bundle every
  asset in the folder, or inline it.
- `eval`, `new Function`, and string arguments to `setTimeout`/`setInterval`. Any library
  that needs these won't run.
- `localStorage`, `sessionStorage`, `indexedDB` and `document.cookie`: they throw a
  `SecurityError`. Keep state in memory. Anything that must persist belongs in a setting.
- Web Workers, service workers, popups (`window.open`), forms, navigation, `alert`/`confirm`/`prompt`.
- Reading files outside the widget's folder, or reaching the parent page (`parent`, `top`).
- **User input.** Widgets sit behind the desktop icons, so they never get clicks, hover,
  scroll or keyboard events. Don't build buttons, hover effects or inputs. All
  customisation goes through `settings`.
- Files larger than 10 MB aren't served.

Use classic `<script>` tags. Avoid `type="module"`.

**Watch out for global names.** A top-level `var` in a classic script becomes a property of
`window`. Names the browser already uses either fail silently or break things, for example
`history`, `name`, `status`, `location`, `top`, `parent`, `self`, `length`, `origin`, `event`,
`screen`, `frames`, `closed`, `open`, `close`, `stop`, `print`, `focus` and `blur`.
(`var history = {}` is silently ignored, so `history.cpu` is `undefined`.) Use specific names
like `samples` or `songName`, or wrap the whole script in `(function () { ... })();`.

## 7. Layout and look

- The widget's viewport is the widget, in CSS pixels. The user can resize it at any time, so
  size things with `vw`/`vh`/`%`/`min()`/`clamp()` or listen for `resize`. It must look right
  anywhere from `minWidth × minHeight` to very large.
- Start with `html, body { margin: 0; height: 100%; overflow: hidden; }`.
- The page is **transparent** by default and the wallpaper (often a moving video) shows
  through. For a tile look, paint a semi-transparent background such as
  `rgba(28, 28, 30, 0.78)` on a full-size wrapper. Kinora rounds the corners using `radius`.
- **Never set `color-scheme`** (CSS or meta tag). It makes the iframe background opaque white
  or black.
- Text must stay readable over any wallpaper: use a background tint, or a `text-shadow` on
  "clear" themes.
- Offer a `theme` setting (`dark` / `light` / `clear`) when it makes sense. The built-in
  widgets do.
- Remove focus outlines and selection styles. Nothing can be focused or selected anyway.

## 8. Performance

Widgets run all day next to a video wallpaper, so they must be light:

- Don't run a `requestAnimationFrame` loop unless it animates continuously, and stop it when
  `onVisibility(false)` is called.
- Line timers up with what changes. A clock without seconds should wake once a minute:
  `setTimeout(tick, 60000 - (Date.now() % 60000) + 20)`.
- When hidden, clear timers and set `animation-play-state: paused` on CSS animations.
- Keep the DOM small. No video backgrounds or large images unless the user asks for them.

## 9. Template

Start from this. It handles settings, visibility, themes and resizing correctly.

```html
<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      html, body { margin: 0; height: 100%; overflow: hidden; }
      body {
        --fg: #ffffff;
        --dim: rgba(235, 235, 245, 0.6);
        --tint: rgba(28, 28, 30, 0.78);
        font-family: "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif;
        color: var(--fg);
        -webkit-font-smoothing: antialiased;
      }
      body.light { --fg: #1c1c1e; --dim: rgba(60, 60, 67, 0.6); --tint: rgba(246, 246, 248, 0.86); }
      body.clear { --tint: transparent; text-shadow: 0 1px 12px rgba(0, 0, 0, 0.5); }
      .tile {
        box-sizing: border-box;
        height: 100%;
        display: flex;
        flex-direction: column;
        justify-content: center;
        padding: min(10vh, 8vw);
        background: var(--tint);
      }
      .title { font-size: min(30vh, 14vw); font-weight: 600; line-height: 1.1; }
      .sub { font-size: min(11vh, 6vw); color: var(--dim); margin-top: 0.4em; }
    </style>
  </head>
  <body class="dark">
    <div class="tile">
      <div class="title" id="title"></div>
      <div class="sub" id="sub"></div>
    </div>

    <script src="/kinora.js"></script>
    <script>
      // Defaults: keep these in sync with widget.json.
      var s = { title: "Hello", theme: "dark" };
      var timer = 0;

      function render() {
        document.body.className = s.theme;
        document.getElementById("title").textContent = s.title || "Hello";
        document.getElementById("sub").textContent = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      }

      function start() {
        clearTimeout(timer);
        render();
        timer = setTimeout(start, 60000 - (Date.now() % 60000) + 20); // next minute
      }

      Kinora.onSettings(function (next) {
        s = Object.assign(s, next);
        start();
      });

      Kinora.onVisibility(function (visible) {
        if (visible) start();
        else clearTimeout(timer);
      });

      start();
    </script>
  </body>
</html>
```

```json
{
  "name": "Hello",
  "description": "A minimal example widget.",
  "author": "You",
  "version": "1.0.0",
  "size": { "width": 320, "height": 160, "minWidth": 140, "minHeight": 80, "resizable": true },
  "radius": 24,
  "permissions": [],
  "settings": [
    { "key": "title", "type": "text", "label": "Title", "placeholder": "Hello", "default": "Hello" },
    { "key": "theme", "type": "select", "label": "Theme", "options": ["dark", "light", "clear"], "default": "dark" }
  ]
}
```

## 10. Checklist before you answer

- [ ] The folder name uses only `a-z 0-9 - _`.
- [ ] `widget.json` is valid JSON. Every setting has a unique `key`, a valid `type` and a `default`.
- [ ] `<script src="/kinora.js"></script>` comes before the code that uses `Kinora`.
- [ ] Every `Kinora.on(topic)` / `Kinora.notify` has its permission in `widget.json`, and no unused permissions are listed.
- [ ] The widget renders sensibly before live data arrives, and when `media` is `null`.
- [ ] Lyrics widgets also request `"media"`; audio animations stop when silent or hidden.
- [ ] No network requests, CDNs or remote fonts. Every asset is in the folder or inline.
- [ ] No `localStorage`/cookies/`eval`/workers/`type="module"`/popups.
- [ ] No top-level variables named after browser globals (`history`, `name`, `status`, `top`, …).
- [ ] No buttons, hover or inputs. Customisation goes through settings only.
- [ ] Nothing sets `color-scheme`.
- [ ] The layout scales from the minimum size to large sizes.
- [ ] Timers and animations stop when `onVisibility(false)`.
- [ ] The JavaScript defaults match the `widget.json` defaults.
- [ ] Every file is given in full, with install steps.
