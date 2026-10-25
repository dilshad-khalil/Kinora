import { invoke } from "@tauri-apps/api/core";

export type Fit = "cover" | "contain" | "fill";

export type KinoraState = {
  current: string | null;
  paused: boolean;
  autoPaused: boolean;
  muted: boolean;
  fit: Fit;
};

export type MonitorInfo = {
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
};

export type PlatformInfo = {
  name: string;
  /** The desktop plays the video itself, so the webview never has to decode it. */
  nativePlayback: boolean;
};

export const commands = {
  setWallpaper: (path: string) => invoke<KinoraState>("set_wallpaper", { path }),
  stopWallpaper: () => invoke<KinoraState>("stop_wallpaper"),
  setPaused: (paused: boolean) => invoke<KinoraState>("set_paused", { paused }),
  setMuted: (muted: boolean) => invoke<KinoraState>("set_muted", { muted }),
  setFit: (fit: Fit) => invoke<KinoraState>("set_fit", { fit }),
  getState: () => invoke<KinoraState>("get_state"),
  platformInfo: () => invoke<PlatformInfo>("platform_info"),
  listMonitors: () => invoke<MonitorInfo[]>("list_monitors"),
  missingFiles: (paths: string[]) => invoke<string[]>("missing_files", { paths }),
};
