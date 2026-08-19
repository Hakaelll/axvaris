import fs from "node:fs";
import path from "node:path";

const repository = fs.realpathSync(path.resolve(import.meta.dirname, ".."));
const targetRoot = path.join(repository, "target");
const execute = process.argv.includes("--execute");
const json = process.argv.includes("--json");
const enforceBuildLimit = process.argv.includes("--enforce-build-limit");
const command = process.argv[2] ?? "report";
const scopeArgument = process.argv.find((value) => value.startsWith("--scope="));
const scope = scopeArgument?.slice("--scope=".length) ?? "incremental";

function bytesAt(filename) {
  if (!fs.existsSync(filename)) return 0;
  const stat = fs.lstatSync(filename);
  if (!stat.isDirectory()) return stat.size;
  let bytes = 0;
  for (const entry of fs.readdirSync(filename, { withFileTypes: true })) {
    if (entry.isSymbolicLink()) continue;
    bytes += bytesAt(path.join(filename, entry.name));
  }
  return bytes;
}

function findNamedDirectories(root, name) {
  if (!fs.existsSync(root)) return [];
  const matches = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    if (!entry.isDirectory() || entry.isSymbolicLink()) continue;
    const absolute = path.join(root, entry.name);
    if (entry.name === name) matches.push(absolute);
    else matches.push(...findNamedDirectories(absolute, name));
  }
  return matches;
}

function withinRepository(candidate) {
  const resolved = fs.existsSync(candidate)
    ? fs.realpathSync(candidate)
    : path.resolve(candidate);
  const relative = path.relative(repository, resolved);
  if (relative === "" || relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`Refusing path outside the workspace: ${resolved}`);
  }
  return resolved;
}

function report() {
  const categories = [
    ["target/debug", path.join(targetRoot, "debug")],
    ["target/release", path.join(targetRoot, "release")],
    ["target/linux-cross", path.join(targetRoot, "x86_64-unknown-linux-gnu")],
    ["target/flycheck", path.join(targetRoot, "flycheck0")],
    ["node_modules", path.join(repository, "node_modules")],
    ["pnpm store", path.join(repository, ".pnpm-store")],
    ["desktop dist", path.join(repository, "apps", "desktop", "dist")],
    ["local releases", path.join(repository, "release")],
  ].map(([label, filename]) => ({ label, path: filename, bytes: bytesAt(filename) }));
  const targetBytes = bytesAt(targetRoot);
  const repositoryBytes = fs
    .readdirSync(repository, { withFileTypes: true })
    .filter((entry) => entry.name !== ".git")
    .reduce((total, entry) => total + bytesAt(path.join(repository, entry.name)), 0);
  return {
    generatedAt: new Date().toISOString(),
    repository,
    repositoryBytes,
    targetBytes,
    warning: targetBytes >= 6 * 1024 ** 3,
    limitExceeded: targetBytes >= 10 * 1024 ** 3,
    categories,
  };
}

function cleanupTargets() {
  const scopes = {
    incremental: findNamedDirectories(targetRoot, "incremental"),
    debug: [path.join(targetRoot, "debug"), path.join(targetRoot, "flycheck0")],
    release: [path.join(targetRoot, "release")],
    cross: fs.existsSync(targetRoot)
      ? fs
          .readdirSync(targetRoot, { withFileTypes: true })
          .filter((entry) => entry.isDirectory() && entry.name.includes("-unknown-"))
          .map((entry) => path.join(targetRoot, entry.name))
      : [],
    full: [targetRoot],
  };
  if (!(scope in scopes)) {
    throw new Error(`Unknown scope ${scope}; use incremental, debug, release, cross or full`);
  }
  return scopes[scope]
    .filter((filename) => fs.existsSync(filename))
    .map((filename) => ({ path: withinRepository(filename), bytes: bytesAt(filename) }));
}

if (command === "report") {
  const result = report();
  if (json) console.log(JSON.stringify(result, null, 2));
  else {
    console.log(`Workspace: ${result.repository}`);
    for (const item of result.categories) {
      console.log(`${item.label.padEnd(20)} ${item.bytes.toLocaleString("en-US")} B`);
    }
    console.log(`target total         ${result.targetBytes.toLocaleString("en-US")} B`);
    console.log(`workspace without git ${result.repositoryBytes.toLocaleString("en-US")} B`);
    if (result.limitExceeded) {
      console.log("ACTION REQUIRED: target exceeds 10 GiB; inspect a dry-run cleanup scope.");
    } else if (result.warning) {
      console.log("WARNING: target exceeds 6 GiB; consider cleaning regenerable caches.");
    }
  }
  if (enforceBuildLimit && result.targetBytes > 8 * 1024 ** 3) {
    console.error("BUILD SPACE GATE FAILED: target exceeds 8 GiB after the clean gate.");
    process.exitCode = 1;
  }
} else if (command === "clean") {
  const targets = cleanupTargets();
  const total = targets.reduce((sum, item) => sum + item.bytes, 0);
  console.log(`${execute ? "Cleaning" : "Dry run"} scope=${scope}`);
  for (const item of targets) {
    console.log(`${item.path} (${item.bytes.toLocaleString("en-US")} B)`);
  }
  console.log(`Recoverable: ${total.toLocaleString("en-US")} B`);
  if (!execute) {
    console.log("No files removed. Repeat with --execute after reviewing the exact paths.");
  } else {
    for (const item of targets) fs.rmSync(item.path, { recursive: true, force: true });
    console.log(`Removed ${total.toLocaleString("en-US")} B of regenerable build artifacts.`);
  }
} else {
  throw new Error("Usage: workspace-space.mjs report [--json] | clean --scope=<scope> [--execute]");
}
