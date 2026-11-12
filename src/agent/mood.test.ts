import { expect, test } from "vitest";
import { moodOf } from "./mood";

test("reads the mood of a spoken line", () => {
  expect(moodOf("That's awesome, congrats on the offer")).toBe("happy");
  expect(moodOf("I'm sorry, that sounds really tough!")).toBe("sad");
  expect(moodOf("Tell me about a hard problem you solved.")).toBe("neutral");
});
