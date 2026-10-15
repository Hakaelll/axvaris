import { useMemo } from "react";
import { useI18n } from "../i18n";
import { classifyUnlocatedTargets } from "../lib/unlocated";
import type { DashboardSnapshot, TodayEvidenceTarget } from "../types";
import { usePresentation } from "../privacy/PresentationMode";
import { worldBorders } from "../lib/world";

interface UnlocatedTargetsProps {
  snapshot: DashboardSnapshot;
  geoIpInstalled: boolean;
  onNavigate: (
    target: TodayEvidenceTarget,
    connectionId: string | null,
    query: string,
  ) => void;
}

const reasonLabels = {
  geoip_absent: "GeoIP data absent or no local match",
  local_or_reserved: "Local or reserved destination",
  low_precision: "Location precision is too low",
  no_coordinates: "No usable coordinates",
  pending: "Enrichment pending",
} as const;

export function UnlocatedTargets({
  snapshot,
  geoIpInstalled,
  onNavigate,
}: UnlocatedTargetsProps) {
  const t = useI18n();
  const { display } = usePresentation();
  const targets = useMemo(
    () =>
      classifyUnlocatedTargets(
        snapshot.connections,
        snapshot.generated_at_ms,
        snapshot.capture.mode !== "off",
      ),
    [snapshot.capture.mode, snapshot.connections, snapshot.generated_at_ms],
  );
  const plotted = snapshot.connections.length - targets.length;
  const mapOutline = useMemo(
    () =>
      worldBorders()
        .map((line) =>
          line
            .map(
              ([longitude, latitude], index) =>
                `${index && Math.abs(longitude - (line[index - 1]?.[0] ?? longitude)) <= 180 ? "L" : "M"}${longitude + 180},${90 - latitude}`,
            )
            .join(" "),
        )
        .join(" "),
    [],
  );

  return (
    <section className="unlocated-targets" aria-labelledby="unlocated-title">
      <header>
        <div>
          <h2 id="unlocated-title">{t("Location coverage")}</h2>
          <p>
            {t(
              "Missing coordinates stay unplotted; Axvaris never invents a location.",
            )}
          </p>
        </div>
        <dl>
          <div>
            <dt>{t("Plotted")}</dt>
            <dd>{plotted}</dd>
          </div>
          <div>
            <dt>{t("Unlocated")}</dt>
            <dd>{targets.length}</dd>
          </div>
        </dl>
      </header>
      <svg
        className="location-coverage-map"
        aria-hidden="true"
        viewBox="0 0 360 165"
      >
        <path d={mapOutline} />
      </svg>
      <div className="unlocated-actions">
        <button
          className="button button--secondary"
          disabled={targets.length === 0}
          onClick={() => onNavigate("investigate", null, "geo:unlocated")}
          type="button"
        >
          {t("Filter unlocated evidence")}
        </button>
        <button
          className="button button--secondary"
          onClick={() => onNavigate("health", null, "geoip")}
          type="button"
        >
          {geoIpInstalled
            ? t("Open GeoIP settings")
            : t("Configure local GeoIP")}
        </button>
      </div>
      {targets.length > 0 ? (
        <ul
          className="unlocated-list"
          aria-label={t("Unlocated target evidence")}
        >
          {targets.slice(0, 12).map(({ connection, reason }) => {
            const destination =
              connection.domain ??
              connection.hostname ??
              connection.key.remote.address;
            const displayedProcess = display(
              "process",
              connection.process.name,
            );
            const displayedDestination = display(
              connection.domain || connection.hostname
                ? "domain"
                : connection.key.remote.scope === "global"
                  ? "public_ip"
                  : "local_ip",
              destination,
            );
            return (
              <li key={connection.id}>
                <div>
                  <strong>{displayedProcess}</strong>
                  <span>{displayedDestination}</span>
                  <small>{t(reasonLabels[reason])}</small>
                </div>
                <button
                  aria-label={t(
                    "Inspect {process} to {destination}: {reason}",
                    {
                      process: displayedProcess,
                      destination: displayedDestination,
                      reason: t(reasonLabels[reason]),
                    },
                  )}
                  onClick={() =>
                    onNavigate(
                      "investigate",
                      connection.id,
                      `process:${JSON.stringify(connection.process.name)} AND ip:${connection.key.remote.address}`,
                    )
                  }
                  type="button"
                >
                  {t("Inspect")}
                </button>
              </li>
            );
          })}
        </ul>
      ) : (
        <p className="list-empty">
          {t("Every current target has a usable plotted position.")}
        </p>
      )}
    </section>
  );
}
