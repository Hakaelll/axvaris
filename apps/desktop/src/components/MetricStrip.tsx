import type { DashboardSnapshot } from "../types";
import { useI18n } from "../i18n";
import { formatBytes } from "../lib/format";
import { Icon } from "./Icon";

export function MetricStrip({ snapshot }: { snapshot: DashboardSnapshot }) {
  const t = useI18n();
  const { stats, diagnostics } = snapshot;
  const trafficSource =
    snapshot.capture.mode === "live"
      ? t("Live device interface total")
      : t("Observed connection total");
  return (
    <footer aria-label={t("Live network metrics")} className="metric-strip">
      <div>
        <span className="metric-strip__pulse" />
        <small>{t("ACTIVE")}</small>
        <strong>{stats.active_connections}</strong>
      </div>
      <div title={trafficSource}>
        <Icon name="arrow-down" />
        <small>{t("DOWN")}</small>
        <strong>{formatBytes(stats.down_bytes_per_second, true)}</strong>
      </div>
      <div title={trafficSource}>
        <Icon name="arrow-up" />
        <small>{t("UP")}</small>
        <strong>{formatBytes(stats.up_bytes_per_second, true)}</strong>
      </div>
      <div>
        <small>{t("COUNTRIES")}</small>
        <strong>{stats.countries}</strong>
      </div>
      <div>
        <small>{t("PROCESSES")}</small>
        <strong>{stats.processes}</strong>
      </div>
      <div title={trafficSource}>
        <small>{t("PACKETS")}</small>
        <strong>
          {Math.round(stats.packets_per_second).toLocaleString()}/s
        </strong>
      </div>
      <div
        className="metric-strip__health"
        title={`${diagnostics.backend}; ${diagnostics.capture_queue_depth}/${diagnostics.capture_queue_capacity} queued`}
      >
        <span
          className={diagnostics.dropped_capture_events > 0 ? "is-warning" : ""}
        />
        <small>{diagnostics.backend}</small>
      </div>
    </footer>
  );
}
