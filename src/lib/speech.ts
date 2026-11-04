import { isTauri, native } from "./native";

/** Deepgram Aura voices offered in Settings (value = Deepgram model id). */
export const ASSISTANT_VOICES: Array<{ id: string; label: string }> = [
  { id: "aura-2-thalia-en", label: "Thalia (warm, clear)" },
  { id: "aura-2-andromeda-en", label: "Andromeda (calm)" },
  { id: "aura-2-helena-en", label: "Helena (friendly)" },
  { id: "aura-2-asteria-en", label: "Asteria (bright)" },
  { id: "aura-2-apollo-en", label: "Apollo (confident)" },
  { id: "aura-2-arcas-en", label: "Arcas (relaxed)" },
];

export function stopSpeaking() {
  if (isTauri()) void native.stopSpeaking().catch(() => undefined);
  if ("speechSynthesis" in window) window.speechSynthesis.cancel();
}

/**
 * Reads text aloud with Deepgram's voice on the output device chosen in
 * Settings › Audio. Falls back to the system voice only when Deepgram isn't
 * reachable (no key, offline, or browser-only dev mode).
 */
export async function speak(text: string): Promise<void> {
  if (!text.trim()) return;
  stopSpeaking();
  if (isTauri()) {
    try {
      await native.speakText(text);
      return;
    } catch (error) {
      console.warn("Deepgram speech unavailable; using the system voice", error);
    }
  }
  if (!("speechSynthesis" in window)) return;
  window.speechSynthesis.speak(new SpeechSynthesisUtterance(text));
}
