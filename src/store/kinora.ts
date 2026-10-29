import { create } from "zustand";
import { nanoid } from "nanoid";
import { listen } from "@tauri-apps/api/event";
import { load, type Store } from "@tauri-apps/plugin-store";
import * as autostart from "@tauri-apps/plugin-autostart";
import { toast } from "sonner";
import { commands, type Fit, type KinoraState } from "@/lib/commands";
import { captureThumbnail } from "@/lib/thumbnails";

export type VideoItem = {
  id: string; // nanoid
  path: string; // absolute path on disk
  name: string; // file name without extension
  durationSec: number; // 0 when unknown
  thumb: string; // webp data URL, ~320x180; empty when the webview couldn't decode the file
  addedAt: number;
};

/** Read by the Rust watcher from the same kinora.json store. */
export type Settings = {
  pauseOnFullscreen: boolean;
  pauseOnBattery: boolean;
  startMuted: boolean;
};

/** Read by the Rust rotation thread from the same kinora.json store. */
export type Rotation = {
  enabled: boolean;
  paths: string[]; // library videos to cycle through
  intervalMin: number;
  shuffle: boolean;
};

export const VIDEO_EXTENSIONS = ["mp4", "webm", "m4v"];

const DEFAULT_SETTINGS: Settings = {
  pauseOnFullscreen: true,
  pauseOnBattery: false,
  startMuted: true,
};

const DEFAULT_ROTATION: Rotation = {
  enabled: false,
  paths: [],
  intervalMin: 15,
  shuffle: false,
};

type KinoraStore = {
  ready: boolean;
  library: VideoItem[];
  missing: string[]; // paths that no longer exist
  state: KinoraState;
  settings: Settings;
  rotation: Rotation;
  launchAtLogin: boolean;
  nativePlayback: boolean; // the desktop plays videos itself (Plasma), so previews can be missing

  init: () => Promise<void>;
  addFiles: (paths: string[]) => Promise<void>;
  remove: (id: string) => Promise<void>;
  setWallpaper: (path: string) => Promise<void>;
  togglePaused: () => Promise<void>;
  toggleMuted: () => Promise<void>;
  setFit: (fit: Fit) => Promise<void>;
  stop: () => Promise<void>;
  updateSettings: (patch: Partial<Settings>) => Promise<void>;
  updateRotation: (patch: Partial<Rotation>) => Promise<void>;
  toggleRotationMember: (path: string) => Promise<void>;
  nextWallpaper: () => Promise<void>;
  setLaunchAtLogin: (enabled: boolean) => Promise<void>;
};

let store: Store;

const fileName = (path: string) =>
  path.split(/[\\/]/).pop()!.replace(/\.[^.]+$/, "");

const errorMessage = (e: unknown) => (e instanceof Error ? e.message : String(e));

export const useKinora = create<KinoraStore>((set, get) => {
  const saveLibrary = async (library: VideoItem[]) => {
    set({ library });
    await store.set("library", library);
    await store.save();
  };

  /** Run a command that returns the new state; toast on failure. */
  const run = async (fn: () => Promise<KinoraState>) => {
    try {
      set({ state: await fn() });
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return {
    ready: false,
    library: [],
    missing: [],
    state: { current: null, paused: false, autoPaused: false, muted: true, fit: "cover" },
    settings: DEFAULT_SETTINGS,
    rotation: DEFAULT_ROTATION,
    launchAtLogin: false,
    nativePlayback: false,

    init: async () => {
      store = await load("kinora.json", { autoSave: false, defaults: {} });
      const library = (await store.get<VideoItem[]>("library")) ?? [];
      const settings = { ...DEFAULT_SETTINGS, ...(await store.get<Settings>("settings")) };
      const rotation = { ...DEFAULT_ROTATION, ...(await store.get<Rotation>("rotation")) };
      const [state, missing, launchAtLogin, platform] = await Promise.all([
        commands.getState(),
        commands.missingFiles(library.map((v) => v.path)),
        autostart.isEnabled().catch(() => false),
        commands.platformInfo(),
      ]);
      set({ ready: true, library, settings, rotation, state, missing, launchAtLogin, nativePlayback: platform.nativePlayback });

      // Tray actions and the watcher change state behind our back.
      listen<KinoraState>("kinora:state", (e) => set({ state: e.payload }));
      listen<string>("kinora:error", (e) => toast.error(e.payload, { id: "player-error" }));
    },

    addFiles: async (paths) => {
      const videos = paths.filter((p) =>
        VIDEO_EXTENSIONS.includes(p.split(".").pop()?.toLowerCase() ?? ""),
      );
      if (videos.length < paths.length) toast.error("Only MP4, WebM and M4V files can be added.");

      const known = new Set(get().library.map((v) => v.path));
      const added: VideoItem[] = [];
      for (const path of videos.filter((p) => !known.has(p))) {
        try {
          const { thumb, durationSec } = await captureThumbnail(path);
          added.push({ id: nanoid(), path, name: fileName(path), thumb, durationSec, addedAt: Date.now() });
        } catch (e) {
          // The webview can't decode it, but the desktop plays videos itself here.
          if (get().nativePlayback) {
            added.push({ id: nanoid(), path, name: fileName(path), thumb: "", durationSec: 0, addedAt: Date.now() });
          } else {
            toast.error(`${fileName(path)}: ${errorMessage(e)}`);
          }
        }
      }
      if (added.length) await saveLibrary([...added, ...get().library]);
    },

    remove: async (id) => {
      const gone = get().library.find((v) => v.id === id);
      await saveLibrary(get().library.filter((v) => v.id !== id));
      if (gone && get().rotation.paths.includes(gone.path)) {
        await get().updateRotation({ paths: get().rotation.paths.filter((p) => p !== gone.path) });
      }
    },

    setWallpaper: async (path) => {
      const missing = await commands.missingFiles([path]);
      if (missing.length) {
        set({ missing: [...new Set([...get().missing, path])] });
        toast.error("File not found. It may have been moved or deleted.");
        return;
      }
      await run(() => commands.setWallpaper(path));
    },

    togglePaused: () => run(() => commands.setPaused(!get().state.paused)),
    toggleMuted: () => run(() => commands.setMuted(!get().state.muted)),
    setFit: (fit) => run(() => commands.setFit(fit)),
    stop: () => run(() => commands.stopWallpaper()),

    updateSettings: async (patch) => {
      const settings = { ...get().settings, ...patch };
      set({ settings });
      await store.set("settings", settings);
      await store.save();
    },

    updateRotation: async (patch) => {
      const rotation = { ...get().rotation, ...patch };
      set({ rotation });
      await store.set("rotation", rotation);
      await store.save();
      await commands.rotationChanged();
    },

    toggleRotationMember: async (path) => {
      const { paths } = get().rotation;
      await get().updateRotation({
        paths: paths.includes(path) ? paths.filter((p) => p !== path) : [...paths, path],
      });
    },

    nextWallpaper: () => run(() => commands.nextWallpaper()),

    setLaunchAtLogin: async (enabled) => {
      try {
        await (enabled ? autostart.enable() : autostart.disable());
        set({ launchAtLogin: await autostart.isEnabled() });
      } catch (e) {
        toast.error(errorMessage(e));
      }
    },
  };
});
