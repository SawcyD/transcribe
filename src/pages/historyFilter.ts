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

/** Renders transcripts as a Markdown document, newest first, grouped by day. */
export function historyToMarkdown(records: TranscriptRecord[], now = new Date()): string {
  const lines = [`# VoiceFlow dictations`, ``, `_Exported ${now.toLocaleString()} · ${records.length} ${records.length === 1 ? "dictation" : "dictations"}_`];
  let day = "";
  for (const record of [...records].sort((a, b) => b.createdAt.localeCompare(a.createdAt))) {
    const created = new Date(record.createdAt);
    const label = created.toLocaleDateString([], { weekday: "long", year: "numeric", month: "long", day: "numeric" });
    if (label !== day) {
      day = label;
      lines.push("", `## ${label}`);
    }
    const time = created.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
    const star = record.isFavorite ? " ★" : "";
    lines.push("", `**${time} · ${record.applicationName ?? "Unknown app"}**${star}`, "", record.finalTranscript.trim() || "_(empty)_");
  }
  return `${lines.join("\n")}\n`;
}
