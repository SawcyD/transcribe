// Prints the CHANGELOG.md section for one version (used by the release
// workflow as the GitHub release body and the in-app update notes).
// Usage: node scripts/release-notes.mjs 1.2.0
import { readFileSync } from "node:fs";

const version = (process.argv[2] ?? "").replace(/^v/, "");
const lines = readFileSync("CHANGELOG.md", "utf8").split("\n");
const start = lines.findIndex((line) => line.startsWith(`## v${version}`));
if (start === -1) {
  console.error(`CHANGELOG.md has no "## v${version}" section.`);
  process.exit(1);
}
const end = lines.findIndex((line, index) => index > start && line.startsWith("## v"));
process.stdout.write(lines.slice(start + 1, end === -1 ? undefined : end).join("\n").trim() + "\n");
