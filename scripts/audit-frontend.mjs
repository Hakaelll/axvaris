import fs from "node:fs";
import path from "node:path";

const repository = path.resolve(import.meta.dirname, "..");
const sourceRoot = path.join(repository, "apps", "desktop", "src");
const tauriSource = path.join(repository, "apps", "desktop", "src-tauri", "src", "lib.rs");
const productionBackend = path.join(sourceRoot, "lib", "backend-tauri.ts");

function walk(root) {
  if (!fs.existsSync(root)) return [];
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(root, entry.name);
    return entry.isDirectory() ? walk(absolute) : [absolute];
  });
}

function resolveImport(from, specifier) {
  const base = specifier === "#backend"
    ? productionBackend.slice(0, -3)
    : specifier.startsWith(".")
      ? path.resolve(path.dirname(from), specifier)
      : null;
  if (!base) return null;
  for (const candidate of [base, `${base}.ts`, `${base}.tsx`, path.join(base, "index.ts"), path.join(base, "index.tsx")]) {
    if (fs.existsSync(candidate)) return path.normalize(candidate);
  }
  return null;
}

const modules = walk(sourceRoot).filter((filename) => /\.tsx?$/.test(filename) && !filename.endsWith(".d.ts"));
const graph = new Map();
for (const filename of modules) {
  const text = fs.readFileSync(filename, "utf8");
  const dependencies = [];
  const imports = /(?:\b(?:import|export)\s+(?:type\s+)?(?:[^"']*?\s+from\s+)?|\bimport\s*\()(["'])([^"']+)\1/g;
  for (const match of text.matchAll(imports)) {
    const resolved = resolveImport(filename, match[2]);
    if (resolved) dependencies.push(resolved);
  }
  graph.set(path.normalize(filename), dependencies);
}
const reachable = new Set();
const pending = [path.join(sourceRoot, "main.tsx")];
while (pending.length) {
  const filename = path.normalize(pending.pop());
  if (reachable.has(filename)) continue;
  reachable.add(filename);
  pending.push(...(graph.get(filename) ?? []));
}
const intentionalBuildVariants = new Set([
  path.join(sourceRoot, "lib", "backend.ts"),
  path.join(sourceRoot, "lib", "backend-contract.ts"),
].map(path.normalize));
const unreachable = modules.filter(
  (filename) => !reachable.has(path.normalize(filename)) && !intentionalBuildVariants.has(path.normalize(filename)),
);
const css = fs.readFileSync(path.join(sourceRoot, "styles.css"), "utf8");
const cssClasses = new Set(
  [...css.matchAll(/\.([A-Za-z_][A-Za-z0-9_-]*)/g)].map((match) => match[1]),
);
const reachableSource = [...reachable]
  .filter((filename) => fs.existsSync(filename))
  .map((filename) => fs.readFileSync(filename, "utf8"))
  .join("\n");
const dynamicCssClasses = new Set([
  "change-pill--added",
  "change-pill--removed",
  "density--compact",
  "is-degraded",
  "is-healthy",
  "is-unavailable",
  "main-stage--live",
]);
const unusedCssClasses = [...cssClasses]
  .filter((className) => !reachableSource.includes(className) && !dynamicCssClasses.has(className))
  .sort();

const rust = fs.readFileSync(tauriSource, "utf8");
const handler = rust.match(/generate_handler!\[([\s\S]*?)\]\)/)?.[1];
if (!handler) throw new Error("Could not locate the Tauri command handler");
const registered = new Set(handler.match(/\b[a-z][a-z0-9_]+\b/g) ?? []);
const adapter = fs.readFileSync(productionBackend, "utf8");
const invoked = new Set([...adapter.matchAll(/call(?:<[^>]+>)?\(\s*"([a-z0-9_]+)"/g)].map((match) => match[1]));
const adapterMethods = new Set(
  [...adapter.matchAll(/^  ([A-Za-z][A-Za-z0-9]*):/gm)].map((match) => match[1]),
);
const consumedAdapterMethods = new Set(
  [...reachableSource.matchAll(/\bdesktopApi\s*\.\s*([A-Za-z][A-Za-z0-9]*)/g)].map(
    (match) => match[1],
  ),
);
const unusedAdapterMethods = [...adapterMethods]
  .filter((method) => !consumedAdapterMethods.has(method))
  .sort();
const unconsumed = [...registered].filter((command) => !invoked.has(command)).sort();
const unregistered = [...invoked].filter((command) => !registered.has(command)).sort();

const dist = path.join(repository, "apps", "desktop", "dist");
const publicFiles = walk(dist);
const maps = publicFiles.filter((filename) => filename.endsWith(".map"));
const oversizedLogo = publicFiles.filter((filename) => path.basename(filename) === "sabicheiro-logo.png");
const bundledText = publicFiles
  .filter((filename) => /\.(?:js|css|html)$/.test(filename))
  .map((filename) => fs.readFileSync(filename, "utf8"))
  .join("\n");
const previewLeaks = ["Development preview", "preview-session-", "preview-only"].filter((marker) => bundledText.includes(marker));
const localPathLeak = /[A-Za-z]:\\Users\\|\/home\/[A-Za-z0-9_-]+\//.test(bundledText);

const errors = [];
if (unreachable.length) errors.push(`unreachable frontend modules:\n  ${unreachable.map((file) => path.relative(repository, file)).join("\n  ")}`);
if (unusedCssClasses.length) errors.push(`CSS classes without a production consumer: ${unusedCssClasses.join(", ")}`);
if (unconsumed.length) errors.push(`registered Tauri commands without a production consumer: ${unconsumed.join(", ")}`);
if (unregistered.length) errors.push(`production adapter calls unregistered commands: ${unregistered.join(", ")}`);
if (unusedAdapterMethods.length) errors.push(`production adapter methods without a UI consumer: ${unusedAdapterMethods.join(", ")}`);
if (maps.length) errors.push(`public source maps: ${maps.map((file) => path.relative(dist, file)).join(", ")}`);
if (oversizedLogo.length) errors.push("legacy oversized logo is still in the public bundle");
if (previewLeaks.length) errors.push(`preview data leaked into production: ${previewLeaks.join(", ")}`);
if (localPathLeak) errors.push("a local absolute path leaked into the public bundle");

if (errors.length) {
  console.error(`Frontend/product audit failed:\n${errors.map((error) => `- ${error}`).join("\n")}`);
  process.exitCode = 1;
} else {
  console.log(`Frontend/product audit passed: ${reachable.size} reachable modules, ${registered.size} documented IPC commands, no maps/preview/local paths in dist.`);
}
