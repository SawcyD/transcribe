import { describe, expect, it } from "vitest";
import { buddyMoodFor } from "./BuddyState";

describe("buddyMoodFor", () => {
  it("uses the microphone pose while dictation is listening", () => {
    expect(buddyMoodFor("listening_push_to_talk", null, false)).toBe("push_to_talk");
    expect(buddyMoodFor("listening_hands_free", null, false)).toBe("hands_free");
    expect(buddyMoodFor("listening_hands_free", null, false, "call")).toBe("call_recording");
  });

  it("prioritizes an explicit screen capture action", () => {
    expect(buddyMoodFor("idle", "capturing", false)).toBe("capturing");
  });

  it("rests only when VoiceFlow is idle", () => {
    expect(buddyMoodFor("idle", null, true)).toBe("sleeping");
    expect(buddyMoodFor("cleaning", null, true)).toBe("analyzing");
  });
});
