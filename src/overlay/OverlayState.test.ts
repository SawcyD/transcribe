import { describe, expect, it } from "vitest";
import { overlayPresentation } from "./OverlayState";

describe("overlayPresentation", () => {
  it("shows a finish control only for hands-free listening", () => {
    expect(overlayPresentation("listening_push_to_talk").showFinish).toBe(false);
    expect(overlayPresentation("listening_hands_free").showFinish).toBe(true);
  });
  it("maps insertion failures to the recovery state", () => expect(overlayPresentation("error").tone).toBe("error"));
  it("keeps startup explicit without pretending to listen", () => {
    expect(overlayPresentation("starting")).toMatchObject({ label: "Starting", tone: "processing" });
  });
});
