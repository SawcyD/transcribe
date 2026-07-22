import type { AppSettings } from "../types/models";

export interface SettingsErrors {
  cleanupEndpoint?: string;
  assistantEndpoint?: string;
  pasteDelayMs?: string;
  sessionLimitMinutes?: string;
  noiseFloorDb?: string;
  overlayOpacity?: string;
  historyRetentionDays?: string;
  shortcuts?: string;
}

const SHORTCUT_LABELS: Record<string, string> = {
  pushToTalk: "Push to talk",
  handsFree: "Hands-free dictation",
  commandMode: "Command Mode",
  cancel: "Cancel",
};

/** Order-insensitive identity for a gesture, mirroring the Rust canonical form. */
function canonicalShortcut(binding: { modifiers: string[]; key: string | null }): string {
  const modifiers = [...new Set(binding.modifiers.map((value) => value.toLowerCase()))].sort();
  return `${modifiers.join("+")}+${(binding.key ?? "").toUpperCase()}`;
}

export function validateSettings(settings: AppSettings): SettingsErrors {
  const errors: SettingsErrors = {};
  try {
    const url = new URL(settings.cleanupEndpoint);
    if (url.protocol !== "https:" && url.hostname !== "localhost" && url.hostname !== "127.0.0.1") {
      errors.cleanupEndpoint = "Use HTTPS, except for a local endpoint.";
    }
  } catch {
    errors.cleanupEndpoint = "Enter a valid endpoint URL.";
  }
  try {
    const url = new URL(settings.assistantEndpoint);
    if (url.protocol !== "https:" && url.hostname !== "localhost" && url.hostname !== "127.0.0.1") {
      errors.assistantEndpoint = "Use HTTPS, except for a local endpoint.";
    }
  } catch {
    errors.assistantEndpoint = "Enter a valid endpoint URL.";
  }
  if (settings.pasteDelayMs < 40 || settings.pasteDelayMs > 2_000) errors.pasteDelayMs = "Use a delay from 40 to 2000 ms.";
  if (settings.sessionLimitMinutes < 1 || settings.sessionLimitMinutes > 120) errors.sessionLimitMinutes = "Use a limit from 1 to 120 minutes.";
  if (settings.noiseFloorDb < -90 || settings.noiseFloorDb > -10) errors.noiseFloorDb = "Use a noise floor from -90 to -10 dB.";
  if (settings.overlayOpacity < 40 || settings.overlayOpacity > 100) errors.overlayOpacity = "Use an overlay opacity from 40 to 100%.";
  if (settings.historyRetentionDays < 0 || settings.historyRetentionDays > 3650) {
    errors.historyRetentionDays = "Use a retention period from 0 to 3650 days.";
  }

  // Mirrors the Rust conflict check so the save button disables before the
  // backend rejects the settings.
  const seen = new Map<string, string>();
  for (const [id, label] of Object.entries(SHORTCUT_LABELS)) {
    const binding = settings.shortcuts?.[id as keyof typeof settings.shortcuts];
    if (!binding || (binding.modifiers.length === 0 && !binding.key)) {
      errors.shortcuts = `${label} has no shortcut assigned.`;
      break;
    }
    const canonical = canonicalShortcut(binding);
    const other = seen.get(canonical);
    if (other) {
      errors.shortcuts = `${other} and ${label} use the same shortcut.`;
      break;
    }
    seen.set(canonical, label);
  }

  return errors;
}
