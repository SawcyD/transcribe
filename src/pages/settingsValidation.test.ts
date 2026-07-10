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
});
