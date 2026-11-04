import type { AppErrorPayload, DictationState } from "../types/models";

export interface OverlayPresentation {
  label: string;
  /** Optional second line, e.g. a recovery hint. */
  detail?: string;
  tone: "listening" | "processing" | "success" | "error" | "neutral";
  showWaveform: boolean;
  showTimer: boolean;
  showFinish: boolean;
  showSpinner: boolean;
  /** Whether the live transcript should replace the label while it streams. */
  showCaption: boolean;
}

const quiet = { showWaveform: false, showTimer: false, showFinish: false, showSpinner: false, showCaption: false };
const busy = { ...quiet, tone: "processing" as const, showSpinner: true };

/**
 * Turns the backend's error category into what actually happened. The dock used
 * to say "Could not paste automatically" for *every* error — including ones
 * where the paste had worked, or where nothing was ever heard.
 */
export function errorPresentation(error?: AppErrorPayload | null): Pick<OverlayPresentation, "label" | "detail"> {
  switch (error?.category) {
    case "insertion":
      return { label: "Couldn't paste", detail: "Copied — press Ctrl+V" };
    case "empty_transcript":
      return { label: "Didn't catch that", detail: "No speech heard" };
    case "microphone":
      return { label: "No microphone", detail: "Check Settings" };
    case "credential":
      return { label: "Deepgram key needed", detail: "Add it in Settings" };
    case "transcription":
      return {
        label: "Transcription failed",
        detail: /rate limit/i.test(error.message)
          ? "Rate limited, retry soon"
          : /api key|rejected/i.test(error.message)
            ? "API key rejected"
            : "Check your connection",
      };
    case "windows":
      return { label: "Can't dictate here", detail: error.message.replace(/^Windows integration error:\s*/i, "") };
    case "state":
      return { label: "Still finishing the last one", detail: "Try again in a moment" };
    default:
      return { label: "Something went wrong", detail: error?.message };
  }
}

function wordCount(text: string | undefined): number {
  return text?.trim() ? text.trim().split(/\s+/).length : 0;
}

/**
 * Maps the dictation state machine onto what the overlay shows. Every state the
 * machine can enter is represented, so the overlay never falls back to a
 * generic "Processing" for a state with its own meaning.
 */
export function overlayPresentation(
  state: DictationState,
  targetApplication?: string | null,
  error?: AppErrorPayload | null,
  transcript?: string,
): OverlayPresentation {
  switch (state) {
    case "starting":
      return { ...busy, label: "Starting microphone…" };
    case "listening_push_to_talk":
      return { ...quiet, label: "Listening", tone: "listening", showWaveform: true, showTimer: true, showCaption: true };
    case "listening_hands_free":
      return { ...quiet, label: "Listening", tone: "listening", showWaveform: true, showTimer: true, showFinish: true, showCaption: true };
    case "finalizing_audio":
      return { ...busy, label: "Finishing transcription…", showCaption: true };
    case "transcribing":
      return { ...busy, label: "Transcribing…", showCaption: true };
    case "cleaning":
      return { ...busy, label: "Cleaning text…" };
    case "inserting":
      return { ...busy, label: targetApplication ? `Inserting into ${targetApplication}…` : "Inserting…" };
    case "completed": {
      const words = wordCount(transcript);
      return {
        ...quiet,
        label: "Inserted",
        detail: words > 0 ? `${words} ${words === 1 ? "word" : "words"}` : undefined,
        tone: "success",
      };
    }
    case "cancelled":
      return { ...quiet, label: "Dictation cancelled", tone: "neutral" };
    case "error":
      return { ...quiet, ...errorPresentation(error), tone: "error" };
    default:
      return { ...quiet, label: "Ready", tone: "neutral" };
  }
}

/**
 * Keeps the newest words of a long streaming transcript. The caption element
 * also clips from the *start* in CSS, so this only bounds the DOM text.
 */
export function captionTail(text: string, maxChars = 120): string {
  const trimmed = text.trim().replace(/\s+/g, " ");
  if (trimmed.length <= maxChars) return trimmed;
  const tail = trimmed.slice(-maxChars);
  const firstSpace = tail.indexOf(" ");
  return `…${firstSpace > 0 && firstSpace < 16 ? tail.slice(firstSpace + 1) : tail}`;
}
