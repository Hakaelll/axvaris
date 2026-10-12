import { useId, useMemo } from "react";
import { useI18n } from "../i18n";
import { formatBytes, formatDuration } from "../lib/format";
import { classifyUnlocatedTargets } from "../lib/unlocated";
import type {
  DashboardSnapshot,
  NetworkProfile,
  TrafficTrendPoint,
} from "../types";
import { usePresentation } from "../privacy/PresentationMode";

interface TodaySessionStripProps {
  snapshot: DashboardSnapshot;
  profiles: NetworkProfile[];
  activeProfileId: string | null;
  trend: TrafficTrendPoint[];
}

function trendPoints(trend: TrafficTrendPoint[]): string {
  if (trend.length === 0) return "0,28 100,28";
  const values = trend.map(
    (point) => point.down_bytes_per_second + point.up_bytes_per_second,
  );
  const maximum = Math.max(...values, 1);
  return values
    .map((value, index) => {
      const x = values.length === 1 ? 50 : (index / (values.length - 1)) * 100;
      const y = 30 - (value / maximum) * 27;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function TodaySessionStrip({
  snapshot,
  profiles,
  activeProfileId,
  trend,
}: TodaySessionStripProps) {
  const t = useI18n();
  const { display } = usePresentation();
  const chartTitleId = useId();
  const activeConnections = snapshot.connections.filter(
    (connection) => connection.active,
  );
  const activeProcesses = new Set(
    activeConnections.map((connection) => connection.process.name),
  ).size;
  const countersAvailable =
    snapshot.capture.mode === "demo" ||
    snapshot.connections.some(
      (connection) => connection.counter_source !== "unavailable",
    );
  const unlocated = classifyUnlocatedTargets(
    snapshot.connections,
    snapshot.generated_at_ms,
    snapshot.capture.mode !== "off",
  ).length;
  const profile = profiles.find((item) => item.id === activeProfileId);
  const counterQuality =
    snapshot.capture.mode === "demo"
      ? t("Deterministic demo counters")
      : snapshot.diagnostics.flow_counters_available
        ? t("Per-flow counters")
        : countersAvailable
          ? t("Aggregate counters")
          : t("Counters unavailable");
  const duration = snapshot.session
    ? formatDuration(
        Math.max(
          0,
          (snapshot.session.ended_at_ms ?? snapshot.generated_at_ms) -
            snapshot.session.started_at_ms,
        ),
      )
    : "—";
  const traffic = snapshot.session?.traffic;
  const chartSummary = useMemo(() => {
    if (trend.length === 0) return t("Traffic trend unavailable");
    const latest = trend.at(-1);
    return t("Traffic trend with {count} samples; latest {rate}", {
      count: trend.length,
      rate: formatBytes(
        (latest?.down_bytes_per_second ?? 0) +
          (latest?.up_bytes_per_second ?? 0),
        true,
      ),
    });
  }, [t, trend]);

  return (
    <section
      className="today-session-strip"
      aria-labelledby="session-strip-title"
    >
      <div className="today-session-strip__heading">
        <span className="sr-only" id="session-strip-title">
          {t("CURRENT SESSION")}
        </span>
        <strong>
          {profile?.display_name
            ? display("profile", profile.display_name)
            : t("Current network profile")}
        </strong>
        <small>
          {snapshot.capture.mode === "off"
            ? t("Capture stopped")
            : t("Capture active")}
          {" · "}
          {counterQuality}
        </small>
      </div>
      <dl className="today-session-metrics">
        <div>
          <dt>{t("Duration")}</dt>
          <dd>{duration}</dd>
        </div>
        <div>
          <dt>{t("Active connections")}</dt>
          <dd>{activeConnections.length}</dd>
        </div>
        <div>
          <dt>{t("Active processes")}</dt>
          <dd>{activeProcesses}</dd>
        </div>
        <div>
          <dt>{t("Sent")}</dt>
          <dd>
            {countersAvailable && traffic
              ? formatBytes(traffic.bytes_sent)
              : "—"}
          </dd>
        </div>
        <div>
          <dt>{t("Received")}</dt>
          <dd>
            {countersAvailable && traffic
              ? formatBytes(traffic.bytes_received)
              : "—"}
          </dd>
        </div>
        <div>
          <dt>{t("Unlocated")}</dt>
          <dd>{unlocated}</dd>
        </div>
      </dl>
      <svg
        aria-labelledby={chartTitleId}
        className="today-traffic-sparkline"
        preserveAspectRatio="none"
        role="img"
        viewBox="0 0 100 32"
      >
        <title id={chartTitleId}>{chartSummary}</title>
        <path d="M0 30H100" />
        <polyline points={trendPoints(trend)} />
      </svg>
    </section>
  );
}
