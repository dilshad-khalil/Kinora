import { useEffect, useRef, useState } from "react";
import { RotateCcw } from "lucide-react";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { cn } from "@/lib/utils";
import { Permissions } from "@/components/widget-permissions";
import {
  resolveSettings,
  widgetCommands,
  type Instance,
  type SettingDef,
  type SettingValue,
  type Widget,
} from "@/lib/widgets";

const inputClass =
  "h-8 w-full rounded-md border border-input bg-transparent px-2.5 text-sm outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/50";

function Field(props: { def: SettingDef; value: SettingValue | undefined; onChange: (v: SettingValue) => void }) {
  const { def, value, onChange } = props;
  const id = `setting-${def.key}`;
  const label = def.label ?? def.key;

  if (def.type === "toggle") {
    return (
      <div className="flex items-center justify-between gap-4">
        <Label htmlFor={id}>{label}</Label>
        <Switch id={id} checked={Boolean(value)} onCheckedChange={onChange} />
      </div>
    );
  }

  let control;
  if (def.type === "select") {
    control = (
      <div className="flex flex-wrap gap-1 rounded-md bg-zinc-900 p-0.5">
        {(def.options ?? []).map((o) => (
          <button
            key={o}
            onClick={() => onChange(o)}
            className={cn(
              "flex-1 rounded px-2.5 py-1 text-sm capitalize text-muted-foreground transition-colors hover:text-foreground",
              value === o && "bg-zinc-700 text-foreground",
            )}
          >
            {o}
          </button>
        ))}
      </div>
    );
  } else if (def.type === "color") {
    control = (
      <div className="flex items-center gap-2">
        <input
          id={id}
          type="color"
          value={String(value ?? "#ffffff")}
          onChange={(e) => onChange(e.target.value)}
          className="h-8 w-10 cursor-pointer rounded-md border border-input bg-transparent p-0.5"
        />
        <span className="font-mono text-xs text-muted-foreground uppercase">{String(value ?? "")}</span>
      </div>
    );
  } else if (def.type === "number" && def.min !== undefined && def.max !== undefined) {
    const n = Number(value ?? def.min);
    control = (
      <div className="flex items-center gap-3">
        <Slider
          id={id}
          min={def.min}
          max={def.max}
          step={def.step ?? 1}
          value={[n]}
          onValueChange={([v]) => onChange(v)}
        />
        <span className="w-10 text-right text-sm tabular-nums">{n}</span>
      </div>
    );
  } else if (def.type === "number") {
    control = (
      <input
        id={id}
        type="number"
        step={def.step}
        value={Number(value ?? 0)}
        onChange={(e) => onChange(Number(e.target.value))}
        className={inputClass}
      />
    );
  } else {
    control = (
      <input
        id={id}
        type="text"
        value={String(value ?? "")}
        placeholder={def.placeholder}
        onChange={(e) => onChange(e.target.value)}
        className={inputClass}
        spellCheck={false}
      />
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      {control}
    </div>
  );
}

/** Settings form generated from the widget's manifest. Changes show on the desktop live. */
export function WidgetSettings(props: {
  widget: Widget | undefined;
  instance: Instance | undefined;
  onOpenChange: (open: boolean) => void;
}) {
  const { widget, instance } = props;
  const [values, setValues] = useState<Record<string, SettingValue>>({});
  const save = useRef<number>(0);

  useEffect(() => {
    if (widget && instance) setValues(resolveSettings(widget, instance));
    // Only when another instance is opened; our own saves would fight the user's typing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [instance?.id, widget?.id]);

  const change = (patch: Record<string, SettingValue>) => {
    if (!instance) return;
    const next = { ...values, ...patch };
    setValues(next);
    // Debounced: typing or dragging a slider shouldn't write the store on every step.
    clearTimeout(save.current);
    save.current = window.setTimeout(() => widgetCommands.update({ ...instance, settings: next }), 150);
  };

  const reset = () => {
    if (!widget || !instance) return;
    clearTimeout(save.current);
    const defaults = resolveSettings(widget, { ...instance, settings: {} });
    setValues(defaults);
    widgetCommands.update({ ...instance, settings: {} });
  };

  return (
    <Sheet open={!!instance} onOpenChange={props.onOpenChange}>
      <SheetContent className="border-zinc-800">
        <SheetHeader>
          <SheetTitle>{widget?.name ?? "Widget"}</SheetTitle>
          <SheetDescription>{widget?.description || "Changes show on your desktop right away."}</SheetDescription>
          {widget && <Permissions widget={widget} className="mt-1" />}
        </SheetHeader>
        <div className="flex flex-col gap-5 overflow-y-auto px-4 pb-4">
          {widget?.settings.length ? (
            widget.settings.map((def) => (
              <Field key={def.key} def={def} value={values[def.key]} onChange={(v) => change({ [def.key]: v })} />
            ))
          ) : (
            <p className="text-sm text-muted-foreground">This widget has no settings.</p>
          )}
        </div>
        {!!widget?.settings.length && (
          <div className="mt-auto border-t border-zinc-800 p-4">
            <Button variant="ghost" size="sm" onClick={reset}>
              <RotateCcw /> Reset to defaults
            </Button>
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}
