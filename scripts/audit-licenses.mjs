import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repository = path.resolve(import.meta.dirname, "..");
const prohibitedLicense = /\b(?:commercial|proprietary|unlicensed)\b|see license/i;

function isDirectory(directory) {
  try {
    return fs.statSync(directory).isDirectory();
  } catch {
    return false;
  }
}

function normalizedLicense(value) {
  if (typeof value === "string") return value.trim();
  if (value && typeof value.type === "string") return value.type.trim();
  if (Array.isArray(value))
    return value.map(normalizedLicense).filter(Boolean).join(" OR ");
  return "";
}

function cargoPackages() {
  const result = spawnSync(
    "cargo",
    ["metadata", "--locked", "--offline", "--format-version", "1"],
    { cwd: repository, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  if (result.status !== 0)
    throw new Error(result.stderr || "cargo metadata failed");
  return JSON.parse(result.stdout).packages.map((item) => ({
    ecosystem: "Cargo",
    name: item.name,
    version: item.version,
    license: normalizedLicense(item.license),
  }));
}

function addNodePackage(packages, directory) {
  const manifest = path.join(directory, "package.json");
  if (!fs.existsSync(manifest)) return;
  const item = JSON.parse(fs.readFileSync(manifest, "utf8"));
  packages.push({
    ecosystem: "pnpm",
    name: item.name ?? path.basename(directory),
    version: item.version ?? "unknown",
    license: normalizedLicense(item.license ?? item.licenses),
  });
}

function nodePackages() {
  const store = path.join(repository, "node_modules", ".pnpm");
  if (!isDirectory(store))
    throw new Error("node_modules/.pnpm is missing; run pnpm install --frozen-lockfile");
  const packages = [];
  for (const storeEntry of fs.readdirSync(store, { withFileTypes: true })) {
    if (!storeEntry.isDirectory()) continue;
    const modules = path.join(store, storeEntry.name, "node_modules");
    if (!isDirectory(modules)) continue;
    for (const entry of fs.readdirSync(modules, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const candidate = path.join(modules, entry.name);
      if (entry.name.startsWith("@")) {
        for (const scoped of fs.readdirSync(candidate, { withFileTypes: true }))
          if (scoped.isDirectory())
            addNodePackage(packages, path.join(candidate, scoped.name));
      } else {
        addNodePackage(packages, candidate);
      }
    }
  }
  return packages;
}

const packages = [...cargoPackages(), ...nodePackages()];
const unique = [
  ...new Map(
    packages.map((item) => [
      `${item.ecosystem}:${item.name}@${item.version}`,
      item,
    ]),
  ).values(),
];
const missing = unique.filter((item) => !item.license);
const prohibited = unique.filter((item) => prohibitedLicense.test(item.license));

if (missing.length || prohibited.length) {
  const details = [];
  if (missing.length)
    details.push(
      `missing license declarations:\n  ${missing.map((item) => `${item.ecosystem} ${item.name}@${item.version}`).join("\n  ")}`,
    );
  if (prohibited.length)
    details.push(
      `prohibited license declarations:\n  ${prohibited.map((item) => `${item.ecosystem} ${item.name}@${item.version}: ${item.license}`).join("\n  ")}`,
    );
  console.error(`License audit failed:\n- ${details.join("\n- ")}`);
  process.exitCode = 1;
} else {
  const cargoCount = unique.filter((item) => item.ecosystem === "Cargo").length;
  const nodeCount = unique.length - cargoCount;
  console.log(
    `Offline locked license audit passed: ${cargoCount} Cargo packages and ${nodeCount} pnpm packages; zero missing, proprietary, commercial-only or unlicensed declarations.`,
  );
}
