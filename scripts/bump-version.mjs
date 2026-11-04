// Sets the app version everywhere it lives: package.json, tauri.conf.json,
// Cargo.toml and Cargo.lock. Usage: pnpm release:version 1.2.0
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version ?? "")) {
  console.error("Usage: pnpm release:version <major.minor.patch>   e.g. pnpm release:version 1.2.0");
  process.exit(1);
}

const edit = (path, pattern, replacement) => {
  const before = readFileSync(path, "utf8");
  const after = before.replace(pattern, replacement);
  if (after === before) throw new Error(`No version found to update in ${path}`);
  writeFileSync(path, after);
  console.log(`  ${path}`);
};

console.log(`Setting version ${version} in:`);
edit("package.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
edit("src-tauri/tauri.conf.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
edit("src-tauri/Cargo.toml", /^(version\s*=\s*")[^"]+(")/m, `$1${version}$2`);
edit("src-tauri/Cargo.lock", /(name = "voiceflow-dev"\nversion = ")[^"]+(")/, `$1${version}$2`);

const changelog = readFileSync("CHANGELOG.md", "utf8");
if (!changelog.includes(`## v${version}`)) {
  console.warn(`\n⚠ CHANGELOG.md has no "## v${version}" section yet. Add one before tagging: it becomes the release notes.`);
}
