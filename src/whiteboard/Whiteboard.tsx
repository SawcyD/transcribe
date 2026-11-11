import { convertToExcalidrawElements, Excalidraw } from "@excalidraw/excalidraw";
import "@excalidraw/excalidraw/index.css";
import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";
import { parseMermaidToExcalidraw } from "@excalidraw/mermaid-to-excalidraw";
import { emit } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { events, isTauri } from "../lib/native";

const STORAGE_KEY = "voiceflow.whiteboard";

// The handwriting fonts ship with the app (public/excalidraw/fonts). Left
// alone, Excalidraw fetches them from a CDN, which the app's CSP blocks.
(window as unknown as { EXCALIDRAW_ASSET_PATH: string }).EXCALIDRAW_ASSET_PATH = "/excalidraw/";

function savedElements() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
  } catch {
    return [];
  }
}

/** Turns Mermaid into hand-drawn shapes and places them right of what's already there. */
async function draw(api: ExcalidrawImperativeAPI, mermaid: string) {
  const { elements: skeleton, files } = await parseMermaidToExcalidraw(mermaid);
  const fresh = convertToExcalidrawElements(skeleton);
  if (fresh.length === 0) throw new Error("the diagram was empty");
  const existing = api.getSceneElements();
  const shiftX = (existing.length ? Math.max(...existing.map((element) => element.x + element.width)) + 120 : 0) - Math.min(...fresh.map((element) => element.x));
  const shiftY = (existing.length ? Math.min(...existing.map((element) => element.y)) : 0) - Math.min(...fresh.map((element) => element.y));
  const placed = fresh.map((element) => ({ ...element, x: element.x + shiftX, y: element.y + shiftY })) as typeof fresh;
  if (files) api.addFiles(Object.values(files));
  api.updateScene({ elements: [...existing, ...placed] });
  api.scrollToContent(placed, { fitToContent: true, animate: true });
}

/**
 * A shared Excalidraw board: you draw by hand, Flow draws by sending Mermaid
 * (see `draw_on_whiteboard` in voice_agent.rs). The board is kept between
 * sessions in this window's local storage.
 */
export function Whiteboard() {
  const api = useRef<ExcalidrawImperativeAPI | null>(null);
  const saveTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    const subscriptions = Promise.all([
      events.whiteboardDraw(async (mermaid) => {
        let error: string | null = null;
        try {
          if (!api.current) throw new Error("the whiteboard is still loading");
          await draw(api.current, mermaid);
        } catch (cause) {
          error = cause instanceof Error ? cause.message : String(cause);
        }
        // Tells Flow whether the diagram landed, so it can fix bad Mermaid.
        if (isTauri()) void emit("whiteboard-result", error);
      }),
      events.whiteboardClear(() => api.current?.resetScene()),
    ]);
    return () => void subscriptions.then((items) => items.forEach((unlisten) => unlisten()));
  }, []);

  return (
    <div className="whiteboard">
      <Excalidraw
        excalidrawAPI={(instance) => {
          api.current = instance;
        }}
        initialData={{ elements: savedElements(), scrollToContent: true }}
        theme={window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark"}
        onChange={(elements) => {
          // ponytail: saves shapes and text only; pasted images are not kept across restarts.
          window.clearTimeout(saveTimer.current);
          saveTimer.current = window.setTimeout(() => {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(elements.filter((element) => !element.isDeleted)));
          }, 500);
        }}
      />
    </div>
  );
}
