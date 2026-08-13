import type { Connection } from "../types";

export type RouteLevel = "country" | "region" | "destination";

export interface VisualRoute {
  key: string;
  representative: Connection;
  memberIds: string[];
  latitude: number;
  longitude: number;
  count: number;
  active: boolean;
}

export function routeLevel(
  cameraDistance: number,
  connectionCount: number,
): RouteLevel {
  if (cameraDistance >= 3.8 || connectionCount > 90) return "country";
  if (cameraDistance >= 2.7 || connectionCount > 45) return "region";
  return "destination";
}

function groupKey(connection: Connection, level: RouteLevel): string {
  const geo = connection.geo;
  if (!geo) return `unknown:${connection.key.remote.address}`;
  if (level === "country") return `country:${geo.country_code}`;
  if (level === "region") {
    return `region:${geo.country_code}:${geo.region ?? geo.city ?? connection.key.remote.address}`;
  }
  return `destination:${connection.key.remote.address}`;
}

interface RouteAccumulator {
  representative: Connection;
  memberIds: string[];
  x: number;
  y: number;
  z: number;
  samples: number;
  count: number;
  active: boolean;
}

export function groupVisualRoutes(
  connections: Connection[],
  level: RouteLevel,
): VisualRoute[] {
  const groups = new Map<string, RouteAccumulator>();
  for (const connection of connections) {
    const geo = connection.geo;
    if (
      !geo ||
      geo.placement === "not_plotted" ||
      geo.latitude === null ||
      geo.longitude === null ||
      !Number.isFinite(geo.latitude) ||
      !Number.isFinite(geo.longitude)
    )
      continue;
    const key = groupKey(connection, level);
    const latitude = (geo.latitude * Math.PI) / 180;
    const longitude = (geo.longitude * Math.PI) / 180;
    const accumulator = groups.get(key) ?? {
      representative: connection,
      memberIds: [],
      x: 0,
      y: 0,
      z: 0,
      samples: 0,
      count: 0,
      active: false,
    };
    accumulator.memberIds.push(connection.id);
    accumulator.x += Math.cos(latitude) * Math.cos(longitude);
    accumulator.y += Math.cos(latitude) * Math.sin(longitude);
    accumulator.z += Math.sin(latitude);
    accumulator.samples += 1;
    accumulator.count += Math.max(1, connection.repeat_count);
    accumulator.active ||= connection.active;
    groups.set(key, accumulator);
  }

  return [...groups.entries()].map(([key, group]) => {
    const longitude = (Math.atan2(group.y, group.x) * 180) / Math.PI;
    const horizontal = Math.hypot(group.x, group.y);
    const latitude = (Math.atan2(group.z, horizontal) * 180) / Math.PI;
    return {
      key,
      representative: group.representative,
      memberIds: group.memberIds,
      latitude,
      longitude,
      count: group.count,
      active: group.active,
    };
  });
}
