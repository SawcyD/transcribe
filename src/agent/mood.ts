export type Mood = "neutral" | "happy" | "sad";

// ponytail: this reads the words, not the tone of voice. If it misreads too
// often, have the model report a mood alongside each reply instead.
const SAD = /\b(sorry|sad|unfortunately|tough|rough|stressed|worried|anxious|tired|upset|hard time|failed|rejected|afraid|nervous)\b/i;
const HAPPY = /\b(great|awesome|amazing|love|nice|congrats|congratulations|excited|exciting|perfect|fantastic|haha|yay|well done|good job|glad|happy)\b|!/i;

/** The mood of one spoken line, used to tint and pace the voice agent's orb. */
export function moodOf(text: string): Mood {
  if (SAD.test(text)) return "sad";
  if (HAPPY.test(text)) return "happy";
  return "neutral";
}
