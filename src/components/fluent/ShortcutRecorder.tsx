import { useEffect, useRef, useState } from "react";
import type { ShortcutBinding } from "../../types/models";

/** Maps a KeyboardEvent.code/key onto the key names the Rust side understands. */
function keyName(event: KeyboardEvent): string | null {
  const { key, code } = event;
  if (["Control", "Alt", "Shift", "Meta", "OS"].includes(key)) return null;
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (/^F([1-9]|1[0-2])$/.test(key)) return key;
  const named: Record<string, string> = {
    " ": "Space",
    Escape: "Escape",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
  };
  return named[key] ?? null;
}

function modifiersOf(event: KeyboardEvent): string[] {
  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push("ctrl");
  if (event.altKey) modifiers.push("alt");
  if (event.shiftKey) modifiers.push("shift");
  if (event.metaKey) modifiers.push("win");
  return modifiers;
}

function formatBinding(binding: ShortcutBinding): string {
  const order: Array<[string, string]> = [
    ["ctrl", "Ctrl"],
    ["alt", "Alt"],
    ["shift", "Shift"],
    ["win", "Win"],
  ];
  const parts = order.filter(([id]) => binding.modifiers.includes(id)).map(([, label]) => label);
  if (binding.key) parts.push(binding.key);
  return parts.join(" + ") || "Not set";
}

interface ShortcutRecorderProps {
  value: ShortcutBinding;
  onChange: (binding: ShortcutBinding) => void;
  onReset: () => void;
  label: string;
}

/**
 * Captures a shortcut by listening to real key events while armed.
 *
 * Modifier-only gestures are supported: releasing the modifiers without ever
 * pressing a main key commits the modifier combination on its own, which is how
 * VoiceFlow's default "hold Ctrl + Win" push-to-talk is expressed.
 */
export function ShortcutRecorder({ value, onChange, onReset, label }: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState<ShortcutBinding | null>(null);
  const committed = useRef(false);

  useEffect(() => {
    if (!recording) return;

    const stop = () => {
      setRecording(false);
      setPreview(null);
    };

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape" && !event.ctrlKey && !event.altKey && !event.metaKey) {
        stop();
        return;
      }
      const modifiers = modifiersOf(event);
      const key = keyName(event);
      setPreview({ modifiers, key });
      if (key) {
        // A main key completes the gesture immediately.
        committed.current = true;
        onChange({ modifiers, key });
        stop();
      }
    };

    const onKeyUp = (event: KeyboardEvent) => {
      event.preventDefault();
      if (committed.current) {
        committed.current = false;
        return;
      }
      // All modifiers released with no main key: commit the modifier-only gesture.
      const stillHeld = modifiersOf(event);
      if (stillHeld.length === 0 && preview && preview.modifiers.length > 0) {
        onChange({ modifiers: preview.modifiers, key: null });
        stop();
      }
    };

    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
    };
  }, [recording, preview, onChange]);

  const shown = recording ? preview ?? { modifiers: [], key: null } : value;

  return (
    <span className="shortcut-recorder">
      <button
        type="button"
        className={`shortcut-recorder__field${recording ? " shortcut-recorder__field--recording" : ""}`}
        aria-label={`${label}. Current shortcut ${formatBinding(value)}. Activate to change.`}
        onClick={() => setRecording((current) => !current)}
      >
        {recording ? (
          <span className="shortcut-recorder__hint">
            {shown.modifiers.length > 0 ? formatBinding(shown) : "Press keys…"}
          </span>
        ) : (
          <span className="shortcut-chip">
            {formatBinding(value)
              .split(" + ")
              .map((part) => (
                <kbd key={part}>{part}</kbd>
              ))}
          </span>
        )}
      </button>
      <button type="button" className="link-button" onClick={() => setRecording((current) => !current)}>
        {recording ? "Cancel" : "Change"}
      </button>
      <button type="button" className="link-button" onClick={onReset}>
        Reset
      </button>
    </span>
  );
}
