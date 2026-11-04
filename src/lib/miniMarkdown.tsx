import { Fragment, type ReactNode } from "react";
import { parseBlocks, type Block } from "./markdownBlocks";

/**
 * Just enough Markdown for assistant answers: paragraphs, headings, bullet and
 * numbered lists, fenced code, `inline code` and **bold**. Renders React
 * elements only (never innerHTML), so model output can't inject markup.
 */
export function MiniMarkdown({ text }: { text: string }) {
  return <>{parseBlocks(text).map((block, index) => renderBlock(block, index))}</>;
}

function inline(text: string): ReactNode[] {
  return text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g).map((part, index) => {
    if (part.startsWith("`") && part.endsWith("`") && part.length > 1) return <code key={index}>{part.slice(1, -1)}</code>;
    if (part.startsWith("**") && part.endsWith("**") && part.length > 3) return <strong key={index}>{part.slice(2, -2)}</strong>;
    return <Fragment key={index}>{part}</Fragment>;
  });
}

function renderBlock(block: Block, key: number) {
  switch (block.kind) {
    case "code":
      return <pre key={key}><code>{block.text}</code></pre>;
    case "heading":
      return <p key={key} className="md-heading">{inline(block.text)}</p>;
    case "list": {
      const List = block.ordered ? "ol" : "ul";
      return <List key={key}>{block.items.map((item, index) => <li key={index}>{inline(item)}</li>)}</List>;
    }
    default:
      return <p key={key}>{inline(block.text)}</p>;
  }
}
