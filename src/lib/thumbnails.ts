import { convertFileSrc } from "@tauri-apps/api/core";

export const UNSUPPORTED = "This format isn't supported. Try MP4 (H.264) or WebM.";

const W = 320;
const H = 180;

/** Grab a frame ~10% in (max 1s) as a 320x180 webp data URL. No ffmpeg needed. */
export function captureThumbnail(path: string): Promise<{ thumb: string; durationSec: number }> {
  return new Promise((resolve, reject) => {
    const video = document.createElement("video");
    video.crossOrigin = "anonymous";
    video.muted = true;
    video.preload = "metadata";

    const done = () => {
      video.removeAttribute("src");
      video.load();
    };

    video.onloadedmetadata = () => {
      video.currentTime = Math.min(1, video.duration * 0.1);
    };
    video.onseeked = () => {
      const canvas = document.createElement("canvas");
      canvas.width = W;
      canvas.height = H;
      const scale = Math.max(W / video.videoWidth, H / video.videoHeight);
      const w = video.videoWidth * scale;
      const h = video.videoHeight * scale;
      canvas.getContext("2d")!.drawImage(video, (W - w) / 2, (H - h) / 2, w, h);
      const durationSec = video.duration;
      done();
      resolve({ thumb: canvas.toDataURL("image/webp", 0.7), durationSec });
    };
    video.onerror = () => {
      done();
      reject(new Error(UNSUPPORTED));
    };

    video.src = convertFileSrc(path);
  });
}
