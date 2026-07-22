import { describe, expect, it } from "vitest";
import { diffWords } from "./textDiff";

/** Reassembles one side of the diff to check nothing was lost or invented. */
function reconstruct(segments: ReturnType<typeof diffWords>, side: "before" | "after") {
  const skip = side === "before" ? "added" : "removed";
  return segments
    .filter((segment) => segment.type !== skip)
    .map((segment) => segment.text)
    .join("");
}

describe("diffWords", () => {
  it("reports no changes for identical text", () => {
    const segments = diffWords("the same words", "the same words");
    expect(segments.every((segment) => segment.type === "equal")).toBe(true);
  });

  it("round-trips both sides exactly", () => {
    const before = "please add a debounce to the inventory search";
    const after = "Add a debounce to the inventory search field.";
    const segments = diffWords(before, after);
    expect(reconstruct(segments, "before")).toBe(before);
    expect(reconstruct(segments, "after")).toBe(after);
  });

  it("marks only the words that actually changed", () => {
    const segments = diffWords("fix the login bug", "fix the logout bug");
    const changed = segments.filter((segment) => segment.type !== "equal").map((segment) => segment.text.trim());
    expect(changed).toEqual(["login", "logout"]);
  });

  it("handles pure insertion and pure deletion", () => {
    expect(diffWords("", "brand new").every((segment) => segment.type === "added")).toBe(true);
    expect(diffWords("all gone", "").every((segment) => segment.type === "removed")).toBe(true);
  });

  it("returns nothing for two empty inputs", () => {
    expect(diffWords("", "")).toEqual([]);
  });

  it("falls back to a block replacement beyond the token cap", () => {
    const before = "word ".repeat(1500);
    const after = "other ".repeat(1500);
    const segments = diffWords(before, after);
    expect(segments.map((segment) => segment.type)).toEqual(["removed", "added"]);
  });

  it("merges adjacent segments of the same type", () => {
    const segments = diffWords("a b c d", "a d");
    // "b c" was removed as one run, not two separate removed segments.
    expect(segments.filter((segment) => segment.type === "removed")).toHaveLength(1);
  });
});
