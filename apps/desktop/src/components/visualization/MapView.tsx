import { memo, useMemo } from "react";
import { useI18n } from "../../i18n";
import type { Connection } from "../../types";
import { groupVisualRoutes, routeLevel } from "../../lib/routes";
import { detailedWorldBorders, mapPoint } from "../../lib/world-detail";

interface MapViewProps {
  connections: Connection[];
  home: [number, number];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function borderPath(line: [number, number][]): string {
  return line
    .map(([longitude, latitude], index) => {
      const [x, y] = mapPoint(longitude, latitude);
      return `${index === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export const MapView = memo(function MapView({
  connections,
  home,
  selectedId,
  onSelect,
}: MapViewProps) {
  const t = useI18n();
  const borders = useMemo(() => detailedWorldBorders().map(borderPath), []);
  const arcs = useMemo(
    () =>
      groupVisualRoutes(
        connections.filter((connection) => connection.geo !== null),
        routeLevel(2.4, connections.length),
      )
        .slice(0, 180)
        .map((route) => {
          const connection = route.representative;
          const [startX, startY] = mapPoint(home[1], home[0]);
          const [endX, endY] = mapPoint(route.longitude, route.latitude);
          const distance = Math.hypot(endX - startX, endY - startY);
          const controlX = (startX + endX) / 2;
          const controlY = Math.max(
            12,
            (startY + endY) / 2 - Math.min(105, distance * 0.24),
          );
          return {
            route,
            connection,
            startX,
            startY,
            endX,
            endY,
            controlX,
            controlY,
          };
        }),
    [connections, home],
  );

  return (
    <div className="world-map" data-testid="map-view">
      <svg
        aria-label={t("World map showing filtered network destinations")}
        className="world-map__svg"
        preserveAspectRatio="xMidYMid meet"
        role="img"
        viewBox="0 0 1000 500"
      >
        <rect className="world-map__ocean" height="500" width="1000" />
        <g aria-hidden="true" className="world-map__grid">
          {[-120, -60, 0, 60, 120].map((longitude) => {
            const [x] = mapPoint(longitude, 0);
            return (
              <line key={`lon-${longitude}`} x1={x} x2={x} y1="0" y2="500" />
            );
          })}
          {[-60, -30, 0, 30, 60].map((latitude) => {
            const [, y] = mapPoint(0, latitude);
            return (
              <line key={`lat-${latitude}`} x1="0" x2="1000" y1={y} y2={y} />
            );
          })}
        </g>
        <g aria-hidden="true" className="world-map__borders">
          {borders.map((path, index) => (
            <path d={path} key={index} />
          ))}
        </g>
        <g className="world-map__arcs">
          {arcs.map(
            ({
              route,
              connection,
              startX,
              startY,
              endX,
              endY,
              controlX,
              controlY,
            }) => {
              const selected =
                selectedId !== null && route.memberIds.includes(selectedId);
              const selectionTarget =
                selectedId && route.memberIds.includes(selectedId)
                  ? selectedId
                  : connection.id;
              return (
                <g
                  className={selected ? "is-selected" : undefined}
                  key={route.key}
                  onClick={() => onSelect(selectionTarget)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ")
                      onSelect(selectionTarget);
                  }}
                >
                  <title>
                    {t(
                      route.count === 1
                        ? "{count} connection to {destination}"
                        : "{count} connections to {destination}",
                      {
                        count: route.count,
                        destination:
                          connection.geo?.country_name ??
                          connection.domain ??
                          connection.key.remote.address,
                      },
                    )}
                  </title>
                  <path
                    className="world-map__arc-hit"
                    d={`M${startX},${startY} Q${controlX},${controlY} ${endX},${endY}`}
                  />
                  <path
                    className="world-map__arc"
                    d={`M${startX},${startY} Q${controlX},${controlY} ${endX},${endY}`}
                  />
                  <circle
                    className="world-map__endpoint"
                    cx={endX}
                    cy={endY}
                    r={
                      selected
                        ? 5
                        : Math.min(5, 2.5 + Math.log2(route.count + 1) * 0.45)
                    }
                  />
                </g>
              );
            },
          )}
        </g>
        {(() => {
          const [x, y] = mapPoint(home[1], home[0]);
          return (
            <g aria-label={t("Approximate home location")}>
              <circle className="world-map__home-ring" cx={x} cy={y} r="9" />
              <circle className="world-map__home" cx={x} cy={y} r="3" />
            </g>
          );
        })()}
      </svg>
    </div>
  );
});
