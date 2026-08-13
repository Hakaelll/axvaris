import type { Connection } from "../types";

const fields = new Set([
  "process",
  "country",
  "protocol",
  "port",
  "direction",
  "ip",
  "domain",
  "state",
  "firstseen",
  "asn",
  "cidr",
  "scope",
  "geo",
  "source",
]);

interface QueryTerm {
  field: string | null;
  value: string;
  negated: boolean;
}

export interface ParsedQuery {
  terms: QueryTerm[];
  error: string | null;
}

function tokenizeQuery(query: string): {
  tokens: string[];
  error: string | null;
} {
  const tokens: string[] = [];
  let current = "";
  let quoted = false;
  for (const character of query) {
    if (character === '"') {
      quoted = !quoted;
    } else if (/\s/.test(character) && !quoted) {
      if (current) tokens.push(current);
      current = "";
    } else {
      current += character;
    }
  }
  if (quoted) return { tokens, error: "Unterminated quoted value" };
  if (current) tokens.push(current);
  return { tokens, error: null };
}

function invalidValue(field: string, value: string): string | null {
  if (field === "protocol" && !["tcp", "udp"].includes(value))
    return "Use tcp or udp";
  if (field === "port" && (!/^\d+$/.test(value) || Number(value) > 65_535))
    return "Port must be between 0 and 65535";
  if (
    field === "direction" &&
    !["in", "inbound", "out", "outbound", "unknown"].includes(value)
  )
    return "Use in, out or unknown";
  if (
    field === "ip" &&
    !(/^\d{1,3}(?:\.\d{1,3}){3}$/.test(value) || value.includes(":"))
  )
    return "Enter an IPv4 or IPv6 address";
  if (
    field === "state" &&
    ![
      "listen",
      "listening",
      "connecting",
      "active",
      "established",
      "closing",
      "closed",
      "unknown",
    ].includes(value)
  )
    return "Unknown connection state";
  if (
    field === "firstseen" &&
    !["true", "yes", "1", "false", "no", "0"].includes(value)
  )
    return "Use true or false";
  if (field === "asn" && !/^\d+$/.test(value)) return "ASN must be a number";
  if (field === "cidr" && !parseCidr(value))
    return "Enter a valid IPv4 or IPv6 CIDR prefix";
  if (
    field === "scope" &&
    ![
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
    ].includes(value)
  )
    return "Unknown address scope";
  if (
    field === "source" &&
    ![
      "windows_ip_helper",
      "ip_helper",
      "windows_etw",
      "etw",
      "linux_inet_diag",
      "inet_diag",
      "netlink",
      "local_dns",
      "dns",
      "local_mmdb",
      "mmdb",
      "geoip",
      "demo",
      "historical",
      "history",
      "unknown",
    ].includes(value)
  )
    return "Unknown evidence source";
  return null;
}

export function parseQuery(query: string): ParsedQuery {
  const tokenized = tokenizeQuery(query);
  if (tokenized.error) return { terms: [], error: tokenized.error };
  const terms: QueryTerm[] = [];
  for (const rawToken of tokenized.tokens) {
    const negated = rawToken.startsWith("-") || rawToken.startsWith("!");
    const token = negated ? rawToken.slice(1) : rawToken;
    if (!token) return { terms, error: "Missing value after negation" };
    const separator = token.indexOf(":");
    if (separator < 0) {
      terms.push({ field: null, value: token.toLowerCase(), negated });
      continue;
    }
    const field = token.slice(0, separator).toLowerCase();
    const value = token.slice(separator + 1);
    if (!fields.has(field)) return { terms, error: `Unknown filter: ${field}` };
    if (!value) return { terms, error: `Missing value for ${field}` };
    const normalized = value.toLowerCase();
    const valueError = invalidValue(field, normalized);
    if (valueError) return { terms, error: `${field}: ${valueError}` };
    terms.push({ field, value: normalized, negated });
  }
  return { terms, error: null };
}

export function matchesQuery(
  connection: Connection,
  parsed: ParsedQuery,
): boolean {
  if (parsed.error) return true;
  return parsed.terms.every(({ field, value, negated }) => {
    const matched = matchesTerm(connection, field, value);
    return negated ? !matched : matched;
  });
}

function matchesTerm(
  connection: Connection,
  field: string | null,
  value: string,
): boolean {
    const domain = (
      connection.domain ??
      connection.hostname ??
      ""
    ).toLowerCase();
    const countryCode = connection.geo?.country_code?.toLowerCase() ?? "";
    const countryName = connection.geo?.country_name?.toLowerCase() ?? "";
    switch (field) {
      case "process":
        return connection.process.name.toLowerCase().includes(value);
      case "country":
        return countryCode === value || countryName.includes(value);
      case "protocol":
        return connection.protocol === value;
      case "port":
        return (
          connection.key.local.port === Number(value) ||
          connection.key.remote.port === Number(value)
        );
      case "direction":
        return connection.direction.startsWith(value);
      case "ip":
        return (
          connection.key.local.address === value ||
          connection.key.remote.address === value
        );
      case "domain":
        return domain.includes(value);
      case "state":
        if (value === "active" || value === "established")
          return connection.state === "established";
        if (value === "listen") return connection.state === "listening";
        return connection.state === value;
      case "firstseen":
        return (
          connection.first_seen_destination ===
          ["true", "yes", "1"].includes(value)
        );
      case "asn":
        return connection.asn?.number === Number(value);
      case "cidr": {
        const cidr = parseCidr(value);
        return cidr
          ? cidrContains(cidr, connection.key.local.address) ||
              cidrContains(cidr, connection.key.remote.address)
          : false;
      }
      case "scope":
        return connection.key.remote.scope === value;
      case "geo":
        return [
          countryCode,
          countryName,
          connection.geo?.region ?? "",
          connection.geo?.city ?? "",
        ]
          .join(" ")
          .toLowerCase()
          .includes(value);
      case "source":
        return connection.evidence.observation_source === normalizeSource(value);
      default:
        return [
          connection.process.name,
          connection.key.local.address,
          connection.key.remote.address,
          domain,
          countryCode,
          countryName,
          connection.asn?.organization ?? "",
        ]
          .join(" ")
          .toLowerCase()
          .includes(value);
    }
}

function normalizeSource(value: string): string {
  const aliases: Record<string, string> = {
    ip_helper: "windows_ip_helper",
    etw: "windows_etw",
    inet_diag: "linux_inet_diag",
    netlink: "linux_inet_diag",
    dns: "local_dns",
    mmdb: "local_mmdb",
    geoip: "local_mmdb",
    history: "historical",
  };
  return aliases[value] ?? value;
}

interface ParsedCidr {
  value: bigint;
  prefix: number;
  bits: 32 | 128;
}

function parseIp(address: string): { value: bigint; bits: 32 | 128 } | null {
  if (address.includes(".")) {
    const parts = address.split(".");
    if (
      parts.length !== 4 ||
      parts.some(
        (part) => !/^\d{1,3}$/.test(part) || Number(part) > 255,
      )
    )
      return null;
    return {
      value: parts.reduce(
        (total, part) => (total << 8n) | BigInt(Number(part)),
        0n,
      ),
      bits: 32,
    };
  }
  const halves = address.toLowerCase().split("::");
  if (halves.length > 2) return null;
  const left = halves[0] ? halves[0].split(":") : [];
  const right = halves.length === 2 && halves[1] ? halves[1].split(":") : [];
  if (halves.length === 1 && left.length !== 8) return null;
  const missing = 8 - left.length - right.length;
  if (missing < 0 || (halves.length === 2 && missing < 1)) return null;
  const groups = [...left, ...Array(missing).fill("0"), ...right];
  if (
    groups.length !== 8 ||
    groups.some((group) => !/^[0-9a-f]{1,4}$/.test(group))
  )
    return null;
  return {
    value: groups.reduce(
      (total, group) => (total << 16n) | BigInt(`0x${group}`),
      0n,
    ),
    bits: 128,
  };
}

function parseCidr(value: string): ParsedCidr | null {
  const [address, rawPrefix, extra] = value.split("/");
  if (!address || rawPrefix === undefined || extra !== undefined) return null;
  const parsed = parseIp(address);
  if (!parsed || !/^\d+$/.test(rawPrefix)) return null;
  const prefix = Number(rawPrefix);
  if (prefix < 0 || prefix > parsed.bits) return null;
  return { ...parsed, prefix };
}

function cidrContains(cidr: ParsedCidr, address: string): boolean {
  const parsed = parseIp(address);
  if (!parsed || parsed.bits !== cidr.bits) return false;
  const shift = BigInt(cidr.bits - cidr.prefix);
  return shift === BigInt(cidr.bits)
    ? true
    : parsed.value >> shift === cidr.value >> shift;
}

export function isLocalAddress(address: string): boolean {
  return (
    address === "0.0.0.0" ||
    address === "::" ||
    address === "::1" ||
    address.startsWith("127.") ||
    address.startsWith("10.") ||
    address.startsWith("192.168.") ||
    /^172\.(1[6-9]|2\d|3[01])\./.test(address) ||
    address.toLowerCase().startsWith("fc") ||
    address.toLowerCase().startsWith("fd") ||
    address.toLowerCase().startsWith("fe80:")
  );
}

export function isListenerOnly(connection: Connection): boolean {
  return (
    connection.state === "listening" ||
    ((connection.key.remote.address === "0.0.0.0" ||
      connection.key.remote.address === "::") &&
      connection.key.remote.port === 0)
  );
}
