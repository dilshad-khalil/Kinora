import { AnimatePresence, motion } from "framer-motion";
import { Expand, Film, Pause, Play, Square, Volume2, VolumeX } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Button } from "@/components/ui/button";
import { IconButton } from "@/components/icon-button";
import type { Fit } from "@/lib/commands";
import { useKinora } from "@/store/kinora";

const FITS: { value: Fit; label: string }[] = [
  { value: "cover", label: "Cover" },
  { value: "contain", label: "Contain" },
  { value: "fill", label: "Stretch" },
];

export function NowPlaying() {
  const { state, library, togglePaused, toggleMuted, setFit, stop } = useKinora();
  const item = library.find((v) => v.path === state.current);
  const name = item?.name ?? state.current?.split(/[\\/]/).pop();

  return (
    <Card className="gap-0 overflow-hidden rounded-lg border-zinc-800 p-0">
      <AnimatePresence mode="wait" initial={false}>
        {state.current ? (
          <motion.div
            key={state.current}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -6 }}
            transition={{ duration: 0.18 }}
            className="flex items-center gap-4 p-3"
          >
            <div className="aspect-video w-36 shrink-0 overflow-hidden rounded-md bg-zinc-900">
              {item?.thumb && <img src={item.thumb} alt="" className="size-full object-cover" />}
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-xs text-muted-foreground">Now playing</p>
              <p className="truncate font-medium">{name}</p>
              <div className="mt-1 flex gap-1.5">
                {state.paused && <Badge variant="secondary">Paused</Badge>}
                {!state.paused && state.autoPaused && (
                  <Badge variant="secondary">Auto-paused</Badge>
                )}
                {!state.muted && <Badge variant="secondary">Sound on</Badge>}
              </div>
            </div>
            <div className="flex items-center gap-1">
              <IconButton label={state.paused ? "Play" : "Pause"} onClick={togglePaused}>
                {state.paused ? <Play /> : <Pause />}
              </IconButton>
              <IconButton label={state.muted ? "Unmute" : "Mute"} onClick={toggleMuted}>
                {state.muted ? <VolumeX /> : <Volume2 />}
              </IconButton>
              <DropdownMenu>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon" aria-label="Fit">
                        <Expand />
                      </Button>
                    </DropdownMenuTrigger>
                  </TooltipTrigger>
                  <TooltipContent>Fit</TooltipContent>
                </Tooltip>
                <DropdownMenuContent align="end">
                  <DropdownMenuRadioGroup value={state.fit} onValueChange={(v) => setFit(v as Fit)}>
                    {FITS.map((f) => (
                      <DropdownMenuRadioItem key={f.value} value={f.value}>
                        {f.label}
                      </DropdownMenuRadioItem>
                    ))}
                  </DropdownMenuRadioGroup>
                </DropdownMenuContent>
              </DropdownMenu>
              <IconButton label="Stop wallpaper" onClick={stop}>
                <Square />
              </IconButton>
            </div>
          </motion.div>
        ) : (
          <motion.div
            key="empty"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18 }}
            className="flex items-center gap-3 p-3 text-muted-foreground"
          >
            <div className="flex aspect-video w-36 items-center justify-center rounded-md bg-zinc-900">
              <Film className="size-5" />
            </div>
            <div>
              <p className="font-medium text-foreground">No wallpaper set</p>
              <p className="text-sm">Pick a video from your library below.</p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </Card>
  );
}
