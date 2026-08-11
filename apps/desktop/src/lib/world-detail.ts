import { mesh } from "topojson-client";
import world from "world-atlas/countries-50m.json";
import type { GeometryCollection } from "topojson-specification";
import type { CoordinateLine } from "./world";

let cachedBorders: CoordinateLine[] | null = null;

export function detailedWorldBorders(): CoordinateLine[] {
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

export function mapPoint(
  longitude: number,
  latitude: number,
): [number, number] {
  return [((longitude + 180) / 360) * 1_000, ((90 - latitude) / 180) * 500];
}
