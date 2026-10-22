import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Topics a widget can subscribe to (each needs the same-named permission). */
export const DATA_TOPICS = ["system", "media", "lyrics", "audio"] as const;

export const PERMISSION_LABELS: Record<string, string> = {
  system: "System stats",
  media: "Now playing",
  lyrics: "Lyrics (lrclib.net)",
  audio: "Audio levels",
  notifications: "Notifications",
};

type Listener = (data: unknown) => void;

const latest = new Map<string, unknown>();
const listeners = new Map<string, Set<Listener>>();
let started = false;

function start() {
  if (started) return;
  started = true;
  listen<{ topic: string; data: unknown }>("widgets:data", (e) => {
    latest.set(e.payload.topic, e.payload.data);
    listeners.get(e.payload.topic)?.forEach((fn) => fn(e.payload.data));
  });
}

/** Live data for one topic. One Tauri listener per page, shared by every widget on it. */
export function subscribe(topic: string, fn: Listener) {
  start();
  let set = listeners.get(topic);
  if (!set) listeners.set(topic, (set = new Set()));
  set.add(fn);

  if (latest.has(topic)) {
    fn(latest.get(topic));
  } else {
    // Sent before this page opened? Rust keeps the last value.
    invoke<unknown>("widget_data", { topic }).then((data) => {
      if (data !== null && !latest.has(topic)) {
        latest.set(topic, data);
        fn(data);
      }
    });
  }
  return () => {
    set.delete(fn);
  };
}

export function latestData(topic: string) {
  return latest.get(topic);
}
