import { ArrowUp, ImagePlus, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { native } from "../lib/native";

const MAX_IMAGES = 4;
/** Long side of pasted images. Enough to read code, small enough to send quickly. */
const MAX_SIDE = 1600;

/** Re-encodes an image as a JPEG data URL no larger than MAX_SIDE. */
async function shrink(file: Blob): Promise<string> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, MAX_SIDE / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  const context = canvas.getContext("2d");
  if (!context) throw new Error("couldn't read the image");
  // JPEG has no transparency; paint white behind it so it isn't black.
  context.fillStyle = "#fff";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  return canvas.toDataURL("image/jpeg", 0.85);
}

/**
 * Type or paste to Flow during a voice conversation: text, and images it
 * gets described. Takes the caption pill's place while open.
 */
export function AgentComposer({ onClose }: { onClose: () => void }) {
  const [draft, setDraft] = useState("");
  const [images, setImages] = useState<string[]>([]);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const field = useRef<HTMLTextAreaElement>(null);
  const picker = useRef<HTMLInputElement>(null);

  useEffect(() => field.current?.focus(), []);

  // Grow with the text, up to four lines, then scroll.
  useEffect(() => {
    const element = field.current;
    if (!element) return;
    element.style.height = "auto";
    element.style.height = `${Math.min(element.scrollHeight, 76)}px`;
  }, [draft]);

  const addImages = async (files: Blob[]) => {
    const room = MAX_IMAGES - images.length;
    if (files.length > room) setError(`Up to ${MAX_IMAGES} images at a time.`);
    try {
      const added = await Promise.all(files.slice(0, Math.max(0, room)).map(shrink));
      setImages((current) => [...current, ...added].slice(0, MAX_IMAGES));
    } catch {
      setError("That image couldn't be read.");
    }
  };

  const send = async () => {
    if (sending || (!draft.trim() && images.length === 0)) return;
    setSending(true);
    setError(null);
    try {
      await native.agentSendMessage(draft, images);
      onClose();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setSending(false);
    }
  };

  const canSend = !sending && (draft.trim().length > 0 || images.length > 0);

  return (
    <form
      className="agent-composer"
      onSubmit={(event) => {
        event.preventDefault();
        void send();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
      }}
    >
      {(images.length > 0 || error) && (
        <div className="agent-composer__extras">
          {images.map((image, index) => (
            <span key={index} className="agent-composer__thumb">
              <img src={image} alt={`Image ${index + 1}`} />
              <button type="button" aria-label={`Remove image ${index + 1}`} onClick={() => setImages((current) => current.filter((_, at) => at !== index))}>
                <X size={11} />
              </button>
            </span>
          ))}
          {error && <span className="agent-composer__error">{error}</span>}
        </div>
      )}
      <div className="agent-composer__row">
        <button type="button" className="agent-composer__icon" aria-label="Attach images" title="Attach images (or paste them)" disabled={sending || images.length >= MAX_IMAGES} onClick={() => picker.current?.click()}>
          <ImagePlus size={15} />
        </button>
        <input
          ref={picker}
          type="file"
          accept="image/*"
          multiple
          hidden
          onChange={(event) => {
            void addImages(Array.from(event.target.files ?? []));
            event.target.value = "";
          }}
        />
        <textarea
          ref={field}
          rows={1}
          value={draft}
          disabled={sending}
          aria-label="Message to Flow"
          placeholder={sending ? (images.length ? "Looking at your images…" : "Sending…") : "Type or paste to Flow…"}
          onChange={(event) => setDraft(event.target.value)}
          onPaste={(event) => {
            const pasted = Array.from(event.clipboardData.files).filter((file) => file.type.startsWith("image/"));
            if (pasted.length) {
              event.preventDefault();
              void addImages(pasted);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void send();
            }
          }}
        />
        <button type="submit" className="agent-composer__send" aria-label="Send to Flow" title="Send · Enter" disabled={!canSend}>
          <ArrowUp size={15} />
        </button>
      </div>
    </form>
  );
}
