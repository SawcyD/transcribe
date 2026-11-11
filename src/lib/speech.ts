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

/** Deepgram Flux voices for the live voice agent (value = Deepgram model id). */
export const VOICE_AGENT_VOICES: Array<{ id: string; label: string }> = [
  { id: "flux-hannah-en", label: "Hannah (American feminine)" },
  { id: "flux-kit-en", label: "Kit (British masculine)" },
  { id: "flux-alexis-en", label: "Alexis (American feminine)" },
  { id: "flux-cliff-en", label: "Cliff (American masculine)" },
  { id: "flux-sienna-en", label: "Sienna (American feminine)" },
  { id: "flux-cole-en", label: "Cole (American masculine)" },
  { id: "flux-brooke-en", label: "Brooke (American feminine)" },
  { id: "flux-colin-en", label: "Colin (British masculine)" },
  { id: "flux-gemma-en", label: "Gemma (British feminine)" },
  { id: "flux-haley-en", label: "Haley (American feminine)" },
  { id: "flux-heather-en", label: "Heather (American feminine)" },
  { id: "flux-miles-en", label: "Miles (American masculine)" },
  { id: "flux-sean-en", label: "Sean (British masculine)" },
  { id: "flux-bree-en", label: "Bree (American feminine)" },
  { id: "flux-brittany-en", label: "Brittany (American feminine)" },
  { id: "flux-bruce-en", label: "Bruce (American masculine)" },
  { id: "flux-conor-en", label: "Conor (British masculine)" },
  { id: "flux-donovan-en", label: "Donovan (American masculine)" },
  { id: "flux-drew-en", label: "Drew (American masculine)" },
  { id: "flux-elise-en", label: "Elise (American feminine)" },
  { id: "flux-jack-en", label: "Jack (British masculine)" },
  { id: "flux-kai-en", label: "Kai (Singaporean masculine)" },
  { id: "flux-kelsey-en", label: "Kelsey (American feminine)" },
  { id: "flux-maeve-en", label: "Maeve (Irish feminine)" },
  { id: "flux-marcelo-en", label: "Marcelo (Filipino masculine)" },
  { id: "flux-marcus-en", label: "Marcus (American masculine)" },
  { id: "flux-meena-en", label: "Meena (Indian feminine)" },
  { id: "flux-meghan-en", label: "Meghan (American feminine)" },
  { id: "flux-naveen-en", label: "Naveen (Indian masculine)" },
  { id: "flux-paige-en", label: "Paige (American feminine)" },
  { id: "flux-priya-en", label: "Priya (Indian feminine)" },
  { id: "flux-rufus-en", label: "Rufus (British masculine)" },
  { id: "flux-sharon-en", label: "Sharon (Australian feminine)" },
  { id: "flux-tanner-en", label: "Tanner (British masculine)" },
  { id: "flux-wade-en", label: "Wade (American masculine)" },
  { id: "flux-wes-en", label: "Wes (American masculine)" },
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
