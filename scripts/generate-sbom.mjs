import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repository = path.resolve(import.meta.dirname, "..");
const version = process.argv[2];
const outputArgument = process.argv[3];
if (!version || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error("usage: node scripts/generate-sbom.mjs <version> <output>");
}
if (!outputArgument) throw new Error("an output path is required");

const output = path.resolve(repository, outputArgument);
if (!output.startsWith(`${repository}${path.sep}`)) {
  throw new Error("the SBOM output must stay inside the repository");
}

function spdxId(ecosystem, name, packageVersion) {
  const stable = `${ecosystem}-${name}-${packageVersion}`.replace(
    /[^A-Za-z0-9.-]+/g,
    "-",
  );
  return `SPDXRef-${stable.replace(/^-+|-+$/g, "")}`;
}

function cargoPackages() {
  const result = spawnSync(
    "cargo",
    ["metadata", "--locked", "--offline", "--format-version", "1"],
    { cwd: repository, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  if (result.status !== 0) throw new Error(result.stderr || "cargo metadata failed");
  return JSON.parse(result.stdout).packages.map((item) => ({
    SPDXID: spdxId("cargo", item.name, item.version),
    name: item.name,
    versionInfo: item.version,
    downloadLocation: "NOASSERTION",
    filesAnalyzed: false,
    licenseConcluded: "NOASSERTION",
    licenseDeclared: item.license || "NOASSERTION",
    supplier: "NOASSERTION",
    externalRefs: [
      {
        referenceCategory: "PACKAGE-MANAGER",
        referenceType: "purl",
        referenceLocator: `pkg:cargo/${encodeURIComponent(item.name)}@${item.version}`,
      },
    ],
  }));
}

function addNodePackage(packages, directory) {
  const manifest = path.join(directory, "package.json");
  if (!fs.existsSync(manifest)) return;
  const item = JSON.parse(fs.readFileSync(manifest, "utf8"));
  if (!item.name || !item.version) return;
  const declared =
    typeof item.license === "string"
      ? item.license
      : typeof item.license?.type === "string"
        ? item.license.type
        : "NOASSERTION";
  packages.push({
    SPDXID: spdxId("npm", item.name, item.version),
    name: item.name,
    versionInfo: item.version,
    downloadLocation: "NOASSERTION",
    filesAnalyzed: false,
    licenseConcluded: "NOASSERTION",
    licenseDeclared: declared,
    supplier: "NOASSERTION",
    externalRefs: [
      {
        referenceCategory: "PACKAGE-MANAGER",
        referenceType: "purl",
        referenceLocator: `pkg:npm/${encodeURIComponent(item.name)}@${item.version}`,
      },
    ],
  });
}

function nodePackages() {
  const store = path.join(repository, "node_modules", ".pnpm");
  if (!fs.existsSync(store)) {
    throw new Error("node_modules/.pnpm is missing; install the frozen lockfile first");
  }
  const packages = [];
  for (const storeEntry of fs.readdirSync(store, { withFileTypes: true })) {
    if (!storeEntry.isDirectory()) continue;
    const modules = path.join(store, storeEntry.name, "node_modules");
    if (!fs.existsSync(modules)) continue;
    for (const entry of fs.readdirSync(modules, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const candidate = path.join(modules, entry.name);
      if (entry.name.startsWith("@")) {
        for (const scoped of fs.readdirSync(candidate, { withFileTypes: true })) {
          if (scoped.isDirectory()) addNodePackage(packages, path.join(candidate, scoped.name));
        }
      } else {
        addNodePackage(packages, candidate);
      }
    }
  }
  return packages;
}

const componentMap = new Map();
for (const item of [...cargoPackages(), ...nodePackages()]) {
  componentMap.set(item.SPDXID, item);
}
const components = [...componentMap.values()].sort((left, right) =>
  left.SPDXID.localeCompare(right.SPDXID),
);
const rootId = "SPDXRef-Package-Sabicheiro";
const packages = [
  {
    SPDXID: rootId,
    name: "Sabicheiro",
    versionInfo: version,
    downloadLocation: "NOASSERTION",
    filesAnalyzed: false,
    licenseConcluded: "MIT",
    licenseDeclared: "MIT",
    supplier: "Organization: Sabicheiro contributors",
  },
  ...components,
];
const created = process.env.SOURCE_DATE_EPOCH
  ? new Date(Number(process.env.SOURCE_DATE_EPOCH) * 1_000).toISOString()
  : "2026-08-14T00:00:00.000Z";
const document = {
  spdxVersion: "SPDX-2.3",
  dataLicense: "CC0-1.0",
  SPDXID: "SPDXRef-DOCUMENT",
  name: `Sabicheiro-${version}-SBOM`,
  documentNamespace: `https://sabicheiro.invalid/spdx/${version}`,
  creationInfo: {
    created,
    creators: ["Tool: scripts/generate-sbom.mjs", "Organization: Sabicheiro contributors"],
    licenseListVersion: "3.27",
  },
  documentDescribes: [rootId],
  packages,
  relationships: [
    {
      spdxElementId: "SPDXRef-DOCUMENT",
      relationshipType: "DESCRIBES",
      relatedSpdxElement: rootId,
    },
    ...components.map((item) => ({
      spdxElementId: rootId,
      relationshipType: "DEPENDS_ON",
      relatedSpdxElement: item.SPDXID,
    })),
  ],
};

fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(document, null, 2)}\n`, "utf8");
console.log(
  `SPDX 2.3 SBOM written to ${path.relative(repository, output)} with ${packages.length} packages.`,
);
