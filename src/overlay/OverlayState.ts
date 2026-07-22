import type { DictationState } from "../types/models";

export interface OverlayPresentation {
  label: string;
  /** Optional second line, e.g. the elapsed timer or a recovery hint. */
  detail?: string;
  tone: "listening" | "processing" | "success" | "error" | "neutral";
  showWaveform: boolean;
  showTimer: boolean;
  showFinish: boolean;
  showSpinner: boolean;
}

/**
 * Maps the dictation state machine onto what the overlay shows. Every state the
 * machine can enter is represented, so the overlay never falls back to a
 * generic "Processing" for a state with its own meaning.
 */
export function overlayPresentation(
  state: DictationState,
  targetApplication?: string | null,
): OverlayPresentation {
  switch (state) {
    case "starting":
      return { label: "Starting microphone…", tone: "processing", showWaveform: false, showTimer: false, showFinish: false, showSpinner: true };
    case "listening_push_to_talk":
      return { label: "Listening", tone: "listening", showWaveform: true, showTimer: true, showFinish: false, showSpinner: false };
    case "listening_hands_free":
      return { label: "Listening", tone: "listening", showWaveform: true, showTimer: true, showFinish: true, showSpinner: false };
    case "finalizing_audio":
      return { label: "Finishing transcription…", tone: "processing", showWaveform: false, showTimer: false, showFinish: false, showSpinner: true };
    case "transcribing":
      return { label: "Processing transcription", tone: "processing", showWaveform: false, showTimer: false, showFinish: false, showSpinner: true };
    case "cleaning":
      return { label: "Cleaning text…", tone: "processing", showWaveform: false, showTimer: false, showFinish: false, showSpinner: true };
    case "inserting":
      return {
        label: targetApplication ? `Inserting into ${targetApplication}…` : "Inserting…",
        tone: "processing",
        showWaveform: false,
        showTimer: false,
        showFinish: false,
        showSpinner: true,
      };
    case "completed":
      return { label: "Inserted", tone: "success", showWaveform: false, showTimer: false, showFinish: false, showSpinner: false };
    case "cancelled":
      return { label: "Dictation cancelled", tone: "neutral", showWaveform: false, showTimer: false, showFinish: false, showSpinner: false };
    case "error":
      return {
        label: "Could not paste automatically",
        detail: "Text copied to clipboard",
        tone: "error",
        showWaveform: false,
        showTimer: false,
        showFinish: false,
        showSpinner: false,
      };
    default:
      return { label: "Ready", tone: "neutral", showWaveform: false, showTimer: false, showFinish: false, showSpinner: false };
  }
}
