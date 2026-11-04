import type { TranscriptRecord } from "../types/models";

/** Average typing speed used for the "time saved" estimate. */
export const TYPING_WPM = 40;

export interface HomeStats {
  wordsToday: number;
  dictationsToday: number;
  /** Minutes saved today vs typing at TYPING_WPM, never negative. */
  minutesSavedToday: number;
  /** Consecutive days with at least one dictation, ending today or yesterday. */
  streakDays: number;
  wordsAllTime: number;
}

const dayKey = (date: Date) => `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;

function words(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

export function computeHomeStats(records: TranscriptRecord[], now = new Date()): HomeStats {
  const today = dayKey(now);
  const days = new Set<string>();
  let wordsToday = 0;
  let dictationsToday = 0;
  let speakingMsToday = 0;
  let wordsAllTime = 0;
  for (const record of records) {
    if (record.insertionStatus === "cancelled") continue;
    const count = words(record.finalTranscript);
    const created = new Date(record.createdAt);
    wordsAllTime += count;
    days.add(dayKey(created));
    if (dayKey(created) === today) {
      wordsToday += count;
      dictationsToday += 1;
      speakingMsToday += record.durationMs;
    }
  }
  const typingMinutes = wordsToday / TYPING_WPM;
  const minutesSavedToday = Math.max(0, typingMinutes - speakingMsToday / 60_000);

  let streakDays = 0;
  const cursor = new Date(now);
  // A streak is still alive if you haven't dictated *yet* today.
  if (!days.has(dayKey(cursor))) cursor.setDate(cursor.getDate() - 1);
  while (days.has(dayKey(cursor))) {
    streakDays += 1;
    cursor.setDate(cursor.getDate() - 1);
  }
  return { wordsToday, dictationsToday, minutesSavedToday, streakDays, wordsAllTime };
}

export function formatMinutes(minutes: number): string {
  if (minutes < 1) return `${Math.round(minutes * 60)}s`;
  if (minutes < 60) return `${Math.round(minutes)} min`;
  return `${Math.floor(minutes / 60)}h ${Math.round(minutes % 60)}m`;
}
