import type { Connection, ProcessSummary } from "../types";
import { useI18n } from "../i18n";
import { formatBytes, processInitials } from "../lib/format";

interface ProcessViewProps {
  processes: ProcessSummary[];
  connections: Connection[];
  onFilter: (query: string) => void;
  rateHistory: Record<string, number[]>;
  flowCountersAvailable: boolean;
}

export function ProcessView({
  processes,
  connections,
  onFilter,
  rateHistory,
  flowCountersAvailable,
}: ProcessViewProps) {
  const t = useI18n();
  const peak = Math.max(
    1,
    ...processes.map(
      (item) => item.traffic.bytes_received + item.traffic.bytes_sent,
    ),
  );
  return (
    <section className="data-view">
      <header className="view-heading">
        <div>
          <span className="eyebrow">{t("NETWORK ACTIVITY")}</span>
          <h1>{t("Processes")}</h1>
          <p>{t("Applications observed opening local network sockets.")}</p>
        </div>
        <span className="view-heading__count">
          {t("{count} observed", { count: processes.length })}
        </span>
      </header>
      <div className="rank-list">
        <div className="rank-list__header">
          <span>{t("Process")}</span>
          <span>{t("Connections")}</span>
          <span>{t("Countries")}</span>
          <span>{t("Traffic")}</span>
          <span />
        </div>
        {processes.map((process, index) => {
          const total =
            process.traffic.bytes_received + process.traffic.bytes_sent;
          const related = connections.filter(
            (connection) =>
              `${connection.process.name}:${connection.process.pid ?? "unknown"}` ===
              process.key,
          );
          const activeRate = related.reduce(
            (sum, item) =>
              sum +
              item.rates.down_bytes_per_second +
              item.rates.up_bytes_per_second,
            0,
          );
          const sparkline = rateHistory[process.key] ?? [];
          const sparklinePeak = Math.max(1, ...sparkline);
          const points = sparkline
            .map(
              (value, pointIndex) =>
                `${(pointIndex / Math.max(1, sparkline.length - 1)) * 86},${24 - (value / sparklinePeak) * 22}`,
            )
            .join(" ");
          return (
            <button
              className="rank-row"
              key={process.key}
              onClick={() =>
                onFilter(`process:${JSON.stringify(process.process.name)}`)
              }
              type="button"
            >
              <span className="rank-row__identity">
                <em>{String(index + 1).padStart(2, "0")}</em>
                <i className="process-avatar">
                  {processInitials(process.process.name)}
                </i>
                <span>
                  <strong>{process.process.name}</strong>
                  <small>PID {process.process.pid ?? t("unknown")}</small>
                </span>
              </span>
              <span>
                <strong>{process.active_connections}</strong>
                <small>
                  {t("{count} total", { count: process.total_connections })}
                </small>
              </span>
              <span>
                <strong>{process.countries}</strong>
                <small>{t("destinations")}</small>
              </span>
              <span className="rank-row__bar">
                <i style={{ width: `${(total / peak) * 100}%` }} />
                <svg
                  aria-label={t("Recent process traffic sparkline")}
                  className="process-sparkline"
                  role="img"
                  viewBox="0 0 86 26"
                >
                  <polyline points={points} />
                </svg>
                <strong>{formatBytes(total)}</strong>
                <small>
                  {t("{rate} now", { rate: formatBytes(activeRate, true) })}
                  {" / "}
                  {flowCountersAvailable
                    ? t("per-flow events")
                    : t("interface aggregate only")}
                </small>
              </span>
              <span className="rank-row__arrow">→</span>
            </button>
          );
        })}
        {processes.length === 0 && (
          <div className="list-empty">
            {t("No processes match the current view.")}
          </div>
        )}
      </div>
    </section>
  );
}
