import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { KinoraState } from "@/lib/commands";
import { clampRect, regionFor, type Region } from "@/lib/regions";
import { useWidgets } from "@/lib/widgets";
import { WidgetFrame } from "@/components/widget-frame";

/** The placed widgets, above the video. Hidden while a fullscreen app is open. */
export function WidgetLayer({ regions }: { regions: Region[] }) {
  const { widgets, instances, revision } = useWidgets();
  const [visible, setVisible] = useState(true);

  useEffect(() => {
    const off = listen<boolean>("widgets:visible", (e) => setVisible(e.payload));
    invoke<KinoraState>("get_state").then((s) => setVisible(!s.autoPaused));
    return () => {
      off.then((f) => f());
    };
  }, []);

  if (regions.length === 0) return null;
  return (
    <>
      {instances.map((inst) => {
        const widget = widgets.find((w) => w.id === inst.widget);
        if (!widget || widget.error) return null;
        const r = regionFor(regions, inst.monitor);
        const rect = clampRect(r, inst.x, inst.y, inst.width, inst.height);
        return (
          <WidgetFrame
            key={inst.id}
            widget={widget}
            instance={inst}
            revision={revision}
            visible={visible}
            actions
            style={{ left: r.left + rect.x, top: r.top + rect.y, width: rect.width, height: rect.height }}
          />
        );
      })}
    </>
  );
}
