import { Check, Copy, X } from "lucide-react";
import { useAppStore } from "../app/useAppStore";
import { native } from "../lib/native";
import { AudioVisualizer } from "./AudioVisualizer";
import { overlayPresentation } from "./OverlayState";

export function RecordingOverlay() {
  const { dictation, audio, lastTranscript } = useAppStore();
  const presentation = overlayPresentation(dictation.state);
  const processing = presentation.tone === "processing";
  const handsFree = presentation.showFinish;

  return (
    <div className={`recording-pill recording-pill--${presentation.tone} ${handsFree ? "recording-pill--hands-free" : ""}`} data-tauri-drag-region role="status" aria-live="polite">
      <button className="pill-action pill-action--cancel" aria-label="Cancel dictation" onClick={() => void native.cancel()}>
        <X size={15} />
      </button>
      {presentation.tone === "success" ? (
        <div className="pill-message"><Check size={16} /><span>{presentation.label}</span></div>
      ) : presentation.tone === "error" ? (
        <div className="pill-message"><span>Paste failed</span></div>
      ) : (
        <div className="pill-center">
          <AudioVisualizer bars={audio?.bars ?? []} processing={processing} />
          {processing && <span className="pill-state-label">{presentation.label}</span>}
        </div>
      )}
      {presentation.tone === "error" ? (
        <button className="pill-action pill-action--copy" aria-label="Copy recovered transcript" onClick={() => lastTranscript && void native.copyText(lastTranscript.finalTranscript)}>
          <Copy size={15} />
        </button>
      ) : presentation.showFinish ? (
        <button className="pill-action pill-action--finish" aria-label="Finish dictation" onClick={() => void native.finish()}>
          <Check size={15} />
        </button>
      ) : processing ? (
        <span className="pill-spinner" aria-label={presentation.label} />
      ) : presentation.tone === "success" ? <span className="pill-spacer" /> : <span className="pill-end-cap" aria-hidden="true" />}
    </div>
  );
}
