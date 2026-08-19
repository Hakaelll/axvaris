import type { CoordinateLine } from "./world";
import { worldBorders } from "./world";

export function detailedWorldBorders(): CoordinateLine[] {
  return worldBorders();
}

export function mapPoint(
  longitude: number,
  latitude: number,
): [number, number] {
  return [((longitude + 180) / 360) * 1_000, ((90 - latitude) / 180) * 500];
}
