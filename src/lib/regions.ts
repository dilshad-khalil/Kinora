import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { MonitorInfo } from "@/lib/commands";

/** A monitor's area inside a window that spans every monitor, in CSS pixels. */
export type Region = {
  name: string;
  primary: boolean;
  left: number;
  top: number;
  width: number;
  height: number;
};

/** One region per monitor, relative to the window's top-left (the monitors' bounding box). */
export function useMonitorRegions() {
  const [regions, setRegions] = useState<Region[]>([]);
  useEffect(() => {
    invoke<MonitorInfo[]>("list_monitors").then((monitors) => {
      const dpr = window.devicePixelRatio;
      const x0 = Math.min(...monitors.map((m) => m.x));
      const y0 = Math.min(...monitors.map((m) => m.y));
      setRegions(
        monitors.map((m) => ({
          name: m.name,
          primary: m.primary,
          left: (m.x - x0) / dpr,
          top: (m.y - y0) / dpr,
          width: m.width / dpr,
          height: m.height / dpr,
        })),
      );
    });
  }, []);
  return regions;
}

/** The instance's monitor, or the primary one if that monitor is gone. */
export function regionFor(regions: Region[], monitor: string) {
  return regions.find((r) => r.name === monitor) ?? regions.find((r) => r.primary) ?? regions[0];
}

/** Keep a rect inside its region. */
export function clampRect(r: Region, x: number, y: number, width: number, height: number) {
  width = Math.min(width, r.width);
  height = Math.min(height, r.height);
  return {
    x: Math.max(0, Math.min(x, r.width - width)),
    y: Math.max(0, Math.min(y, r.height - height)),
    width,
    height,
  };
}
