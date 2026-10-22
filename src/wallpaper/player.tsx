import { useEffect, useRef, useState, type CSSProperties } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import type { Fit, KinoraState } from "@/lib/commands";
import { useMonitorRegions, type Region } from "@/lib/regions";
import { WidgetLayer } from "./widget-layer";

/**
 * One window spans every monitor. The video is decoded once, on the first monitor;
 * the other monitors get a canvas that copies each decoded frame. A second decoder
 * costs ~90 MB of GPU-process memory, a canvas ~8 MB at 1080p.
 * With no video (widgets only), each monitor shows the static wallpaper image instead.
 */
export function Player() {
  const video = useRef<HTMLVideoElement>(null);
  const mirrors = useRef<HTMLDivElement>(null);
  const paused = useRef(false);
  const muted = useRef(true);
  const regions = useMonitorRegions();
  const [src, setSrc] = useState<string>();
  const [still, setStill] = useState<string>(); // static wallpaper, when there's no video
  const [fit, setFit] = useState<Fit>("cover");

  useEffect(() => {
    const load = (s: KinoraState) => {
      if (!s.current) {
        // Unloading needs an explicit load(); dropping the src attribute keeps it playing.
        setSrc(undefined);
        video.current?.removeAttribute("src");
        video.current?.load();
        invoke<string | null>("static_wallpaper").then((p) => setStill(p ? convertFileSrc(p) : undefined));
        return;
      }
      setStill(undefined);
      setSrc(convertFileSrc(s.current));
      setFit(s.fit);
      setMuted(s.muted);
      paused.current = s.paused || s.autoPaused;
    };

    const setMuted = (m: boolean) => {
      muted.current = m;
      if (video.current) video.current.muted = m;
    };

    const offs = [
      listen<KinoraState>("player:load", (e) => load(e.payload)),
      listen("player:pause", () => {
        paused.current = true;
        video.current?.pause();
      }),
      listen("player:play", () => {
        paused.current = false;
        video.current?.play().catch(() => {});
      }),
      listen<boolean>("player:mute", (e) => setMuted(e.payload)),
      listen<Fit>("player:fit", (e) => setFit(e.payload)),
    ];
    // Events sent before this page loaded are lost, so ask for the current state once.
    invoke<KinoraState>("get_state").then(load);

    return () => {
      offs.forEach((p) => p.then((off) => off()));
    };
  }, []);

  // Copy every decoded frame onto the other monitors' canvases.
  useEffect(() => {
    const v = video.current;
    const canvases = Array.from(mirrors.current?.querySelectorAll("canvas") ?? []);
    if (!v || canvases.length === 0) return;

    let handle = 0;
    const draw = () => {
      for (const c of canvases) {
        if (c.width !== v.videoWidth || c.height !== v.videoHeight) {
          c.width = v.videoWidth;
          c.height = v.videoHeight;
        }
        c.getContext("2d")?.drawImage(v, 0, 0);
      }
      handle = v.requestVideoFrameCallback(draw);
    };
    handle = v.requestVideoFrameCallback(draw);
    return () => v.cancelVideoFrameCallback(handle);
  }, [regions]);

  const [first, ...rest] = regions;
  const style = ({ left, top, width, height }: Region): CSSProperties => ({
    position: "fixed",
    left,
    top,
    width,
    height,
    objectFit: fit,
    background: "#000",
  });

  return (
    <>
      {first && (
        <video
          ref={(el) => {
            video.current = el;
            if (el) el.muted = muted.current; // the state may arrive before the layout
          }}
          src={src}
          autoPlay
          loop
          muted
          playsInline
          onCanPlay={(e) => {
            if (paused.current) e.currentTarget.pause();
          }}
          onError={(e) => emit("player:error", { code: e.currentTarget.error?.code })}
          style={style(first)}
        />
      )}
      <div ref={mirrors}>
        {rest.map((r, i) => (
          <canvas key={i} style={style(r)} />
        ))}
      </div>
      {!src &&
        regions.map((r, i) =>
          still ? (
            <img key={i} src={still} alt="" style={{ ...style(r), objectFit: "cover" }} />
          ) : (
            <div key={i} style={style(r)} />
          ),
        )}
      <WidgetLayer regions={regions} />
    </>
  );
}
