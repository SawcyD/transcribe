import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { parseBlocks } from "./markdownBlocks";
import { MiniMarkdown } from "./miniMarkdown";

describe("MiniMarkdown", () => {
  it("splits headings, lists, code and paragraphs", () => {
    const blocks = parseBlocks("## Cause\nThe port is busy.\n\n1. Stop it\n2. Retry\n```\npnpm dev\n```\n- done");
    expect(blocks.map((block) => block.kind)).toEqual(["heading", "paragraph", "list", "code", "list"]);
  });

  it("never renders raw HTML from the model", () => {
    const { container } = render(<MiniMarkdown text={'<img src=x onerror="alert(1)"> **bold** `code`'} />);
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("strong")?.textContent).toBe("bold");
    expect(container.querySelector("code")?.textContent).toBe("code");
  });
});
