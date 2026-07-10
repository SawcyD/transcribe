import type { TranscriptRecord } from "../types/models";

export function filterHistory(records: TranscriptRecord[], query: string): TranscriptRecord[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return records;
  return records.filter((record) => [
    record.rawTranscript,
    record.normalizedTranscript,
    record.cleanedTranscript,
    record.finalTranscript,
    record.applicationName,
    record.windowTitle,
  ].some((value) => value?.toLocaleLowerCase().includes(needle)));
}
