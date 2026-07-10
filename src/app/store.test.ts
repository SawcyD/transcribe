import { describe, expect, it } from "vitest";
import { appReducer, initialAppState } from "./state";

describe("appReducer", () => {
  it("tracks an explicit push-to-talk lifecycle", () => {
    const listening = appReducer(initialAppState, { type: "snapshot", value: {
      state: "listening_push_to_talk",
      sessionId: "session-1",
      mode: "push_to_talk",
      interimTranscript: "",
    } });
    expect(listening.dictation.state).toBe("listening_push_to_talk");
    const finalizing = appReducer(listening, { type: "snapshot", value: {
      ...listening.dictation,
      state: "finalizing_audio",
    } });
    expect(finalizing.dictation.sessionId).toBe("session-1");
  });

  it("clears stale audio after returning to idle", () => {
    const withAudio = appReducer(initialAppState, { type: "audio", value: {
      sessionId: "session-1", rms: .5, peak: .8, decibels: -6, bars: [.2, .8],
    } });
    const idle = appReducer(withAudio, { type: "snapshot", value: { state: "idle", interimTranscript: "" } });
    expect(idle.audio).toBeNull();
  });
});
