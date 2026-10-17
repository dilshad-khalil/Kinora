import { useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import type { Fit, KinoraState } from "@/lib/commands";

export function Player() {
  const ref = useRef<HTMLVideoElement>(null);
  const paused = useRef(false);
  const [src, setSrc] = useState<string>();
  const [fit, setFit] = useState<Fit>("cover");

  useEffect(() => {
    const load = (s: KinoraState) => {
      if (!s.current) return;
      setSrc(convertFileSrc(s.current));
      setFit(s.fit);
      paused.current = s.paused || s.autoPaused;
      if (ref.current) ref.current.muted = s.muted;
    };

    const offs = [
      listen<KinoraState>("player:load", (e) => load(e.payload)),
      listen("player:pause", () => {
        paused.current = true;
        ref.current?.pause();
      }),
      listen("player:play", () => {
        paused.current = false;
        ref.current?.play().catch(() => {});
      }),
      listen<boolean>("player:mute", (e) => {
        if (ref.current) ref.current.muted = e.payload;
      }),
      listen<Fit>("player:fit", (e) => setFit(e.payload)),
    ];
    // Events sent before this page loaded are lost, so ask for the current state once.
    invoke<KinoraState>("get_state").then(load);

    return () => {
      offs.forEach((p) => p.then((off) => off()));
    };
  }, []);

  return (
    <video
      ref={ref}
      src={src}
      autoPlay
      loop
      muted
      playsInline
      onCanPlay={() => {
        if (paused.current) ref.current?.pause();
      }}
      onError={() => emit("player:error", { code: ref.current?.error?.code })}
      style={{
        position: "fixed",
        inset: 0,
        width: "100%",
        height: "100%",
        objectFit: fit,
        background: "#000",
      }}
    />
  );
}
