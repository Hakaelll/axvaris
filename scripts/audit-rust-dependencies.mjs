import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repository = path.resolve(import.meta.dirname, "..");
const metadataResult = spawnSync(
  "cargo",
  ["metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"],
  { cwd: repository, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
);
if (metadataResult.status !== 0) {
  throw new Error(metadataResult.stderr || "cargo metadata failed");
}
const metadata = JSON.parse(metadataResult.stdout);
const members = new Set(metadata.workspace_members);

function rustSources(root) {
  const files = [];
  function walk(directory) {
    if (!fs.existsSync(directory)) return;
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      if (entry.name === "target" || entry.name === ".git") continue;
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) walk(absolute);
      else if (entry.isFile() && entry.name.endsWith(".rs")) files.push(absolute);
    }
  }
  walk(root);
  return files;
}

const unused = [];
let directDependencies = 0;
for (const item of metadata.packages.filter((candidate) => members.has(candidate.id))) {
  const root = path.dirname(item.manifest_path);
  const source = rustSources(root).map((filename) => fs.readFileSync(filename, "utf8")).join("\n");
  for (const dependency of item.dependencies) {
    directDependencies += 1;
    const identifier = (dependency.rename ?? dependency.name).replaceAll("-", "_");
    const pattern = new RegExp(`\\b${identifier.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`);
    if (!pattern.test(source)) {
      unused.push(
        `${item.name}: ${dependency.rename ? `${dependency.rename} (${dependency.name})` : dependency.name}` +
          ` [${dependency.kind ?? "normal"}${dependency.target ? `; ${dependency.target}` : ""}]`,
      );
    }
  }
}

if (unused.length) {
  console.error(`Rust direct-dependency audit failed; no symbol reference found:\n  ${unused.join("\n  ")}`);
  process.exitCode = 1;
} else {
  console.log(
    `Rust direct-dependency audit passed: ${directDependencies} declarations across ${members.size} workspace packages have source consumers (including cfg-specific and dev targets).`,
  );
}
