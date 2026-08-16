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

export interface QueryTerm {
  field: string | null;
  value: string;
  negated: boolean;
}

export type QueryExpression =
  | { kind: "term"; term: QueryTerm }
  | { kind: "and"; items: QueryExpression[] }
  | { kind: "or"; items: QueryExpression[] }
  | { kind: "not"; item: QueryExpression };

export interface ParsedQuery {
  terms: QueryTerm[];
  error: string | null;
  expression: QueryExpression | null;
}

interface QueryLexeme {
  kind: "word" | "and" | "or" | "not" | "left" | "right";
  value: string;
  position: number;
}

function tokenizeQuery(query: string): {
  tokens: QueryLexeme[];
  error: string | null;
} {
  const tokens: QueryLexeme[] = [];
  let current = "";
  let currentPosition = 0;
  let quoted = false;
  let wordHadQuote = false;
  const push = (position: number) => {
    if (!current) return;
    const upper = current.toUpperCase();
    const kind = !wordHadQuote && ["AND", "OR", "NOT"].includes(upper)
      ? (upper.toLowerCase() as "and" | "or" | "not")
      : "word";
    tokens.push({ kind, value: current, position: currentPosition });
    current = "";
    currentPosition = position;
    wordHadQuote = false;
  };
  for (let position = 0; position < query.length; position += 1) {
    const character = query[position]!;
    if (character === '"') {
      quoted = !quoted;
      wordHadQuote = true;
    } else if (!quoted && (character === "(" || character === ")")) {
      push(position);
      tokens.push({
        kind: character === "(" ? "left" : "right",
        value: character,
        position,
      });
      currentPosition = position + 1;
    } else if (/\s/.test(character) && !quoted) {
      push(position);
      currentPosition = position + 1;
    } else {
      if (!current) currentPosition = position;
      current += character;
    }
  }
  if (quoted) return { tokens, error: "Unterminated quoted value" };
  push(query.length);
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
      "windows_helper",
      "helper",
      "linux_inet_diag",
      "inet_diag",
      "netlink",
      "linux_ebpf",
      "ebpf",
      "local_dns",
      "dns",
      "local_mmdb",
      "mmdb",
      "geoip",
      "handshake_metadata",
      "handshake",
      "offline_catalog",
      "catalog",
      "user_annotation",
      "annotation",
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
  if (tokenized.error)
    return { terms: [], error: tokenized.error, expression: null };
  if (tokenized.tokens.length === 0)
    return { terms: [], error: null, expression: null };

  const parseTerm = (rawToken: string): QueryTerm => {
    const negated = rawToken.startsWith("-") || rawToken.startsWith("!");
    const token = negated ? rawToken.slice(1) : rawToken;
    if (!token) throw new Error("Missing value after negation");
    const separator = token.indexOf(":");
    if (separator < 0) {
      return { field: null, value: token.toLowerCase(), negated };
    }
    const field = token.slice(0, separator).toLowerCase();
    const value = token.slice(separator + 1);
    if (!fields.has(field)) throw new Error(`Unknown filter: ${field}`);
    if (!value) throw new Error(`Missing value for ${field}`);
    const normalized = value.toLowerCase();
    const valueError = invalidValue(field, normalized);
    if (valueError) throw new Error(`${field}: ${valueError}`);
    return { field, value: normalized, negated };
  };

  let position = 0;
  const peek = () => tokenized.tokens[position];
  const next = () => tokenized.tokens[position++];
  const parsePrimary = (): QueryExpression => {
    const token = next();
    if (!token) throw new Error(`Missing operand at position ${query.length}`);
    if (token.kind === "word") return { kind: "term", term: parseTerm(token.value) };
    if (token.kind === "not") return { kind: "not", item: parsePrimary() };
    if (token.kind === "left") {
      if (peek()?.kind === "right")
        throw new Error(`Empty group at position ${token.position}`);
      const expression = parseOr();
      if (next()?.kind !== "right")
        throw new Error(`Missing closing parenthesis at position ${token.position}`);
      return expression;
    }
    throw new Error(`Unexpected ${token.value} at position ${token.position}`);
  };
  const parseAnd = (): QueryExpression => {
    const items = [parsePrimary()];
    while (peek() && peek()?.kind !== "or" && peek()?.kind !== "right") {
      if (peek()?.kind === "and") {
        const operator = next()!;
        if (!peek() || ["and", "or", "right"].includes(peek()!.kind))
          throw new Error(`AND is missing an operand at position ${operator.position}`);
      }
      items.push(parsePrimary());
    }
    return items.length === 1 ? items[0]! : { kind: "and", items };
  };
  const parseOr = (): QueryExpression => {
    const items = [parseAnd()];
    while (peek()?.kind === "or") {
      const operator = next()!;
      if (!peek() || peek()?.kind === "right")
        throw new Error(`OR is missing an operand at position ${operator.position}`);
      items.push(parseAnd());
    }
    return items.length === 1 ? items[0]! : { kind: "or", items };
  };

  try {
    const expression = parseOr();
    if (peek())
      throw new Error(`Unexpected ${peek()!.value} at position ${peek()!.position}`);
    const terms: QueryTerm[] = [];
    const collect = (item: QueryExpression) => {
      if (item.kind === "term") terms.push(item.term);
      else if (item.kind === "not") collect(item.item);
      else item.items.forEach(collect);
    };
    collect(expression);
    return { terms, error: null, expression };
  } catch (error) {
    return {
      terms: [],
      error: error instanceof Error ? error.message : String(error),
      expression: null,
    };
  }
}

export function matchesQuery(
  connection: Connection,
  parsed: ParsedQuery,
): boolean {
  if (parsed.error) return true;
  const matchesExpression = (expression: QueryExpression): boolean => {
    if (expression.kind === "and") return expression.items.every(matchesExpression);
    if (expression.kind === "or") return expression.items.some(matchesExpression);
    if (expression.kind === "not") return !matchesExpression(expression.item);
    const { field, value, negated } = expression.term;
    const matched = matchesTerm(connection, field, value);
    return negated ? !matched : matched;
  };
  return parsed.expression ? matchesExpression(parsed.expression) : true;
}

function quoteQueryValue(value: string): string {
  return /[\s()]/.test(value) ? `"${value}"` : value;
}

export function serializeQueryTerm(term: QueryTerm): string {
  const value = quoteQueryValue(term.value);
  return `${term.negated ? "-" : ""}${term.field ? `${term.field}:` : ""}${value}`;
}

export function serializeQueryExpression(
  expression: QueryExpression,
  parentPrecedence = 0,
): string {
  const precedence =
    expression.kind === "or" ? 1 : expression.kind === "and" ? 2 : 3;
  let serialized: string;
  if (expression.kind === "term") serialized = serializeQueryTerm(expression.term);
  else if (expression.kind === "not")
    serialized = `NOT (${serializeQueryExpression(expression.item)})`;
  else
    serialized = expression.items
      .map((item) => serializeQueryExpression(item, precedence))
      .join(expression.kind === "and" ? " AND " : " OR ");
  return precedence < parentPrecedence ? `(${serialized})` : serialized;
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
    helper: "windows_helper",
    inet_diag: "linux_inet_diag",
    netlink: "linux_inet_diag",
    ebpf: "linux_ebpf",
    dns: "local_dns",
    mmdb: "local_mmdb",
    geoip: "local_mmdb",
    handshake: "handshake_metadata",
    catalog: "offline_catalog",
    annotation: "user_annotation",
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
