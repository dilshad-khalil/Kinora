import { useEffect, useRef, type CSSProperties } from "react";
import { resolveSettings, widgetUrl, type Instance, type Widget } from "@/lib/widgets";

/**
 * A widget in a sandboxed iframe: scripts only, opaque origin, so it can't reach this page,
 * Tauri, or anything outside its own folder (the kwidget protocol's CSP covers the rest).
 */
export function WidgetFrame(props: {
  widget: Widget;
  instance: Instance;
  revision: number;
  visible?: boolean;
  style?: CSSProperties;
}) {
  const { widget, instance, revision, visible = true } = props;
  const frame = useRef<HTMLIFrameElement>(null);
  const settings = resolveSettings(widget, instance);
  const settingsKey = JSON.stringify(settings);
  const glass = typeof settings.glass === "boolean" ? settings.glass : widget.glass;

  const post = (message: object) => frame.current?.contentWindow?.postMessage(message, "*");
  const sendAll = () => {
    post({ kinora: "settings", settings: JSON.parse(settingsKey) });
    post({ kinora: "visible", visible });
  };

  useEffect(() => post({ kinora: "settings", settings: JSON.parse(settingsKey) }), [settingsKey]);
  useEffect(() => post({ kinora: "visible", visible }), [visible]);

  return (
    <div
      style={{
        position: "absolute",
        overflow: "hidden",
        borderRadius: widget.radius,
        backdropFilter: glass ? "blur(28px) saturate(1.7)" : undefined,
        visibility: visible ? "visible" : "hidden",
        ...props.style,
      }}
    >
      <iframe
        ref={frame}
        key={revision}
        src={widgetUrl(widget, revision)}
        sandbox="allow-scripts"
        title={widget.name}
        tabIndex={-1}
        onLoad={sendAll}
        style={{ display: "block", width: "100%", height: "100%", border: 0, background: "transparent" }}
      />
    </div>
  );
}
