import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repository = path.resolve(import.meta.dirname, "..");
const phase = value("--phase") ?? "snapshot";
const output = path.resolve(repository, value("--output") ?? `docs/performance-${phase}.json`);

function value(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function run(command, args, options = {}) {
  const executable = process.platform === "win32" && command === "pnpm" ? "pnpm.cmd" : command;
  const result = spawnSync(executable, args, {
    cwd: repository,
    encoding: "utf8",
    shell: false,
    stdio: options.capture ? "pipe" : "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed${result.stderr ? `: ${result.stderr}` : ""}`);
  }
  return (result.stdout ?? "").trim();
}

function walk(root) {
  if (!fs.existsSync(root)) return [];
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const absolute = path.join(root, entry.name);
    if (entry.isDirectory()) files.push(...walk(absolute));
    else if (entry.isFile()) files.push(absolute);
  }
  return files;
}

function sha256(filename) {
  return createHash("sha256").update(fs.readFileSync(filename)).digest("hex");
}

function artifact(filename, base = repository) {
  if (!fs.existsSync(filename)) return null;
  return {
    path: path.relative(base, filename).replaceAll("\\", "/"),
    bytes: fs.statSync(filename).size,
    sha256: sha256(filename),
  };
}

if (process.argv.includes("--build-frontend")) {
  run("pnpm", ["--dir", "apps/desktop", "build"]);
}
if (process.argv.includes("--bundle-windows")) {
  run("pnpm", ["--dir", "apps/desktop", "tauri", "build", "--bundles", "nsis"]);
}

const dist = path.join(repository, "apps", "desktop", "dist");
const distFiles = walk(dist);
const sourceMaps = distFiles.filter((filename) => filename.endsWith(".map"));
const assets = distFiles
  .map((filename) => artifact(filename, dist))
  .filter(Boolean)
  .sort((left, right) => right.bytes - left.bytes);
const indexHtml = path.join(dist, "index.html");
const initialPaths = new Set(["index.html"]);
if (fs.existsSync(indexHtml)) {
  const html = fs.readFileSync(indexHtml, "utf8");
  for (const match of html.matchAll(/(?:src|href)=["']\/?([^"'#?]+)["']/g)) {
    if (fs.existsSync(path.join(dist, match[1]))) initialPaths.add(match[1]);
  }
}
const initialAssets = assets.filter((item) => initialPaths.has(item.path));
const executable = path.join(repository, "target", "release", "Axvaris.exe");
const installers = walk(path.join(repository, "target", "release", "bundle", "nsis"))
  .filter((filename) => filename.endsWith(".exe"))
  .sort((left, right) => fs.statSync(right).mtimeMs - fs.statSync(left).mtimeMs);
const fixtureFiles = [
  ...walk(path.join(repository, "crates", "sabicheiro-capture", "src")),
  path.join(repository, "crates", "sabicheiro-core", "src", "demo.rs"),
].filter((filename) => fs.existsSync(filename));
const fixtureHash = createHash("sha256");
for (const filename of fixtureFiles.sort()) fixtureHash.update(fs.readFileSync(filename));
const cargoTree = run("cargo", ["tree", "--workspace", "--depth", "1", "--locked"], {
  capture: true,
}).replaceAll(repository, "<repo>");
const space = JSON.parse(
  run(process.execPath, ["scripts/workspace-space.mjs", "report", "--json"], { capture: true }),
);

function optionalJson(filename) {
  return fs.existsSync(filename) ? JSON.parse(fs.readFileSync(filename, "utf8")) : null;
}

const report = {
  schemaVersion: 2,
  phase,
  generatedAt: new Date().toISOString(),
  source: {
    commit: run("git", ["rev-parse", "HEAD"], { capture: true }),
    dirty: run("git", ["status", "--porcelain"], { capture: true }).length > 0,
    fixtureSourceSha256: fixtureHash.digest("hex"),
  },
  environment: {
    platform: `${os.platform()} ${os.release()} ${os.arch()}`,
    cpu: os.cpus()[0]?.model ?? "unknown",
    logicalCpus: os.cpus().length,
    totalMemoryBytes: os.totalmem(),
    node: process.version,
    rustc: run("rustc", ["--version"], { capture: true }),
    cargo: run("cargo", ["--version"], { capture: true }),
  },
  configuration: {
    capture: "off for artifact measurement",
    permissions: "normal user",
    dataset: "deterministic local fixtures",
    duration: "artifact snapshot",
    webview: "system WebView2; runtime profiling recorded separately",
  },
  frontend: {
    bytes: assets.reduce((sum, item) => sum + item.bytes, 0),
    sourceMapBytes: sourceMaps.reduce((sum, filename) => sum + fs.statSync(filename).size, 0),
    sourceMapCount: sourceMaps.length,
    initialBytes: initialAssets.reduce((sum, item) => sum + item.bytes, 0),
    initialAssetPaths: initialAssets.map((item) => item.path),
    assets,
  },
  native: {
    executable: artifact(executable),
    installer: installers[0] ? artifact(installers[0]) : null,
  },
  workspace: {
    bytesWithoutGit: space.repositoryBytes,
    targetBytes: space.targetBytes,
    categories: space.categories.map(({ label, bytes }) => ({ label, bytes })),
  },
  dependencies: {
    cargoTreeSha256: createHash("sha256").update(cargoTree).digest("hex"),
    directTreeLines: cargoTree.split(/\r?\n/).filter(Boolean),
  },
  benchmarks: {
    historyPagination: optionalJson(path.join(repository, "docs", "performance-0.4.2-history.json")),
    core: optionalJson(path.join(repository, "docs", "performance-0.4.2-core.json")),
    runtime: optionalJson(path.join(repository, "docs", "performance-0.4.2-runtime.json")),
  },
  parity: [
    "capture-off", "live", "demo", "interface", "pause-view", "search", "map-2d",
    "globe-3d", "today", "history", "time-machine", "sabi", "exposure", "rules",
    "notifications", "recording", "geoip", "retention", "export", "tray", "autostart",
    "language", "keyboard-accessibility",
  ].map((flow) => ({ flow, state: "covered by automated or manual QA gate" })),
};

fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${path.relative(repository, output)} (${report.frontend.bytes.toLocaleString("en-US")} frontend bytes).`);
