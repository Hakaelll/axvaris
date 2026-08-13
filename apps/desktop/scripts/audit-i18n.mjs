import fs from "node:fs";
import path from "node:path";

const sourceRoot = path.resolve("src");
const sourceFiles = [];

function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(absolute);
    else if (/\.tsx?$/.test(entry.name)) sourceFiles.push(absolute);
  }
}

walk(sourceRoot);

const used = new Set();
for (const filename of sourceFiles) {
  const source = fs.readFileSync(filename, "utf8");
  const literalCall = /\bt\(\s*("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\$])*`)/g;
  for (const match of source.matchAll(literalCall)) {
    const literal = match[1];
    if (literal.startsWith('"')) used.add(JSON.parse(literal));
    else used.add(literal.slice(1, -1).replaceAll(`\\${literal[0]}`, literal[0]));
  }
}

const appSource = fs.readFileSync(path.join(sourceRoot, "App.tsx"), "utf8");
const failureMessages = appSource.match(
  /const localizedFailureMessages:[^=]+=\s*\{([\s\S]*?)\n\};/,
)?.[1];
if (!failureMessages) throw new Error("Could not locate localized failure messages");
for (const match of failureMessages.matchAll(/:\s*("(?:\\.|[^"\\])*")/g)) {
  used.add(JSON.parse(match[1]));
}

for (const key of [
  "pass",
  "warn",
  "fail",
  "healthy",
  "degraded",
  "unavailable",
  "disabled",
  "queued",
  "running",
  "completed",
  "cancelled",
  "failed",
  "capture",
  "geoip",
  "dns",
  "storage",
  "render",
  "backpressure",
  "remote_peer",
  "local_peer",
  "listener",
  "confirmed",
  "correlated",
  "approximate",
  "windows_ip_helper",
  "windows_etw",
  "linux_inet_diag",
  "local_dns",
  "local_mmdb",
  "demo",
  "historical",
  "global",
  "private",
  "loopback",
  "link_local",
  "multicast",
  "cgnat",
  "documentation",
  "benchmark",
  "reserved",
  "broadcast",
  "unspecified",
  "country",
  "region",
  "city",
  "coordinates",
  "database_coordinate",
  "country_centroid",
  "not_plotted",
  "interface_aggregate",
  "etw_flow",
  "incomplete_record",
  "historical_record",
  "listener_state",
  "missing_remote_peer",
  "connected_global_peer",
  "connected_non_global_peer",
  "orphan_etw_event",
  "ip_helper_etw_correlation",
  "geo_ip",
  "dns_cache",
  "logs",
  "all",
  "Capture is stopped",
  "SQLite quick_check passed",
  "Process instance observed",
  "Process instance was not available",
  "Connection opened",
  "Local DNS name correlated",
  "Estimated network location enriched",
  "Connection closed",
]) {
  used.add(key);
}

const translationsSource = fs.readFileSync(
  path.join(sourceRoot, "i18n.ts"),
  "utf8",
);
const objectBody = translationsSource.match(
  /const spanish:[^=]+=\s*\{([\s\S]*?)\n\};/,
)?.[1];
if (!objectBody) throw new Error("Could not locate the Spanish translation object");
const translated = new Set();
const property = /^\s*("(?:\\.|[^"\\])*"|[A-Za-z_$][\w$]*)\s*:/gm;
for (const match of objectBody.matchAll(property)) {
  translated.add(match[1].startsWith('"') ? JSON.parse(match[1]) : match[1]);
}

const missing = [...used]
  .filter((key) => !translated.has(key))
  .sort((left, right) => left.localeCompare(right));

if (missing.length > 0) {
  console.error(`Missing ${missing.length} Spanish UI translations:`);
  for (const key of missing) console.error(`- ${JSON.stringify(key)}`);
  process.exitCode = 1;
} else {
  console.log(`Spanish UI translation audit passed (${used.size} literal keys).`);
}
