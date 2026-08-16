import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repository = path.resolve(import.meta.dirname, "..");
const git = spawnSync(
  "git",
  ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
  {
  cwd: repository,
  encoding: "utf8",
  },
);
if (git.status !== 0) throw new Error(git.stderr || "git ls-files failed");
const tracked = git.stdout
  .split("\0")
  .filter(Boolean)
  .filter((filename) => fs.existsSync(path.join(repository, filename)));

const forbiddenPaths = [
  /(^|\/)(android|ios)(\/|$)/i,
  /\.xcodeproj(\/|$)|\.xcworkspace(\/|$)|AppIcon\.appiconset/i,
  /(^|\/)AndroidManifest\.xml$|(^|\/)build\.gradle(?:\.kts)?$/i,
  /(^|\/)gradlew(?:\.bat)?$/i,
  /apps\/desktop\/src-tauri\/gen\//,
  /(^|\/)(target|node_modules|dist|release)(\/|$)/,
  /\.(?:apk|aab|ipa|app|dmg|icns|exe|msi|deb|appimage|mmdb|pcapng?|etl)$/i,
];
const forbidden = tracked.filter((filename) =>
  forbiddenPaths.some((pattern) => pattern.test(filename)),
);

const rootCargo = fs.readFileSync(path.join(repository, "Cargo.toml"), "utf8");
const cargoVersion = rootCargo.match(/\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"/)?.[1];
const packageJson = JSON.parse(
  fs.readFileSync(path.join(repository, "apps/desktop/package.json"), "utf8"),
);
const tauriConfig = JSON.parse(
  fs.readFileSync(
    path.join(repository, "apps/desktop/src-tauri/tauri.conf.json"),
    "utf8",
  ),
);
const versions = new Set([cargoVersion, packageJson.version, tauriConfig.version]);

const desktopCargo = fs.readFileSync(
  path.join(repository, "apps/desktop/src-tauri/Cargo.toml"),
  "utf8",
);
const crateTypes = desktopCargo.match(/crate-type\s*=\s*\[([^\]]+)\]/)?.[1] ?? "";
const desktopOnlyLibrary = /"rlib"/.test(crateTypes) && !/"(?:staticlib|cdylib)"/.test(crateTypes);

const bundleTargets = tauriConfig.bundle?.targets ?? [];
const expectedTargets = ["appimage", "deb", "nsis"];
const desktopTargetsOnly =
  JSON.stringify([...bundleTargets].sort()) === JSON.stringify(expectedTargets);

const textExtensions = new Set([
  "",
  ".css",
  ".html",
  ".json",
  ".md",
  ".mjs",
  ".rs",
  ".toml",
  ".ts",
  ".tsx",
  ".yaml",
  ".yml",
]);
const searchable = tracked
  .filter((filename) => textExtensions.has(path.extname(filename).toLowerCase()))
  .map((filename) => ({
    filename,
    content: fs.readFileSync(path.join(repository, filename), "utf8"),
  }));
const assets = tracked.filter((filename) =>
  /apps\/desktop\/(?:public|src-tauri\/icons)\//.test(filename),
);
const unreferencedAssets = assets.filter((asset) => {
  const basename = path.posix.basename(asset);
  return !searchable.some(
    ({ filename, content }) => filename !== asset && content.includes(basename),
  );
});

const directManifestText = [
  desktopCargo,
  ...tracked
    .filter((filename) => /(?:^|\/)Cargo\.toml$/.test(filename))
    .map((filename) => fs.readFileSync(path.join(repository, filename), "utf8")),
  JSON.stringify(packageJson),
].join("\n");
const prohibitedServiceDependencies = [
  "amplitude",
  "appcenter",
  "datadog",
  "firebase",
  "mixpanel",
  "newrelic",
  "posthog",
  "segment",
  "sentry",
].filter((name) => new RegExp(`(?:^|[\"'\\s])${name}(?:[\"'\\s=/-]|$)`, "im").test(directManifestText));

const errors = [];
if (forbidden.length) errors.push(`forbidden tracked paths:\n  ${forbidden.join("\n  ")}`);
if (versions.size !== 1 || versions.has(undefined)) {
  errors.push(`version mismatch: Cargo=${cargoVersion}, package=${packageJson.version}, Tauri=${tauriConfig.version}`);
}
if (!desktopOnlyLibrary) errors.push(`desktop library has mobile-oriented crate types: [${crateTypes}]`);
if (!desktopTargetsOnly) errors.push(`unexpected bundle targets: ${bundleTargets.join(", ")}`);
if (unreferencedAssets.length) errors.push(`unreferenced assets:\n  ${unreferencedAssets.join("\n  ")}`);
if (prohibitedServiceDependencies.length) {
  errors.push(`prohibited analytics/hosted-service dependencies: ${prohibitedServiceDependencies.join(", ")}`);
}

if (errors.length) {
  console.error(`Repository policy failed:\n${errors.map((error) => `- ${error}`).join("\n")}`);
  process.exitCode = 1;
} else {
  console.log(
    `Repository policy passed: ${tracked.length} tracked files, desktop targets ${bundleTargets.join("/")}, version ${cargoVersion}, ${assets.length} referenced assets, no paid/analytics service dependency.`,
  );
}
