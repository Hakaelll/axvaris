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
]);

interface QueryTerm {
  field: string | null;
  value: string;
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
  return null;
}

export function parseQuery(query: string): ParsedQuery {
  const tokenized = tokenizeQuery(query);
  if (tokenized.error) return { terms: [], error: tokenized.error };
  const terms: QueryTerm[] = [];
  for (const token of tokenized.tokens) {
    const separator = token.indexOf(":");
    if (separator < 0) {
      terms.push({ field: null, value: token.toLowerCase() });
      continue;
    }
    const field = token.slice(0, separator).toLowerCase();
    const value = token.slice(separator + 1);
    if (!fields.has(field)) return { terms, error: `Unknown filter: ${field}` };
    if (!value) return { terms, error: `Missing value for ${field}` };
    const normalized = value.toLowerCase();
    const valueError = invalidValue(field, normalized);
    if (valueError) return { terms, error: `${field}: ${valueError}` };
    terms.push({ field, value: normalized });
  }
  return { terms, error: null };
}

export function matchesQuery(
  connection: Connection,
  parsed: ParsedQuery,
): boolean {
  if (parsed.error) return true;
  return parsed.terms.every(({ field, value }) => {
    const domain = (
      connection.domain ??
      connection.hostname ??
      ""
    ).toLowerCase();
    const countryCode = connection.geo?.country_code.toLowerCase() ?? "";
    const countryName = connection.geo?.country_name.toLowerCase() ?? "";
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
  });
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
