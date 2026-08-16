import type {
  AddressFamily,
  AddressScope,
  FirewallEvidence,
  ListenerExposure,
  ListenerScope,
  ListenerSnapshot,
  NetworkProtocol,
} from "../types";

const protocols = new Set<NetworkProtocol>(["tcp", "udp"]);
const listenerScopes = new Set<ListenerScope>([
  "loopback",
  "lan",
  "vpn",
  "wildcard",
  "unknown",
]);
const firewallStates = new Set<FirewallEvidence>([
  "allowed",
  "blocked",
  "ambiguous",
  "not_queryable",
]);
const addressFamilies = new Set<AddressFamily>(["ipv4", "ipv6"]);
const addressScopes = new Set<AddressScope>([
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
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isMember<T extends string>(
  values: ReadonlySet<T>,
  value: unknown,
): value is T {
  return typeof value === "string" && values.has(value as T);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isDateTimestamp(value: unknown): value is number {
  return isFiniteNumber(value) && value >= 0 && value <= 8_640_000_000_000_000;
}

function normalizeListener(value: unknown): ListenerExposure | null {
  if (!isRecord(value) || !isRecord(value.endpoint)) return null;
  const endpoint = value.endpoint;
  if (
    typeof endpoint.address !== "string" ||
    endpoint.address.length === 0 ||
    !Number.isInteger(endpoint.port) ||
    (endpoint.port as number) < 0 ||
    (endpoint.port as number) > 65_535 ||
    !isMember(addressFamilies, endpoint.observed_family) ||
    !isMember(addressScopes, endpoint.scope) ||
    (endpoint.zone !== null && typeof endpoint.zone !== "string") ||
    !isMember(protocols, value.protocol) ||
    (value.process_instance !== null &&
      typeof value.process_instance !== "string") ||
    !isMember(listenerScopes, value.scope) ||
    !isMember(firewallStates, value.firewall) ||
    !isFiniteNumber(value.first_observed_ms) ||
    !isFiniteNumber(value.last_observed_ms) ||
    typeof value.limitation !== "string"
  ) {
    return null;
  }

  return {
    endpoint: {
      address: endpoint.address,
      port: endpoint.port as number,
      observed_family: endpoint.observed_family,
      scope: endpoint.scope,
      zone: endpoint.zone,
    },
    protocol: value.protocol,
    process_instance: value.process_instance,
    scope: value.scope,
    firewall: value.firewall,
    first_observed_ms: value.first_observed_ms,
    last_observed_ms: value.last_observed_ms,
    limitation: value.limitation,
  };
}

export function listenerKey(listener: ListenerExposure): string {
  return [
    listener.protocol,
    listener.endpoint.address,
    listener.endpoint.port,
    listener.process_instance ?? "unknown",
  ].join("\u0000");
}

export function formatListenerEndpoint(listener: ListenerExposure): string {
  const { address, port } = listener.endpoint;
  return address.includes(":") ? `[${address}]:${port}` : `${address}:${port}`;
}

export interface NormalizedListenerSnapshots {
  snapshots: ListenerSnapshot[];
  rejectedListeners: number;
  rejectedSnapshots: number;
}

export function normalizeListenerSnapshots(
  value: unknown,
): NormalizedListenerSnapshots {
  if (!Array.isArray(value)) {
    throw new TypeError("Listener snapshot response is not an array");
  }

  let rejectedListeners = 0;
  let rejectedSnapshots = 0;
  const snapshots: ListenerSnapshot[] = [];

  for (const candidate of value) {
    if (
      !isRecord(candidate) ||
      (candidate.id !== null && !Number.isInteger(candidate.id)) ||
      (candidate.session_id !== null &&
        typeof candidate.session_id !== "string") ||
      !isDateTimestamp(candidate.observed_at_ms) ||
      typeof candidate.snapshot_sha256 !== "string"
    ) {
      rejectedSnapshots += 1;
      continue;
    }

    const rawListeners = Array.isArray(candidate.listeners)
      ? candidate.listeners
      : [];
    if (!Array.isArray(candidate.listeners)) rejectedListeners += 1;
    const seen = new Set<string>();
    const listeners: ListenerExposure[] = [];
    for (const rawListener of rawListeners) {
      const listener = normalizeListener(rawListener);
      if (!listener) {
        rejectedListeners += 1;
        continue;
      }
      const key = listenerKey(listener);
      if (seen.has(key)) continue;
      seen.add(key);
      listeners.push(listener);
    }

    snapshots.push({
      id: candidate.id as number | null,
      session_id: candidate.session_id as string | null,
      observed_at_ms: candidate.observed_at_ms,
      snapshot_sha256: candidate.snapshot_sha256,
      listeners,
    });
  }

  snapshots.sort(
    (left, right) =>
      right.observed_at_ms - left.observed_at_ms ||
      (right.id ?? -1) - (left.id ?? -1),
  );

  return { snapshots, rejectedListeners, rejectedSnapshots };
}

export interface ListenerComparison {
  added: ListenerExposure[];
  current: ListenerExposure[];
  removed: ListenerExposure[];
  unchanged: number;
}

export function compareListenerSnapshots(
  latest: ListenerSnapshot | null,
  previous: ListenerSnapshot | null,
): ListenerComparison {
  const earlier = new Map(
    (previous?.listeners ?? []).map((listener) => [
      listenerKey(listener),
      listener,
    ]),
  );
  const current = new Map(
    (latest?.listeners ?? []).map((listener) => [
      listenerKey(listener),
      listener,
    ]),
  );
  const addedKeys = new Set(
    [...current.keys()].filter((key) => !earlier.has(key)),
  );

  return {
    added: [...current.entries()]
      .filter(([key]) => addedKeys.has(key))
      .map(([, listener]) => listener),
    current: [...current.entries()]
      .filter(([key]) => !addedKeys.has(key))
      .map(([, listener]) => listener),
    removed: [...earlier.entries()]
      .filter(([key]) => !current.has(key))
      .map(([, listener]) => listener),
    unchanged: [...current.keys()].filter((key) => earlier.has(key)).length,
  };
}
