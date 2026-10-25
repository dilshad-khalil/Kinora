import { motion } from "framer-motion";
import { Film, FolderOpen, MoreHorizontal, Trash2 } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useKinora, type VideoItem } from "@/store/kinora";

function formatDuration(sec: number) {
  if (!Number.isFinite(sec) || sec <= 0) return "";
  const s = Math.round(sec);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export function VideoCard({ video }: { video: VideoItem }) {
  const { state, missing, setWallpaper, remove } = useKinora();
  const active = state.current === video.path;
  const isMissing = missing.includes(video.path);

  return (
    <motion.div whileHover={{ y: -2 }} transition={{ duration: 0.15 }} className="group">
      <div
        className={cn(
          "relative aspect-video overflow-hidden rounded-lg border border-zinc-800 bg-zinc-900",
          active && "ring-2 ring-zinc-300 ring-offset-2 ring-offset-background",
          isMissing && "opacity-50",
        )}
      >
        {video.thumb ? (
          <img src={video.thumb} alt="" className="size-full object-cover" draggable={false} />
        ) : (
          <div className="flex size-full items-center justify-center text-zinc-600">
            <Film className="size-8" />
          </div>
        )}

        {!isMissing && !active && (
          <button
            onClick={() => setWallpaper(video.path)}
            className="absolute inset-0 flex items-center justify-center bg-black/55 text-sm font-medium opacity-0 transition-opacity group-hover:opacity-100"
          >
            Set as wallpaper
          </button>
        )}

        <div className="absolute bottom-1.5 left-1.5 flex gap-1">
          {isMissing ? (
            <Badge variant="destructive">File not found</Badge>
          ) : (
            video.durationSec > 0 && (
              <Badge variant="secondary" className="bg-black/60 tabular-nums">
                {formatDuration(video.durationSec)}
              </Badge>
            )
          )}
        </div>
      </div>

      <div className="mt-1.5 flex items-center gap-1">
        <p className="min-w-0 flex-1 truncate text-sm" title={video.path}>
          {video.name}
        </p>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon-xs" aria-label="More">
              <MoreHorizontal />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem disabled={isMissing} onSelect={() => revealItemInDir(video.path)}>
              <FolderOpen /> Show in folder
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" onSelect={() => remove(video.id)}>
              <Trash2 /> Remove from library
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </motion.div>
  );
}
