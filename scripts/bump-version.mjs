import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const target = process.argv[2];

if (!target) {
  throw new Error("Usage: pnpm version:bump <patch|minor|major|x.y.z>");
}

const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z.-]+))?(?:\+([0-9A-Za-z.-]+))?$/;

function nextVersion(current, kind) {
  const match = current.match(semver);
  if (!match) throw new Error(`Current package version is not valid semver: ${current}`);
  const [major, minor, patch] = match.slice(1, 4).map(Number);
  if (kind === "patch") return `${major}.${minor}.${patch + 1}`;
  if (kind === "minor") return `${major}.${minor + 1}.0`;
  if (kind === "major") return `${major + 1}.0.0`;
  if (!semver.test(kind)) throw new Error(`Invalid version: ${kind}`);
  return kind;
}

async function replace(file, pattern, replacement) {
  const source = await readFile(file, "utf8");
  const updated = source.replace(pattern, replacement);
  if (updated === source && !pattern.test(source)) {
    throw new Error(`Could not find the version field in ${path.relative(root, file)}`);
  }
  await writeFile(file, updated, "utf8");
}

const packageFile = path.join(root, "package.json");
const packageJson = JSON.parse(await readFile(packageFile, "utf8"));
const version = nextVersion(packageJson.version, target);

await replace(packageFile, /("version"\s*:\s*")[^"]+(")/, `$1${version}$2`);
await replace(path.join(root, "src-tauri", "Cargo.toml"), /(\[package\][\s\S]*?^version\s*=\s*")[^"]+/m, `$1${version}`);
await replace(path.join(root, "src-tauri", "tauri.conf.json"), /("version"\s*:\s*")[^"]+(")/, `$1${version}$2`);
await replace(path.join(root, "src", "pages", "AboutPage.tsx"), /(useState\(")[^"]+("\))/, `$1${version}$2`);
await replace(path.join(root, "src-tauri", "Cargo.lock"), /(name = "voiceflow-dev"\r?\nversion = ")[^"]+(")/, `$1${version}$2`);

console.log(`VoiceFlow version updated: ${packageJson.version} → ${version}`);
