# Making Kinora widgets

A widget is a folder in this directory with an `index.html`. Kinora shows it on your desktop
behind the icons, and you drag it into place with **Widgets → Arrange on desktop**.

```
widgets/
└─ my-widget/          folder name = widget id (letters, digits, - and _)
   ├─ index.html       required
   ├─ widget.json      optional: name, size, settings
   └─ ...              your CSS, JS, images and fonts
```

**Using an AI to make one?** Give it `AI-GUIDE.md` from this folder (or press **Copy AI guide**
in the Widgets tab), then describe the widget you want.

After adding or editing a folder, press **Reload** in the Widgets tab. The `clock` and `quote`
folders are good starting points. Copy one, rename the folder, and change it.

## widget.json

```json
{
  "name": "My widget",
  "description": "What it does",
  "author": "You",
  "version": "1.0.0",
  "entry": "index.html",
  "size": { "width": 320, "height": 160, "minWidth": 120, "minHeight": 80, "resizable": true },
  "glass": false,
  "radius": 24,
  "settings": [
    { "key": "title",  "type": "text",   "label": "Title", "placeholder": "Hello", "default": "" },
    { "key": "count",  "type": "number", "label": "Count", "min": 1, "max": 10, "step": 1, "default": 3 },
    { "key": "color",  "type": "color",  "label": "Color", "default": "#ffffff" },
    { "key": "style",  "type": "select", "label": "Style", "options": ["a", "b"], "default": "a" },
    { "key": "glass",  "type": "toggle", "label": "Blur background", "default": false }
  ]
}
```

- `radius`: Kinora rounds the widget's corners by this many pixels.
- `glass`: Kinora blurs the wallpaper behind the widget. It looks great, but blurring over a
  playing video costs a lot of GPU memory (about 200 MB), so keep it `false` and paint a
  semi-transparent background instead. A toggle setting with the key `glass` lets users turn
  the blur on if they want it.
- `settings`: Kinora builds a form for these in the control panel. The types are `text`,
  `number`, `color`, `select` and `toggle`. Each placed copy of a widget has its own values.

## Getting settings

```html
<script src="/kinora.js"></script>
<script>
  Kinora.onSettings((s) => {
    document.body.style.color = s.color;
  });

  // False while a fullscreen app or game is open. Stop timers and animations then.
  Kinora.onVisibility((visible) => {});
</script>
```

## Live data and notifications

Declare what a widget uses in `widget.json` with `"permissions": ["system", "media", "lyrics", "audio", "notifications"]`.
Users see these on the widget's card.

```js
Kinora.on("system", (d) => {});  // every second: d.cpu.usage, d.memory.usage, d.disks, d.network.down/up, d.battery
Kinora.on("media", (m) => {});   // when it changes: m.title, m.artist, m.album, m.artwork (data URL), m.playing,
                                 // m.position at m.updatedAt, m.duration. null when nothing is playing
Kinora.on("lyrics", (l) => {});  // per track: l.synced [{time, text}] or l.plain, from lrclib.net (also request "media")
Kinora.on("audio", (a) => {});   // ~30/s while sound plays: a.peak, a.left, a.right (0-1)
Kinora.notify("Title", "Body");  // Windows notification, at most one every 10 seconds per widget
Kinora.onError((message) => {}); // a request was refused
```

The built-in widgets show each one in use: **System** and **Activity Graph** (system),
**Now Playing** (media), **Lyrics** (media + lyrics), **Audio Visualizer** (audio), and
**Break Reminder** and **Countdown** (notifications).
and `AI-GUIDE.md` documents every field.

## What widgets can't do

Widgets run in a locked-down sandbox, so a widget you download can't harm your PC:

- No network access (`fetch`, images or fonts from the internet). Bundle everything in the folder.
- No reading other apps' notifications or controlling media. The data above is read-only.
- No access to files outside its own folder, to Kinora, or to other widgets.
- No popups, navigation, forms, cookies or storage.
- No clicks: widgets sit behind your desktop icons, so they're for display only.

Keep widgets light. Avoid busy `requestAnimationFrame` loops and stop timers when
`onVisibility(false)` is called. Don't set `color-scheme` in your CSS, because it makes the
widget's background opaque.
