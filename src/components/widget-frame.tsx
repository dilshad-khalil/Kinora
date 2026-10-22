import { useEffect, useRef, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { resolveSettings, widgetUrl, type Instance, type Widget } from "@/lib/widgets";
import { DATA_TOPICS, latestData, subscribe } from "@/lib/widget-data";

/**
 * A widget in a sandboxed iframe: scripts only, opaque origin, so it can't reach this page,
 * Tauri, or anything outside its own folder (the kwidget protocol's CSP covers the rest).
 * This page relays settings and the live data the widget has permission for, and forwards
 * its notification requests when `actions` is set (only on the desktop, not in previews).
 */
export function WidgetFrame(props: {
  widget: Widget;
  instance: Instance;
  revision: number;
  visible?: boolean;
  actions?: boolean;
  style?: CSSProperties;
}) {
  const { widget, instance, revision, visible = true, actions = false } = props;
  const frame = useRef<HTMLIFrameElement>(null);
  const settings = resolveSettings(widget, instance);
  const settingsKey = JSON.stringify(settings);
  const glass = typeof settings.glass === "boolean" ? settings.glass : widget.glass;
  const permissions = widget.permissions;
  const permissionsKey = permissions.join();

  const post = (message: object) => frame.current?.contentWindow?.postMessage(message, "*");
  const sendAll = () => {
    post({ kinora: "permissions", permissions });
    post({ kinora: "settings", settings: JSON.parse(settingsKey) });
    post({ kinora: "visible", visible });
    for (const topic of DATA_TOPICS) {
      const data = latestData(topic);
      if (permissions.includes(topic) && data !== undefined) post({ kinora: "data", topic, data });
    }
  };

  useEffect(() => post({ kinora: "settings", settings: JSON.parse(settingsKey) }), [settingsKey]);
  useEffect(() => post({ kinora: "visible", visible }), [visible]);

  // Live data, only for topics the manifest asks for.
  useEffect(() => {
    const offs = DATA_TOPICS.filter((t) => permissions.includes(t)).map((topic) =>
      subscribe(topic, (data) => post({ kinora: "data", topic, data })),
    );
    return () => offs.forEach((off) => off());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [permissionsKey]);

  // Requests from the widget. Only messages from this iframe count; Rust re-checks the permission.
  useEffect(() => {
    if (!actions) return;
    const onMessage = (e: MessageEvent) => {
      if (e.source !== frame.current?.contentWindow || e.data?.kinora !== "notify") return;
      invoke("widget_notify", {
        instance: instance.id,
        title: String(e.data.title ?? ""),
        body: String(e.data.body ?? ""),
      }).catch((err) => post({ kinora: "error", message: String(err) }));
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [actions, instance.id]);

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
