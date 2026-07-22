import { describe, expect, it } from "vitest";
import { DICTATION_STATES } from "../types/models";
import { overlayPresentation } from "./OverlayState";

describe("overlayPresentation", () => {
  it("shows a finish control only for hands-free listening", () => {
    expect(overlayPresentation("listening_push_to_talk").showFinish).toBe(false);
    expect(overlayPresentation("listening_hands_free").showFinish).toBe(true);
  });

  it("maps insertion failures to the recovery state", () => {
    const presentation = overlayPresentation("error");
    expect(presentation.tone).toBe("error");
    // The transcript is never lost, so the overlay says where it went.
    expect(presentation.detail).toMatch(/clipboard/i);
  });

  it("keeps startup explicit without pretending to listen", () => {
    expect(overlayPresentation("starting")).toMatchObject({
      label: "Starting microphone…",
      tone: "processing",
      showWaveform: false,
    });
  });

  it("names the insertion target when one is known", () => {
    expect(overlayPresentation("inserting", "Visual Studio Code").label).toBe(
      "Inserting into Visual Studio Code…",
    );
    expect(overlayPresentation("inserting", null).label).toBe("Inserting…");
  });

  it("only draws the waveform while audio is actually being captured", () => {
    const capturing = DICTATION_STATES.filter((state) => overlayPresentation(state).showWaveform);
    expect(capturing).toEqual(["listening_push_to_talk", "listening_hands_free"]);
  });

  it("gives every dictation state its own presentation", () => {
    for (const state of DICTATION_STATES) {
      expect(overlayPresentation(state).label.length).toBeGreaterThan(0);
    }
    // Distinct processing stages must not collapse into one generic label.
    const labels = new Set(DICTATION_STATES.map((state) => overlayPresentation(state).label));
    expect(labels.size).toBeGreaterThanOrEqual(8);
  });
});
