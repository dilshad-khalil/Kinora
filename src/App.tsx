import { useEffect, useState } from "react";
import { Minus, Upload, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { IconButton } from "@/components/icon-button";
import { NowPlaying } from "@/components/now-playing";
import { LibraryGrid } from "@/components/library-grid";
import { SettingsSheet } from "@/components/settings-sheet";
import { useKinora } from "@/store/kinora";

function TitleBar() {
  const win = getCurrentWindow();
  return (
    <header data-tauri-drag-region className="flex h-11 shrink-0 items-center justify-between pr-1.5 pl-4">
      <span data-tauri-drag-region className="text-sm font-semibold tracking-tight">
        Kinora
      </span>
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

  useEffect(() => {
    init();
    const off = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "enter" || e.payload.type === "over") setDragging(true);
      else if (e.payload.type === "leave") setDragging(false);
      else if (e.payload.type === "drop") {
        setDragging(false);
        addFiles(e.payload.paths);
      }
    });
    return () => {
      off.then((f) => f());
    };
  }, [init, addFiles]);

  return (
    <TooltipProvider delayDuration={400}>
      <div className="flex h-full flex-col" onContextMenu={(e) => e.preventDefault()}>
        <TitleBar />
        {ready && (
          <main className="flex min-h-0 flex-1 flex-col gap-5 px-4 pb-0">
            <NowPlaying />
            <LibraryGrid />
          </main>
        )}
        {dragging && (
          <div className="pointer-events-none fixed inset-2 flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed border-zinc-500 bg-background/85 text-sm">
            <Upload className="size-6" />
            Drop videos to add them to your library
          </div>
        )}
      </div>
      <Toaster position="bottom-right" />
    </TooltipProvider>
  );
}
