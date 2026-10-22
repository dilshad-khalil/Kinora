import { useEffect, useRef, useState } from "react";
import { Minus, Upload, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { IconButton } from "@/components/icon-button";
import { NowPlaying } from "@/components/now-playing";
import { LibraryGrid } from "@/components/library-grid";
import { SettingsSheet } from "@/components/settings-sheet";
import { WidgetsPanel, importWidgets } from "@/components/widgets-panel";
import { cn } from "@/lib/utils";
import { useKinora } from "@/store/kinora";

type Tab = "wallpapers" | "widgets";
const TABS: { value: Tab; label: string }[] = [
  { value: "wallpapers", label: "Wallpapers" },
  { value: "widgets", label: "Widgets" },
];

function TitleBar({ tab, onTab }: { tab: Tab; onTab: (t: Tab) => void }) {
  const win = getCurrentWindow();
  return (
    <header data-tauri-drag-region className="flex h-11 shrink-0 items-center justify-between pr-1.5 pl-4">
      <div data-tauri-drag-region className="flex items-center gap-4">
        <span data-tauri-drag-region className="text-sm font-semibold tracking-tight">
          Kinora
        </span>
        <nav className="flex gap-0.5 rounded-md bg-zinc-900 p-0.5">
          {TABS.map((t) => (
            <button
              key={t.value}
              onClick={() => onTab(t.value)}
              className={cn(
                "rounded px-2.5 py-0.5 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground",
                tab === t.value && "bg-zinc-700 text-foreground",
              )}
            >
              {t.label}
            </button>
          ))}
        </nav>
      </div>
      <div className="flex items-center gap-0.5">
        <SettingsSheet />
        <IconButton label="Minimize" size="icon-sm" onClick={() => win.minimize()}>
          <Minus />
        </IconButton>
        {/* Closing destroys the panel to free its memory; Kinora keeps running in the tray. */}
        <IconButton label="Close to tray" size="icon-sm" onClick={() => win.close()}>
          <X />
        </IconButton>
      </div>
    </header>
  );
}

export default function App() {
  const { ready, init, addFiles } = useKinora();
  const [dragging, setDragging] = useState(false);
  const [tab, setTab] = useState<Tab>("wallpapers");
  const tabRef = useRef(tab);
  tabRef.current = tab;

  useEffect(() => {
    init();
    const off = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "enter" || e.payload.type === "over") setDragging(true);
      else if (e.payload.type === "leave") setDragging(false);
      else if (e.payload.type === "drop") {
        setDragging(false);
        if (tabRef.current === "widgets") importWidgets(e.payload.paths);
        else addFiles(e.payload.paths);
      }
    });
    return () => {
      off.then((f) => f());
    };
  }, [init, addFiles]);

  return (
    <TooltipProvider delayDuration={400}>
      <div className="flex h-full flex-col" onContextMenu={(e) => e.preventDefault()}>
        <TitleBar tab={tab} onTab={setTab} />
        {ready && (
          <main className="flex min-h-0 flex-1 flex-col gap-5 px-4 pb-0">
            {tab === "wallpapers" ? (
              <>
                <NowPlaying />
                <LibraryGrid />
              </>
            ) : (
              <WidgetsPanel />
            )}
          </main>
        )}
        {dragging && (
          <div className="pointer-events-none fixed inset-2 flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed border-zinc-500 bg-background/85 text-sm">
            <Upload className="size-6" />
            {tab === "widgets"
              ? "Drop a widget folder or .html file to import it"
              : "Drop videos to add them to your library"}
          </div>
        )}
      </div>
      <Toaster position="bottom-right" />
    </TooltipProvider>
  );
}
