import { describe, expect, it } from "vitest";
import { defaultSettings } from "../lib/native";
import { validateSettings } from "./settingsValidation";

describe("validateSettings", () => {
  it("accepts the production defaults", () => expect(validateSettings(defaultSettings)).toEqual({}));
  it("rejects insecure remote cleanup endpoints", () => {
    expect(validateSettings({ ...defaultSettings, cleanupEndpoint: "http://example.com/v1" }).cleanupEndpoint).toBeDefined();
  });
  it("allows local HTTP endpoints for development", () => {
    expect(validateSettings({ ...defaultSettings, cleanupEndpoint: "http://127.0.0.1:11434/v1" }).cleanupEndpoint).toBeUndefined();
  });

  it("rejects two actions bound to the same gesture", () => {
    const settings = {
      ...defaultSettings,
      shortcuts: {
        ...defaultSettings.shortcuts,
        // Same gesture as commandMode, written with the modifiers reversed.
        pushToTalk: { modifiers: ["alt", "ctrl"], key: "B" },
      },
    };
    expect(validateSettings(settings).shortcuts).toBeDefined();
  });

  it("rejects an unassigned shortcut", () => {
    const settings = {
      ...defaultSettings,
      shortcuts: { ...defaultSettings.shortcuts, cancel: { modifiers: [], key: null } },
    };
    expect(validateSettings(settings).shortcuts).toBeDefined();
  });

  it("accepts distinct gestures that share a modifier set", () => {
    // Ctrl+Win and Ctrl+Win+Space differ only by the main key, and coexist.
    expect(validateSettings(defaultSettings).shortcuts).toBeUndefined();
  });

  it("bounds overlay opacity and history retention", () => {
    expect(validateSettings({ ...defaultSettings, overlayOpacity: 10 }).overlayOpacity).toBeDefined();
    expect(validateSettings({ ...defaultSettings, overlayOpacity: 90 }).overlayOpacity).toBeUndefined();
    expect(validateSettings({ ...defaultSettings, historyRetentionDays: 99999 }).historyRetentionDays).toBeDefined();
    // Zero is the documented "keep everything" value, not an invalid input.
    expect(validateSettings({ ...defaultSettings, historyRetentionDays: 0 }).historyRetentionDays).toBeUndefined();
  });
});
