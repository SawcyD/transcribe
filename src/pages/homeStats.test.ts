import { describe, expect, it } from "vitest";
import type { TranscriptRecord } from "../types/models";
import { computeHomeStats, formatMinutes } from "./homeStats";

const now = new Date(2026, 10, 4, 15, 0);
const record = (daysAgo: number, text: string, durationMs = 5_000, status: TranscriptRecord["insertionStatus"] = "inserted") =>
  ({
    createdAt: new Date(2026, 10, 4 - daysAgo, 10, 0).toISOString(),
    finalTranscript: text,
    durationMs,
    insertionStatus: status,
  }) as TranscriptRecord;

describe("computeHomeStats", () => {
  it("counts today's words and dictations and estimates time saved", () => {
    const words80 = Array.from({ length: 80 }, () => "word").join(" ");
    const stats = computeHomeStats([record(0, words80, 30_000), record(0, "hello there"), record(1, "yesterday text")], now);
    expect(stats.wordsToday).toBe(82);
    expect(stats.dictationsToday).toBe(2);
    // 82 words at 40 wpm = 2.05 min typing, minus 35 s spoken.
    expect(stats.minutesSavedToday).toBeCloseTo(2.05 - 35 / 60, 5);
    expect(stats.wordsAllTime).toBe(84);
  });

  it("ignores cancelled dictations", () => {
    expect(computeHomeStats([record(0, "never sent", 1_000, "cancelled")], now).dictationsToday).toBe(0);
  });

  it("counts a streak through yesterday even before today's first dictation", () => {
    expect(computeHomeStats([record(1, "a"), record(2, "b"), record(4, "c")], now).streakDays).toBe(2);
    expect(computeHomeStats([record(0, "a"), record(1, "b")], now).streakDays).toBe(2);
    expect(computeHomeStats([record(3, "a")], now).streakDays).toBe(0);
  });

  it("formats minutes compactly", () => {
    expect(formatMinutes(0.5)).toBe("30s");
    expect(formatMinutes(12.4)).toBe("12 min");
    expect(formatMinutes(75)).toBe("1h 15m");
  });
});
