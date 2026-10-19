import { useEffect, useState } from "react";
import { Settings2 } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useKinora } from "@/store/kinora";

function Row(props: {
  id: string;
  label: string;
  hint?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <Label htmlFor={props.id} className="flex flex-col items-start gap-0.5">
        <span>{props.label}</span>
        {props.hint && <span className="text-xs font-normal text-muted-foreground">{props.hint}</span>}
      </Label>
      <Switch
        id={props.id}
        checked={props.checked}
        disabled={props.disabled}
        onCheckedChange={props.onChange}
      />
    </div>
  );
}

export function SettingsSheet() {
  const { settings, updateSettings, launchAtLogin, setLaunchAtLogin } = useKinora();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  return (
    <Sheet>
      <Tooltip>
        <TooltipTrigger asChild>
          <SheetTrigger asChild>
            <Button variant="ghost" size="icon" aria-label="Settings">
              <Settings2 />
            </Button>
          </SheetTrigger>
        </TooltipTrigger>
        <TooltipContent>Settings</TooltipContent>
      </Tooltip>
      <SheetContent className="border-zinc-800">
        <SheetHeader>
          <SheetTitle>Settings</SheetTitle>
          <SheetDescription>Kinora keeps running in the tray when you close this window.</SheetDescription>
        </SheetHeader>

        <div className="flex flex-col gap-5 px-4">
          <Row
            id="autostart"
            label="Launch at login"
            hint={
              // A dev build loads its UI from the Vite server, which isn't running at login.
              import.meta.env.DEV ? "Only in the installed app" : "Restores your last wallpaper"
            }
            checked={launchAtLogin}
            disabled={import.meta.env.DEV}
            onChange={setLaunchAtLogin}
          />
          <Row
            id="fullscreen"
            label="Pause when an app is fullscreen"
            hint="Games, videos, presentations"
            checked={settings.pauseOnFullscreen}
            onChange={(v) => updateSettings({ pauseOnFullscreen: v })}
          />
          <Row
            id="battery"
            label="Pause on battery"
            checked={settings.pauseOnBattery}
            onChange={(v) => updateSettings({ pauseOnBattery: v })}
          />
          <Row
            id="muted"
            label="Start muted"
            hint="Applies the next time Kinora starts"
            checked={settings.startMuted}
            onChange={(v) => updateSettings({ startMuted: v })}
          />
        </div>

        <div className="mt-auto border-t border-zinc-800 p-4 text-xs text-muted-foreground">
          <p className="font-medium text-foreground">Kinora {version}</p>
          <p>Video wallpapers for Windows. Named after the 1890s Kinora flip-card viewer.</p>
        </div>
      </SheetContent>
    </Sheet>
  );
}
