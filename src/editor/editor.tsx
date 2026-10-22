import { useEffect, useRef, useState, type PointerEvent } from "react";
import { clampRect, regionFor, useMonitorRegions, type Region } from "@/lib/regions";
import { useWidgets, widgetCommands, type Instance, type Widget } from "@/lib/widgets";
import { WidgetFrame } from "@/components/widget-frame";

const GRID = 8;
const snap = (v: number) => Math.round(v / GRID) * GRID;

type Drag = {
  id: string;
  mode: "move" | "resize";
  startX: number;
  startY: number;
  // Absolute position in this window when the drag started.
  left: number;
  top: number;
  width: number;
  height: number;
  // Where the widget is now. Kept here, not read from state: pointerup can arrive before
  // React has re-rendered with the last move.
  latest?: Instance;
};

/** The region under a point, or the closest one. */
function regionAt(regions: Region[], x: number, y: number) {
  const dist = (r: Region) => {
    const dx = Math.max(r.left - x, 0, x - (r.left + r.width));
    const dy = Math.max(r.top - y, 0, y - (r.top + r.height));
    return dx * dx + dy * dy;
  };
  return regions.reduce((best, r) => (dist(r) < dist(best) ? r : best));
}

/**
 * Transparent overlay over every monitor. Shows each placed widget live, lets the user drag
 * and resize it, and saves on release. Esc or Done closes the window.
 */
export function Editor() {
  const regions = useMonitorRegions();
  const { widgets, instances, revision } = useWidgets();
  const [draft, setDraft] = useState<Record<string, Instance>>({});
  const drag = useRef<Drag | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") widgetCommands.closeEditor();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (regions.length === 0) return null;
  const primary = regions.find((r) => r.primary) ?? regions[0];

  const begin = (e: PointerEvent, inst: Instance, mode: Drag["mode"]) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    const r = regionFor(regions, inst.monitor);
    const rect = clampRect(r, inst.x, inst.y, inst.width, inst.height);
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    drag.current = {
      id: inst.id,
      mode,
      startX: e.clientX,
      startY: e.clientY,
      left: r.left + rect.x,
      top: r.top + rect.y,
      width: rect.width,
      height: rect.height,
    };
  };

  const move = (e: PointerEvent, inst: Instance, widget: Widget) => {
    const d = drag.current;
    if (!d || d.id !== inst.id) return;
    const dx = e.clientX - d.startX;
    const dy = e.clientY - d.startY;
    let next: Instance;
    if (d.mode === "move") {
      const left = d.left + dx;
      const top = d.top + dy;
      // The widget belongs to whichever monitor its center is on.
      const r = regionAt(regions, left + d.width / 2, top + d.height / 2);
      const rect = clampRect(r, snap(left - r.left), snap(top - r.top), d.width, d.height);
      next = { ...inst, monitor: r.name, ...rect };
    } else {
      const r = regionFor(regions, inst.monitor);
      const x = d.left - r.left;
      const y = d.top - r.top;
      const width = Math.min(Math.max(widget.size.minWidth, snap(d.width + dx)), r.width - x);
      const height = Math.min(Math.max(widget.size.minHeight, snap(d.height + dy)), r.height - y);
      next = { ...inst, monitor: r.name, x, y, width, height };
    }
    d.latest = next;
    setDraft((all) => ({ ...all, [inst.id]: next }));
  };

  const end = async (inst: Instance) => {
    const d = drag.current;
    if (d?.id !== inst.id) return;
    drag.current = null;
    if (d.latest) await widgetCommands.update(d.latest);
    setDraft(({ [inst.id]: _, ...rest }) => rest);
  };

  return (
    <>
      {regions.map((r) => (
        <div key={r.name} className="screen" style={{ left: r.left, top: r.top, width: r.width, height: r.height }} />
      ))}

      {instances.map((saved) => {
        const inst = draft[saved.id] ?? saved;
        const widget = widgets.find((w) => w.id === inst.widget);
        if (!widget || widget.error) return null;
        const r = regionFor(regions, inst.monitor);
        const rect = clampRect(r, inst.x, inst.y, inst.width, inst.height);
        const style = { left: r.left + rect.x, top: r.top + rect.y, width: rect.width, height: rect.height };
        return (
          <div
            key={inst.id}
            className={`box${drag.current?.id === inst.id ? " dragging" : ""}`}
            style={{ ...style, borderRadius: widget.radius }}
            onPointerDown={(e) => begin(e, inst, "move")}
            onPointerMove={(e) => move(e, inst, widget)}
            onPointerUp={() => end(inst)}
            onPointerCancel={() => end(inst)}
          >
            <WidgetFrame
              widget={widget}
              instance={inst}
              revision={revision}
              style={{ inset: 0, width: "100%", height: "100%" }}
            />
            <div className="outline" />
            {/* Above the widget, or inside it when there's no room at the top of the screen. */}
            <div
              className="tag"
              style={rect.y < 34 ? { top: 8, left: 8 } : undefined}
              onPointerDown={(e) => e.stopPropagation()}
            >
              {widget.name}
              <span className="size">
                {Math.round(rect.width)} × {Math.round(rect.height)}
              </span>
              <button title="Remove from desktop" onClick={() => widgetCommands.remove(inst.id)}>
                ✕
              </button>
            </div>
            {widget.size.resizable && (
              // Move/up events bubble from here to the box's handlers.
              <div className="handle" onPointerDown={(e) => begin(e, inst, "resize")} />
            )}
          </div>
        );
      })}

      {instances.length === 0 && (
        <div className="empty" style={{ left: primary.left + primary.width / 2, top: primary.top + primary.height / 2 }}>
          No widgets on the desktop yet. Add one from Kinora's Widgets tab.
        </div>
      )}

      <div className="toolbar" style={{ left: primary.left + primary.width / 2, top: primary.top + 24 }}>
        <span>Drag widgets to move them · drag the corner to resize · Esc to finish</span>
        <button onClick={() => widgetCommands.closeEditor()}>Done</button>
      </div>
    </>
  );
}
