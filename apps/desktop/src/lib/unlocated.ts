import type { Connection } from "../types";

export type UnlocatedReason =
  | "geoip_absent"
  | "local_or_reserved"
  | "low_precision"
  | "no_coordinates"
  | "pending";

export interface UnlocatedTarget {
  connection: Connection;
  reason: UnlocatedReason;
}

export function hasPlottableCoordinates(connection: Connection): boolean {
  const latitude = connection.geo?.latitude;
  const longitude = connection.geo?.longitude;
  return (
    connection.geo?.placement !== "not_plotted" &&
    typeof latitude === "number" &&
    Number.isFinite(latitude) &&
    latitude >= -90 &&
    latitude <= 90 &&
    typeof longitude === "number" &&
    Number.isFinite(longitude) &&
    longitude >= -180 &&
    longitude <= 180
  );
}

export function unlocatedReason(
  connection: Connection,
  nowMs: number,
  captureActive: boolean,
): UnlocatedReason | null {
  if (hasPlottableCoordinates(connection)) return null;
  if (connection.key.remote.scope !== "global") return "local_or_reserved";
  if (!connection.geo) {
    return captureActive && nowMs - connection.last_seen_ms < 5_000
      ? "pending"
      : "geoip_absent";
  }
  if (
    connection.geo.precision === "unknown" ||
    connection.geo.precision === "country" ||
    connection.geo.precision === "region"
  ) {
    return "low_precision";
  }
  return "no_coordinates";
}

export function classifyUnlocatedTargets(
  connections: Connection[],
  nowMs: number,
  captureActive: boolean,
): UnlocatedTarget[] {
  return connections.flatMap((connection) => {
    const reason = unlocatedReason(connection, nowMs, captureActive);
    return reason ? [{ connection, reason }] : [];
  });
}
