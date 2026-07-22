export interface DiffSegment {
  type: "equal" | "added" | "removed";
  text: string;
}

/**
 * Word-level diff via a longest-common-subsequence table.
 *
 * Transform outputs are short (capped at 50k characters upstream, and in
 * practice a few paragraphs), so the quadratic table is acceptable and gives
 * exact results rather than the heuristic a streaming diff would produce.
 * Inputs beyond `MAX_TOKENS` fall back to a whole-block replacement.
 */
const MAX_TOKENS = 1200;

function tokenize(value: string): string[] {
  // Each token carries its *leading* whitespace, so reassembling either side
  // preserves spacing exactly. Attaching trailing whitespace instead would let
  // a word swallow the space before an inserted neighbour, joining the two.
  const tokens: string[] = value.match(/\s*\S+/g) ?? [];
  const trailing = value.match(/\s+$/);
  if (trailing && tokens.length > 0) tokens.push(trailing[0]);
  return tokens;
}

export function diffWords(before: string, after: string): DiffSegment[] {
  const a = tokenize(before);
  const b = tokenize(after);

  if (a.length === 0 && b.length === 0) return [];
  if (a.length > MAX_TOKENS || b.length > MAX_TOKENS) {
    const segments: DiffSegment[] = [];
    if (before) segments.push({ type: "removed", text: before });
    if (after) segments.push({ type: "added", text: after });
    return segments;
  }

  // lengths[i][j] = LCS length of a[i..] and b[j..]
  const lengths: number[][] = Array.from({ length: a.length + 1 }, () => new Array<number>(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i -= 1) {
    for (let j = b.length - 1; j >= 0; j -= 1) {
      lengths[i][j] =
        a[i].trim() === b[j].trim()
          ? lengths[i + 1][j + 1] + 1
          : Math.max(lengths[i + 1][j], lengths[i][j + 1]);
    }
  }

  const segments: DiffSegment[] = [];
  const push = (type: DiffSegment["type"], text: string) => {
    const last = segments[segments.length - 1];
    if (last && last.type === type) last.text += text;
    else segments.push({ type, text });
  };

  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i].trim() === b[j].trim()) {
      push("equal", a[i]);
      i += 1;
      j += 1;
    } else if (lengths[i + 1][j] >= lengths[i][j + 1]) {
      push("removed", a[i]);
      i += 1;
    } else {
      push("added", b[j]);
      j += 1;
    }
  }
  while (i < a.length) {
    push("removed", a[i]);
    i += 1;
  }
  while (j < b.length) {
    push("added", b[j]);
    j += 1;
  }

  return segments;
}
