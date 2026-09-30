import {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent as ReactWheelEvent,
} from "react";
import { useI18n } from "../../i18n";
import type { Connection } from "../../types";
import { groupVisualRoutes, type VisualRoute } from "../../lib/routes";
import { detailedWorldBorders, mapPoint } from "../../lib/world-detail";

interface MapViewProps {
  connections: Connection[];
  home: [number, number];
  selectedId: string | null;
  onSelect: (id: string) => void;
  expanded?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
}

interface Viewport {
  zoom: number;
  centerX: number;
  centerY: number;
}

const SPAIN_CENTER = mapPoint(-3.7038, 40.4168);
const DEFAULT_VIEWPORT: Viewport = {
  zoom: 2.2,
  centerX: SPAIN_CENTER[0],
  centerY: SPAIN_CENTER[1],
};

function borderPath(line: [number, number][]): string {
  return line
    .map(([longitude, latitude], index) => {
      const [x, y] = mapPoint(longitude, latitude);
      return `${index === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

function routePaths(
  startX: number,
  startY: number,
  endX: number,
  endY: number,
): string[] {
  let wrappedEnd = endX;
  if (Math.abs(endX - startX) > 500) wrappedEnd += endX < startX ? 1000 : -1000;
  const distance = Math.hypot(wrappedEnd - startX, endY - startY);
  const controlX = (startX + wrappedEnd) / 2;
  const controlY = Math.max(
    -30,
    (startY + endY) / 2 - Math.min(105, distance * 0.24),
  );
  const main = `M${startX},${startY} Q${controlX},${controlY} ${wrappedEnd},${endY}`;
  if (wrappedEnd === endX) return [main];
  const shift = wrappedEnd > 1000 ? -1000 : 1000;
  return [
    main,
    `M${startX + shift},${startY} Q${controlX + shift},${controlY} ${wrappedEnd + shift},${endY}`,
  ];
}

function routeLevelForZoom(zoom: number, count: number) {
  if (zoom < 1.45 || count > 240) return "country" as const;
  if (zoom < 2.5 || count > 100) return "region" as const;
  return "destination" as const;
}

function markerRadius(route: VisualRoute, selected: boolean): number {
  if (selected) return 5.2;
  return Math.min(5, 2.4 + Math.log2(route.count + 1) * 0.48);
}

export const MapView = memo(function MapView({
  connections,
  home,
  selectedId,
  onSelect,
  expanded = false,
  onExpandedChange,
}: MapViewProps) {
  const t = useI18n();
  const hostRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{
    pointerId: number;
    x: number;
    y: number;
    viewport: Viewport;
  } | null>(null);
  const [viewport, setViewport] = useState(DEFAULT_VIEWPORT);
  const [feedCollapsed, setFeedCollapsed] = useState(false);
  const worldPaths = useMemo(() => {
    const borders = detailedWorldBorders().map(borderPath);
    return {
      land: borders.map((path) => `${path} Z`).join(" "),
      borders: borders.join(" "),
    };
  }, []);
  const routes = useMemo(
    () =>
      groupVisualRoutes(
        connections,
        routeLevelForZoom(viewport.zoom, connections.length),
      ).slice(0, 240),
    [connections, viewport.zoom],
  );
  const [startX, startY] = useMemo(() => mapPoint(home[1], home[0]), [home]);
  const width = 1000 / viewport.zoom;
  const height = 500 / viewport.zoom;
  const clampedCenterX = Math.min(
    1000 - width / 2,
    Math.max(width / 2, viewport.centerX),
  );
  const clampedCenterY = Math.min(
    500 - height / 2,
    Math.max(height / 2, viewport.centerY),
  );
  const viewBox = `${clampedCenterX - width / 2} ${clampedCenterY - height / 2} ${width} ${height}`;

  const changeZoom = useCallback((next: number) => {
    setViewport((current) => ({
      ...current,
      zoom: Math.min(8, Math.max(1, next)),
    }));
  }, []);

  const frameSelection = useCallback(() => {
    const route = routes.find((candidate) =>
      selectedId ? candidate.memberIds.includes(selectedId) : false,
    );
    if (!route) return;
    const [x, y] = mapPoint(route.longitude, route.latitude);
    setViewport((current) => ({
      zoom: Math.max(2.8, current.zoom),
      centerX: x,
      centerY: y,
    }));
  }, [routes, selectedId]);

  useEffect(() => {
    if (!expanded) setFeedCollapsed(false);
  }, [expanded]);

  useEffect(() => {
    if (!expanded) return;
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onExpandedChange?.(false);
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [expanded, onExpandedChange]);

  const handleWheel = (event: ReactWheelEvent<SVGSVGElement>) => {
    event.preventDefault();
    changeZoom(viewport.zoom * (event.deltaY < 0 ? 1.18 : 1 / 1.18));
  };

  const handlePointerDown = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = {
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      viewport,
    };
  };

  const handlePointerMove = (event: ReactPointerEvent<SVGSVGElement>) => {
    const drag = dragRef.current;
    const host = hostRef.current;
    if (!drag || drag.pointerId !== event.pointerId || !host) return;
    const bounds = host.getBoundingClientRect();
    const dx =
      ((event.clientX - drag.x) / Math.max(1, bounds.width)) *
      (1000 / viewport.zoom);
    const dy =
      ((event.clientY - drag.y) / Math.max(1, bounds.height)) *
      (500 / viewport.zoom);
    setViewport({
      ...drag.viewport,
      centerX: drag.viewport.centerX - dx,
      centerY: drag.viewport.centerY - dy,
    });
  };

  const handleKeyDown = (event: React.KeyboardEvent<SVGSVGElement>) => {
    const pan = 55 / viewport.zoom;
    if (
      ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "+", "-"].includes(
        event.key,
      )
    )
      event.preventDefault();
    if (event.key === "+" || event.key === "=")
      changeZoom(viewport.zoom * 1.25);
    if (event.key === "-") changeZoom(viewport.zoom / 1.25);
    if (event.key.startsWith("Arrow"))
      setViewport((current) => ({
        ...current,
        centerX:
          current.centerX +
          (event.key === "ArrowRight"
            ? pan
            : event.key === "ArrowLeft"
              ? -pan
              : 0),
        centerY:
          current.centerY +
          (event.key === "ArrowDown"
            ? pan
            : event.key === "ArrowUp"
              ? -pan
              : 0),
      }));
  };

  return (
    <div
      className={`world-map${expanded ? " world-map--expanded" : ""}${feedCollapsed ? " is-feed-collapsed" : ""}`}
      data-testid="map-view"
      ref={hostRef}
    >
      <div
        className="world-map__toolbar"
        role="toolbar"
        aria-label={t("Map controls")}
      >
        <button
          aria-label={t("Zoom in")}
          onClick={() => changeZoom(viewport.zoom * 1.25)}
          type="button"
        >
          +
        </button>
        <button
          aria-label={t("Zoom out")}
          onClick={() => changeZoom(viewport.zoom / 1.25)}
          type="button"
        >
          −
        </button>
        <button onClick={() => setViewport(DEFAULT_VIEWPORT)} type="button">
          {t("Reset")}
        </button>
        <button disabled={!selectedId} onClick={frameSelection} type="button">
          {t("Frame selection")}
        </button>
        {expanded && (
          <button
            onClick={() => setFeedCollapsed((value) => !value)}
            type="button"
          >
            {feedCollapsed ? t("Show feed") : t("Collapse feed")}
          </button>
        )}
        <button onClick={() => onExpandedChange?.(!expanded)} type="button">
          {expanded ? t("Restore map") : t("Expand map")}
        </button>
        <output aria-live="polite">{Math.round(viewport.zoom * 100)}%</output>
      </div>
      <svg
        aria-label={t("World map showing filtered network destinations")}
        className="world-map__svg"
        onKeyDown={handleKeyDown}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={() => (dragRef.current = null)}
        onWheel={handleWheel}
        preserveAspectRatio="xMidYMid meet"
        role="application"
        tabIndex={0}
        viewBox={viewBox}
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
        <g aria-hidden="true" className="world-map__land">
          <path d={worldPaths.land} />
        </g>
        <g aria-hidden="true" className="world-map__borders">
          <path d={worldPaths.borders} />
        </g>
        <g className="world-map__arcs">
          {routes.map((route) => {
            const connection = route.representative;
            const [endX, endY] = mapPoint(route.longitude, route.latitude);
            const paths = routePaths(startX, startY, endX, endY);
            const selected =
              selectedId !== null && route.memberIds.includes(selectedId);
            const selectionTarget = selected ? selectedId : connection.id;
            const radius = markerRadius(route, selected);
            const centroid = connection.geo?.placement === "country_centroid";
            const label =
              connection.geo?.country_name ??
              connection.domain ??
              connection.key.remote.address;
            return (
              <g
                aria-label={t(
                  "{count} estimated connections to {destination}",
                  {
                    count: route.count,
                    destination: label,
                  },
                )}
                className={`${selected ? "is-selected" : ""}${route.active ? " is-active" : " is-historical"}`}
                key={route.key}
                onClick={() => onSelect(selectionTarget)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ")
                    onSelect(selectionTarget);
                }}
                role="button"
                tabIndex={0}
              >
                <title>{`${label} / AS${connection.asn?.number ?? "?"} / ${route.count} / ${connection.geo?.precision ?? "unknown"}`}</title>
                {paths.map((path, index) => (
                  <g key={index}>
                    <path className="world-map__arc-hit" d={path} />
                    <path className="world-map__arc" d={path} />
                  </g>
                ))}
                <circle
                  className="location-marker__hit"
                  cx={endX}
                  cy={endY}
                  r={Math.max(10, radius + 5)}
                />
                <circle
                  className="location-marker__halo"
                  cx={endX}
                  cy={endY}
                  r={radius + 4}
                />
                <circle
                  className={`location-marker__ring${centroid ? " is-centroid" : ""}`}
                  cx={endX}
                  cy={endY}
                  r={radius + 1.5}
                />
                <circle
                  className="location-marker__core"
                  cx={endX}
                  cy={endY}
                  r={radius}
                />
                {route.count > 1 && (
                  <g
                    className="location-marker__badge"
                    transform={`translate(${endX + radius + 1} ${endY - radius - 1})`}
                  >
                    <circle r="6" />
                    <text textAnchor="middle" y="2.2">
                      {route.count > 99 ? "99+" : route.count}
                    </text>
                  </g>
                )}
                {viewport.zoom >= 3.2 && (
                  <text
                    className="world-map__label"
                    x={endX + radius + 5}
                    y={endY - radius - 4}
                  >
                    {label}
                  </text>
                )}
              </g>
            );
          })}
        </g>
        <g
          aria-label={t("Approximate home location")}
          className="origin-marker"
        >
          <title>{t("Approximate local location")}</title>
          <circle
            className="origin-marker__halo"
            cx={startX}
            cy={startY}
            r="11"
          />
          <circle
            className="origin-marker__ring"
            cx={startX}
            cy={startY}
            r="7"
          />
          <path
            d={`M${startX - 3},${startY} H${startX + 3} M${startX},${startY - 3} V${startY + 3}`}
          />
        </g>
      </svg>
    </div>
  );
});
