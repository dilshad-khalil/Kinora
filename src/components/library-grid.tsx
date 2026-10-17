import { Plus } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { VideoCard } from "@/components/video-card";
import { useKinora, VIDEO_EXTENSIONS } from "@/store/kinora";

export async function pickVideos() {
  const picked = await open({
    multiple: true,
    filters: [{ name: "Videos", extensions: VIDEO_EXTENSIONS }],
  });
  if (picked) await useKinora.getState().addFiles(picked);
}

export function LibraryGrid() {
  const library = useKinora((s) => s.library);

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-medium text-muted-foreground">Library</h2>
        <Button size="sm" variant="secondary" onClick={pickVideos}>
          <Plus /> Add video
        </Button>
      </div>

      {library.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center rounded-lg border border-dashed border-zinc-800 text-center text-sm text-muted-foreground">
          <p className="font-medium text-foreground">Your library is empty</p>
          <p>Add an MP4 or WebM video, or drop files onto this window.</p>
        </div>
      ) : (
        <div className="-mx-1 grid grid-cols-3 gap-4 overflow-y-auto px-1 pt-1 pb-4">
          {library.map((v) => (
            <VideoCard key={v.id} video={v} />
          ))}
        </div>
      )}
    </section>
  );
}
