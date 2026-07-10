import type { AppSettings } from "../types/models";

export interface SettingsErrors {
  cleanupEndpoint?: string;
  pasteDelayMs?: string;
  sessionLimitMinutes?: string;
  noiseFloorDb?: string;
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
  if (settings.pasteDelayMs < 40 || settings.pasteDelayMs > 2_000) errors.pasteDelayMs = "Use a delay from 40 to 2000 ms.";
  if (settings.sessionLimitMinutes < 1 || settings.sessionLimitMinutes > 120) errors.sessionLimitMinutes = "Use a limit from 1 to 120 minutes.";
  if (settings.noiseFloorDb < -90 || settings.noiseFloorDb > -10) errors.noiseFloorDb = "Use a noise floor from -90 to -10 dB.";
  return errors;
}
