import { mesh } from "topojson-client";
import world from "world-atlas/countries-110m.json";
import type { GeometryCollection } from "topojson-specification";

export type CoordinateLine = [number, number][];

let cachedBorders: CoordinateLine[] | null = null;

export function worldBorders(): CoordinateLine[] {
  if (cachedBorders) return cachedBorders;
  const countries = world.objects.countries as GeometryCollection;
  const geometry = mesh(world, countries);
  cachedBorders = geometry.coordinates.map((line) =>
    line.flatMap((point) =>
      point.length >= 2 ? [[point[0]!, point[1]!] as [number, number]] : [],
    ),
  );
  return cachedBorders;
}

export function spherePoint(
  longitude: number,
  latitude: number,
  radius = 1,
): [number, number, number] {
  const phi = ((90 - latitude) * Math.PI) / 180;
  const theta = ((longitude + 180) * Math.PI) / 180;
  return [
    -radius * Math.sin(phi) * Math.cos(theta),
    radius * Math.cos(phi),
    radius * Math.sin(phi) * Math.sin(theta),
  ];
}
