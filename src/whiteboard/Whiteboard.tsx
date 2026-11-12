import { convertToExcalidrawElements, Excalidraw } from "@excalidraw/excalidraw";
import "@excalidraw/excalidraw/index.css";
import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";
import { parseMermaidToExcalidraw } from "@excalidraw/mermaid-to-excalidraw";
import { emit } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { events, isTauri, native } from "../lib/native";

const LAST_BOARD_KEY = "voiceflow.whiteboard.last";
/** Where the single unnamed board lived before boards were saved as files. */
const LEGACY_KEY = "voiceflow.whiteboard";
/** Must match `whiteboard_path` in commands.rs. */
const VALID_NAME = /^[\p{L}\p{N} _-]{1,60}$/u;

// The handwriting fonts ship with the app (public/excalidraw/fonts). Left
// alone, Excalidraw fetches them from a CDN, which the app's CSP blocks.
(window as unknown as { EXCALIDRAW_ASSET_PATH: string }).EXCALIDRAW_ASSET_PATH = "/excalidraw/";

type Elements = ReturnType<typeof convertToExcalidrawElements>;
type Skeletons = Parameters<typeof convertToExcalidrawElements>[0];

/** The whole reveal takes at most this long, however many shapes there are. */
const REVEAL_MS = 2_400;
const sleep = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));

/**
 * Adds elements, optionally moved to the free space right of what's there,
 * revealing them a shape at a time (each with its label) so the drawing
 * builds up while Flow talks instead of popping in all at once. Returns how many.
 */
async function place(api: ExcalidrawImperativeAPI, fresh: Elements, intoFreeSpace: boolean): Promise<number> {
  if (fresh.length === 0) throw new Error("there was nothing to draw");
  const existing = api.getSceneElements();
  let placed = fresh;
  if (intoFreeSpace && existing.length > 0) {
    const shiftX = Math.max(...existing.map((element) => element.x + element.width)) + 120 - Math.min(...fresh.map((element) => element.x));
    const shiftY = Math.min(...existing.map((element) => element.y)) - Math.min(...fresh.map((element) => element.y));
    placed = fresh.map((element) => ({ ...element, x: element.x + shiftX, y: element.y + shiftY })) as Elements;
  }
  // Frame the finished drawing first, so the view doesn't jump as it grows.
  api.scrollToContent(placed, { fitToContent: true, animate: true });
  // Arrows and lines last, once the shapes they connect are there.
  const isConnector = (element: Elements[number]) => element.type === "arrow" || element.type === "line";
  const labelsOf = (id: string) => placed.filter((element) => element.type === "text" && element.containerId === id);
  const groups = [...placed.filter((element) => !isConnector(element)), ...placed.filter(isConnector)]
    .filter((element) => !(element.type === "text" && element.containerId))
    .map((element) => [element, ...labelsOf(element.id)]);
  const step = Math.min(160, REVEAL_MS / groups.length);
  for (const group of groups) {
    // Read the scene each time, so the user's own edits meanwhile are kept.
    api.updateScene({ elements: [...api.getSceneElements(), ...group] });
    await sleep(step);
  }
  return placed.length;
}

/** Puts an image (fetched by the app) on the board, at most 420 px wide. */
async function addImage(api: ExcalidrawImperativeAPI, args: { dataURL: string; mimeType: string; link?: string; x?: number; y?: number; width?: number }): Promise<string> {
  const size = await new Promise<{ width: number; height: number }>((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve({ width: image.naturalWidth, height: image.naturalHeight });
    image.onerror = () => reject(new Error("the image could not be decoded"));
    image.src = args.dataURL;
  });
  const width = Math.min(Number(args.width) || 420, size.width, 800);
  const fileId = crypto.randomUUID();
  api.addFiles([{ id: fileId as never, dataURL: args.dataURL as never, mimeType: args.mimeType as never, created: Date.now() }]);
  const [element] = convertToExcalidrawElements(
    [{ type: "image", fileId: fileId as never, x: Number(args.x) || 0, y: Number(args.y) || 0, width, height: Math.round((width * size.height) / size.width), ...(args.link ? { link: args.link } : {}) }] as Skeletons,
    { regenerateIds: false },
  );
  await place(api, [{ ...element, status: "saved" }] as Elements, args.x === undefined);
  return `Added the image${args.link ? `, linked to ${args.link}` : ""}.`;
}

/** What Flow gets back from `read_whiteboard`: the board as a compact list. */
function describe(api: ExcalidrawImperativeAPI): string {
  const elements = api.getSceneElements();
  if (elements.length === 0) return "The whiteboard is empty.";
  const items = elements.slice(0, 200).map((element) => ({
    id: element.id,
    type: element.type,
    x: Math.round(element.x),
    y: Math.round(element.y),
    width: Math.round(element.width),
    height: Math.round(element.height),
    ...(element.type === "text" ? { text: element.text, ...(element.containerId ? { inside: element.containerId } : {}) } : {}),
    ...(element.backgroundColor !== "transparent" ? { backgroundColor: element.backgroundColor } : {}),
    ...(element.link ? { link: element.link } : {}),
  }));
  return `${JSON.stringify(items)}\nFreehand drawings and images are listed but their contents can't be read here.`;
}

/** Carries out one request from Flow (see `ask_whiteboard` in commands.rs). */
async function handle(api: ExcalidrawImperativeAPI, action: string, rawArguments: string): Promise<string> {
  const args = rawArguments ? JSON.parse(rawArguments) : {};
  switch (action) {
    case "mermaid": {
      const { elements: skeleton, files } = await parseMermaidToExcalidraw(String(args.mermaid ?? ""));
      if (files) api.addFiles(Object.values(files));
      return `Drew a diagram with ${await place(api, convertToExcalidrawElements(skeleton), true)} elements.`;
    }
    case "add": {
      if (!Array.isArray(args.elements) || args.elements.length === 0) throw new Error("elements must be a non-empty array");
      // Flow's own ids are kept so it can connect arrows and erase things later.
      const fresh = convertToExcalidrawElements(args.elements as Skeletons, { regenerateIds: false });
      return `Added ${await place(api, fresh, args.position !== "as_given")} elements.`;
    }
    case "image":
      return addImage(api, args);
    case "read":
      return describe(api);
    case "erase": {
      const ids = new Set<string>(Array.isArray(args.ids) ? args.ids : []);
      const all = api.getSceneElements();
      // A shape's label is its own element; it goes with the shape.
      const kept = all.filter((element) => !ids.has(element.id) && !(element.type === "text" && element.containerId && ids.has(element.containerId)));
      api.updateScene({ elements: kept });
      return `Erased ${all.length - kept.length} elements.`;
    }
    case "clear":
      api.resetScene();
      return "Cleared the whiteboard.";
    default:
      throw new Error(`unknown whiteboard action "${action}"`);
  }
}

/**
 * A shared Excalidraw board: you draw by hand, Flow draws through the tools in
 * voice_agent.rs. Boards are saved by name as files (with their pasted
 * images) and reopen where you left off.
 */
export function Whiteboard() {
  const [api, setApi] = useState<ExcalidrawImperativeAPI | null>(null);
  const [boards, setBoards] = useState<string[]>([]);
  const [board, setBoard] = useState(() => localStorage.getItem(LAST_BOARD_KEY) ?? "Untitled");
  // The name being typed for a new board, or null when not naming one.
  const [naming, setNaming] = useState<string | null>(null);
  // False while a board is loading, so the empty canvas isn't saved over it.
  const loaded = useRef(false);
  const saveTimer = useRef<number | undefined>(undefined);

  const save = useCallback(
    async (name: string) => {
      window.clearTimeout(saveTimer.current);
      if (!api || !loaded.current) return;
      const elements = api.getSceneElements();
      const used = new Set(elements.flatMap((element) => (element.type === "image" && element.fileId ? [element.fileId as string] : [])));
      const files = Object.fromEntries(Object.entries(api.getFiles()).filter(([id]) => used.has(id)));
      await native.saveWhiteboard(name, JSON.stringify({ elements, files })).catch(() => undefined);
    },
    [api],
  );

  // Open the chosen board.
  useEffect(() => {
    if (!api) return;
    let cancelled = false;
    loaded.current = false;
    void (async () => {
      const [names, content] = await Promise.all([native.listWhiteboards(), native.loadWhiteboard(board)]);
      if (cancelled) return;
      let scene: { elements?: Elements; files?: Record<string, never> } = {};
      try {
        scene = content ? JSON.parse(content) : { elements: JSON.parse(localStorage.getItem(LEGACY_KEY) ?? "[]") };
      } catch {
        // An unreadable file opens as an empty board instead of breaking the window.
      }
      api.resetScene();
      if (scene.files) api.addFiles(Object.values(scene.files));
      api.updateScene({ elements: scene.elements ?? [] });
      api.scrollToContent(undefined, { fitToContent: true });
      setBoards(names.includes(board) ? names : [...names, board].sort());
      localStorage.setItem(LAST_BOARD_KEY, board);
      loaded.current = true;
    })();
    return () => {
      cancelled = true;
    };
  }, [api, board]);

  // Flow's requests.
  useEffect(() => {
    if (!api) return;
    const subscription = events.whiteboardRequest(async ({ id, action, arguments: rawArguments }) => {
      let text: string;
      try {
        text = await handle(api, action, rawArguments);
      } catch (cause) {
        text = `That could not be drawn (${cause instanceof Error ? cause.message : String(cause)}). Fix it and try again.`;
      }
      if (isTauri()) void emit("whiteboard-result", { id, text });
    });
    return () => void subscription.then((unlisten) => unlisten());
  }, [api]);

  const switchTo = async (name: string) => {
    await save(board);
    setBoard(name);
  };

  return (
    <div className="whiteboard">
      <Excalidraw
        excalidrawAPI={setApi}
        theme={window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark"}
        onChange={() => {
          window.clearTimeout(saveTimer.current);
          saveTimer.current = window.setTimeout(() => void save(board), 800);
        }}
        renderTopRightUI={() => (
          <div className="whiteboard-boards">
            {naming === null ? (
              <>
                <select aria-label="Whiteboard" title="Saved whiteboards" value={board} onChange={(event) => void switchTo(event.target.value)}>
                  {boards.map((name) => (
                    <option key={name}>{name}</option>
                  ))}
                </select>
                <button type="button" title="Start a new whiteboard" onClick={() => setNaming("")}>
                  New
                </button>
              </>
            ) : (
              <input
                autoFocus
                aria-label="New whiteboard name"
                placeholder="Name it, then press Enter"
                value={naming}
                onChange={(event) => setNaming(event.target.value)}
                onBlur={() => setNaming(null)}
                onKeyDown={(event) => {
                  if (event.key === "Escape") setNaming(null);
                  if (event.key === "Enter" && VALID_NAME.test(naming.trim())) {
                    void switchTo(naming.trim());
                    setNaming(null);
                  }
                }}
              />
            )}
          </div>
        )}
      />
    </div>
  );
}
