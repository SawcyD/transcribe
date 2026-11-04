import { describe, expect, it } from "vitest";
import { DICTATION_STATES } from "../types/models";
import { captionTail, overlayPresentation } from "./OverlayState";

describe("overlayPresentation", () => {
  it("shows a finish control only for hands-free listening", () => {
    expect(overlayPresentation("listening_push_to_talk").showFinish).toBe(false);
    expect(overlayPresentation("listening_hands_free").showFinish).toBe(true);
  });

  it("maps insertion failures to the recovery state", () => {
    const presentation = overlayPresentation("error", null, { category: "insertion", message: "x", recoverable: true });
    expect(presentation.tone).toBe("error");
    // The transcript is never lost, so the overlay says where it went.
    expect(presentation.detail).toMatch(/copied/i);
  });

  it("never claims a paste failed for errors that are not about pasting", () => {
    for (const category of ["empty_transcript", "microphone", "credential", "transcription", "database", "state"]) {
      const presentation = overlayPresentation("error", null, { category, message: "boom", recoverable: true });
      expect(presentation.tone).toBe("error");
      expect(`${presentation.label} ${presentation.detail ?? ""}`).not.toMatch(/paste|clipboard/i);
    }
    expect(overlayPresentation("error", null, { category: "empty_transcript", message: "", recoverable: true }).label).toBe(
      "Didn't catch that",
    );
  });

  it("reports what was inserted on success", () => {
    expect(overlayPresentation("completed", null, null, "Ship the fix today.").detail).toBe("4 words");
    expect(overlayPresentation("completed").detail).toBeUndefined();
  });

  it("streams a live caption only while speech is being captured or transcribed", () => {
    expect(overlayPresentation("listening_push_to_talk").showCaption).toBe(true);
    expect(overlayPresentation("transcribing").showCaption).toBe(true);
    expect(overlayPresentation("inserting").showCaption).toBe(false);
  });

  it("keeps the newest words of a long caption in view", () => {
    expect(captionTail("short text")).toBe("short text");
    const long = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen";
    const tail = captionTail(long, 30);
    expect(tail.startsWith("…")).toBe(true);
    expect(tail.endsWith("fourteen")).toBe(true);
    expect(tail.length).toBeLessThanOrEqual(31);
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
