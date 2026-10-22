import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { PERMISSION_LABELS } from "@/lib/widget-data";
import type { Widget } from "@/lib/widgets";

/** What the widget can access, so users know before adding it. */
export function Permissions({ widget, className }: { widget: Widget; className?: string }) {
  if (!widget.permissions.length) return null;
  return (
    <div className={cn("flex flex-wrap gap-1", className)}>
      {widget.permissions.map((p) => (
        <Badge key={p} variant="outline" className="border-zinc-700 text-[10px] font-normal text-muted-foreground">
          {PERMISSION_LABELS[p] ?? p}
        </Badge>
      ))}
    </div>
  );
}
