import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type SettingValue = string | number | boolean;

export type SettingDef = {
  key: string;
  type: "text" | "number" | "color" | "select" | "toggle";
  label?: string;
  placeholder?: string;
  options?: string[];
  min?: number;
  max?: number;
  step?: number;
  default?: SettingValue;
};

export type Widget = {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  entry: string;
  size: { width: number; height: number; minWidth: number; minHeight: number; resizable: boolean };
  glass: boolean;
  radius: number;
  settings: SettingDef[];
  error: string | null;
};

/** A widget placed on the desktop. CSS pixels, relative to the monitor's top-left. */
export type Instance = {
  id: string;
  widget: string;
  monitor: string;
  x: number;
  y: number;
  width: number;
  height: number;
  settings: Record<string, SettingValue>;
};

export type WidgetsState = { widgets: Widget[]; instances: Instance[]; revision: number };

export const BUILT_IN_WIDGETS = ["clock", "quote"];

export const widgetCommands = {
  get: () => invoke<WidgetsState>("get_widgets"),
  reload: () => invoke<WidgetsState>("reload_widgets"),
  add: (widget: string) => invoke<WidgetsState>("add_widget", { widget }),
  update: (instance: Instance) => invoke<WidgetsState>("update_widget", { instance }),
  remove: (id: string) => invoke<WidgetsState>("remove_widget", { id }),
  import: (path: string) => invoke<WidgetsState>("import_widget", { path }),
  uninstall: (widget: string) => invoke<WidgetsState>("uninstall_widget", { widget }),
  openFolder: () => invoke<void>("open_widgets_folder"),
  openEditor: () => invoke<void>("open_widget_editor"),
  closeEditor: () => invoke<void>("close_widget_editor"),
};

/** Installed widgets and placed instances, kept live across windows. */
export function useWidgets() {
  const [state, setState] = useState<WidgetsState>({ widgets: [], instances: [], revision: 0 });
  useEffect(() => {
    const off = listen<WidgetsState>("widgets:changed", (e) => setState(e.payload));
    widgetCommands.get().then(setState);
    return () => {
      off.then((f) => f());
    };
  }, []);
  return state;
}

/** Manifest defaults overlaid with the user's values. */
export function resolveSettings(widget: Widget, instance: Instance) {
  const out: Record<string, SettingValue> = {};
  for (const def of widget.settings) {
    if (def.key && def.default !== undefined) out[def.key] = def.default;
  }
  return { ...out, ...instance.settings };
}

export function widgetUrl(widget: Widget, revision: number) {
  const entry = widget.entry.split(/[\\/]/).map(encodeURIComponent).join("/");
  return `http://kwidget.localhost/${widget.id}/${entry}?r=${revision}`;
}
