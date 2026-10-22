import { useState } from "react";
import { motion } from "framer-motion";
import {
  FileCode2,
  FolderInput,
  FolderOpen,
  Sparkles,
  LayoutDashboard,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Settings2,
  Trash2,
  X,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { IconButton } from "@/components/icon-button";
import { WidgetFrame } from "@/components/widget-frame";
import { WidgetSettings } from "@/components/widget-settings";
import { Permissions } from "@/components/widget-permissions";
import aiGuide from "../../src-tauri/widgets/AI-GUIDE.md?raw";
import { BUILT_IN_WIDGETS, useWidgets, widgetCommands, type Widget } from "@/lib/widgets";

const errorMessage = (e: unknown) => (e instanceof Error ? e.message : String(e));

async function run(fn: () => Promise<unknown>) {
  try {
    await fn();
  } catch (e) {
    toast.error(errorMessage(e));
  }
}

export async function importWidgets(paths: string[]) {
  for (const p of paths) await run(() => widgetCommands.import(p));
}

async function copyAiGuide() {
  try {
    await navigator.clipboard.writeText(aiGuide);
    toast.success("AI guide copied. Paste it into your AI chat, then describe the widget you want.");
  } catch (e) {
    toast.error(errorMessage(e));
  }
}

async function pickWidget(directory: boolean) {
  const picked = await open(
    directory
      ? { directory: true, title: "Choose a widget folder" }
      : { filters: [{ name: "HTML widget", extensions: ["html", "htm"] }], title: "Choose an HTML file" },
  );
  if (typeof picked === "string") await importWidgets([picked]);
}

// Stands in for a wallpaper behind the preview.
const PREVIEW_BACKDROP =
  "radial-gradient(circle at 15% 20%, #3b3f8f, transparent 60%), radial-gradient(circle at 85% 85%, #8a3d5f, transparent 60%), #1c1c22";

/** A live, scaled-down render of the widget with its default settings. */
function Preview({ widget, revision }: { widget: Widget; revision: number }) {
  const BOX_W = 248;
  const BOX_H = 120;
  const { width, height } = widget.size;
  const scale = Math.min((BOX_W - 24) / width, (BOX_H - 24) / height, 1);
  return (
    <div
      className="relative flex h-[120px] items-center justify-center overflow-hidden rounded-lg border border-zinc-800"
      style={{ background: PREVIEW_BACKDROP }}
    >
      {widget.error ? (
        <p className="px-4 text-center text-xs text-destructive">{widget.error}</p>
      ) : (
        <div className="relative shrink-0" style={{ width: width * scale, height: height * scale }}>
          <div
            className="pointer-events-none absolute top-0 left-0 origin-top-left"
            style={{ width, height, transform: `scale(${scale})` }}
          >
            <WidgetFrame
              widget={widget}
              instance={{ id: "preview", widget: widget.id, monitor: "", x: 0, y: 0, width, height, settings: {} }}
              revision={revision}
              style={{ inset: 0 }}
            />
          </div>
        </div>
      )}
    </div>
  );
}

export function WidgetsPanel() {
  const { widgets, instances, revision } = useWidgets();
  const [editing, setEditing] = useState<string>();
  const editingInstance = instances.find((i) => i.id === editing);
  const byId = (id: string) => widgets.find((w) => w.id === id);

  return (
    <div className="-mx-1 flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-1 pb-4">
      <section>
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-medium text-muted-foreground">On your desktop</h2>
          <Button size="sm" variant="secondary" onClick={() => run(widgetCommands.openEditor)}>
            <LayoutDashboard /> Arrange on desktop
          </Button>
        </div>
        {instances.length === 0 ? (
          <div className="rounded-lg border border-dashed border-zinc-800 p-4 text-center text-sm text-muted-foreground">
            No widgets yet. Add one below, then drag it into place with <b>Arrange on desktop</b>.
          </div>
        ) : (
          <div className="flex flex-col divide-y divide-zinc-800 rounded-lg border border-zinc-800">
            {instances.map((inst) => {
              const w = byId(inst.widget);
              return (
                <div key={inst.id} className="flex items-center gap-3 py-1.5 pr-1.5 pl-3">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">{w?.name ?? inst.widget}</p>
                    <p className="truncate text-xs text-muted-foreground">
                      {w ? `${Math.round(inst.width)} × ${Math.round(inst.height)}` : "Widget not installed"}
                    </p>
                  </div>
                  <IconButton label="Customize" size="icon-sm" disabled={!w} onClick={() => setEditing(inst.id)}>
                    <Settings2 />
                  </IconButton>
                  <IconButton label="Remove from desktop" size="icon-sm" onClick={() => run(() => widgetCommands.remove(inst.id))}>
                    <X />
                  </IconButton>
                </div>
              );
            })}
          </div>
        )}
      </section>

      <section>
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-medium text-muted-foreground">Widgets</h2>
          <div className="flex items-center gap-1">
            <IconButton label="Reload widgets" size="icon-sm" onClick={() => run(widgetCommands.reload)}>
              <RefreshCw />
            </IconButton>
            <IconButton label="Open widgets folder" size="icon-sm" onClick={() => run(widgetCommands.openFolder)}>
              <FolderOpen />
            </IconButton>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button size="sm" variant="secondary">
                  <Plus /> Import
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onSelect={() => pickWidget(true)}>
                  <FolderInput /> Widget folder…
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={() => pickWidget(false)}>
                  <FileCode2 /> HTML file…
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-4">
          {widgets.map((w) => (
            <motion.div key={w.id} whileHover={{ y: -2 }} transition={{ duration: 0.15 }} className="group">
              <div className="relative">
                <Preview widget={w} revision={revision} />
                {!w.error && (
                  <button
                    onClick={() => run(() => widgetCommands.add(w.id))}
                    className="absolute inset-0 flex items-center justify-center gap-1.5 rounded-lg bg-black/55 text-sm font-medium opacity-0 transition-opacity group-hover:opacity-100"
                  >
                    <Plus className="size-4" /> Add to desktop
                  </button>
                )}
              </div>
              <div className="mt-1.5 flex items-center gap-1">
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm" title={w.description}>
                    {w.name}
                  </p>
                </div>
                {w.error && <Badge variant="destructive">Error</Badge>}
                {!BUILT_IN_WIDGETS.includes(w.id) && (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-xs" aria-label="More">
                        <MoreHorizontal />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem variant="destructive" onSelect={() => run(() => widgetCommands.uninstall(w.id))}>
                        <Trash2 /> Delete widget
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                )}
              </div>
              {w.description && <p className="line-clamp-2 text-xs text-muted-foreground">{w.description}</p>}
              <Permissions widget={w} className="mt-1.5" />
            </motion.div>
          ))}
        </div>
        <div className="mt-4 flex items-center justify-between gap-4 rounded-lg border border-zinc-800 p-3">
          <p className="text-xs text-muted-foreground">
            <span className="font-medium text-foreground">Make your own.</span> Copy the AI guide into ChatGPT,
            Claude or similar and describe the widget you want, or edit a copy of a folder in the widgets
            folder. Then press Reload.
          </p>
          <Button size="sm" variant="secondary" className="shrink-0" onClick={copyAiGuide}>
            <Sparkles /> Copy AI guide
          </Button>
        </div>
      </section>

      <WidgetSettings
        widget={editingInstance && byId(editingInstance.widget)}
        instance={editingInstance}
        onOpenChange={(o) => !o && setEditing(undefined)}
      />
    </div>
  );
}
