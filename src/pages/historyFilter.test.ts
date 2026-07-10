import { describe, expect, it } from "vitest";
import type { TranscriptRecord } from "../types/models";
import { filterHistory } from "./historyFilter";

const record: TranscriptRecord = {
  id: "1", createdAt: "2026-01-01T00:00:00Z", startedAt: "2026-01-01T00:00:00Z",
  durationMs: 900, processingMs: 120, applicationName: "Visual Studio Code", mode: "push_to_talk",
  rawTranscript: "raw", normalizedTranscript: "normalized", cleanedTranscript: "cleaned",
  finalTranscript: "Update InventoryController", provider: "deepgram", model: "nova-3",
  insertionStatus: "inserted", postPasteAction: "none", isFavorite: false,
};

describe("filterHistory", () => {
  it("matches final text and application without case sensitivity", () => {
    expect(filterHistory([record], "inventorycontroller")).toHaveLength(1);
    expect(filterHistory([record], "VISUAL STUDIO")).toHaveLength(1);
  });
  it("returns no records for an unrelated query", () => expect(filterHistory([record], "Discord")).toEqual([]));
});
