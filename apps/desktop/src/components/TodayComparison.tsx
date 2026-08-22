import { useI18n } from "../i18n";
import { formatBytes, formatDateTime } from "../lib/format";
import type {
  ComparisonRow,
  TodayEvidenceTarget,
  TodaySessionComparison,
} from "../types";
import {
  usePresentation,
  type SensitiveKind,
} from "../privacy/PresentationMode";

interface TodayComparisonProps {
  comparison: TodaySessionComparison | null;
  onNavigate: (
    target: TodayEvidenceTarget,
    connectionId: string | null,
    query: string,
  ) => void;
}

const categories = ["processes", "destinations", "countries", "ports"] as const;
type Category = (typeof categories)[number];

function queryFor(category: Category, row: ComparisonRow): string {
  if (category === "processes") return `process:${JSON.stringify(row.key)}`;
  if (category === "countries") return `country:${JSON.stringify(row.key)}`;
  if (category === "ports") return `port:${row.key}`;
  return JSON.stringify(row.key);
}

function signedBytes(value: number): string {
  if (value === 0) return formatBytes(0);
  return `${value > 0 ? "+" : "−"}${formatBytes(Math.abs(value))}`;
}

function kindForCategory(category: Category): SensitiveKind | null {
  if (category === "processes") return "process";
  if (category === "destinations") return "domain";
  return null;
}

export function TodayComparison({
  comparison,
  onNavigate,
}: TodayComparisonProps) {
  const t = useI18n();
  const { display } = usePresentation();
  if (!comparison) {
    return (
      <section
        className="today-comparison"
        aria-labelledby="today-comparison-title"
      >
        <header>
          <div>
            <span className="eyebrow">{t("QUICK COMPARISON")}</span>
            <h2 id="today-comparison-title">{t("Previous session")}</h2>
          </div>
        </header>
        <div className="today-empty">
          <strong>{t("No previous valid session")}</strong>
          <p>
            {t(
              "A comparison appears after a retained session contains connection evidence.",
            )}
          </p>
        </div>
      </section>
    );
  }

  const structuralChanges = categories.reduce(
    (total, category) => total + comparison.comparison[category].length,
    0,
  );
  const listenerChanges =
    comparison.listener_added.length +
    comparison.listener_removed.length +
    comparison.listener_widened.length;
  const sentDelta =
    comparison.current_traffic.bytes_sent -
    comparison.previous_traffic.bytes_sent;
  const receivedDelta =
    comparison.current_traffic.bytes_received -
    comparison.previous_traffic.bytes_received;

  return (
    <section
      className="today-comparison"
      aria-labelledby="today-comparison-title"
    >
      <header>
        <div>
          <span className="eyebrow">{t("QUICK COMPARISON")}</span>
          <h2 id="today-comparison-title">{t("Previous session")}</h2>
          <p>
            {t("Compared with {date}", {
              date: formatDateTime(comparison.previous_session.started_at_ms),
            })}
          </p>
        </div>
        {comparison.traffic_comparable ? (
          <dl className="comparison-traffic">
            <div>
              <dt>{t("Sent difference")}</dt>
              <dd>{signedBytes(sentDelta)}</dd>
            </div>
            <div>
              <dt>{t("Received difference")}</dt>
              <dd>{signedBytes(receivedDelta)}</dd>
            </div>
          </dl>
        ) : (
          <p className="quality-warning" role="note">
            {t(
              "Traffic totals are not comparable because counter sources differ or are unavailable. Structural changes remain valid.",
            )}
          </p>
        )}
      </header>

      {structuralChanges === 0 && listenerChanges === 0 ? (
        <p className="list-empty">
          {t("No structural differences from the previous session.")}
        </p>
      ) : (
        <div className="today-comparison-grid">
          {categories.map((category) => {
            const rows = comparison.comparison[category];
            if (rows.length === 0) return null;
            return (
              <section
                key={category}
                aria-labelledby={`comparison-${category}`}
              >
                <h3 id={`comparison-${category}`}>{t(category)}</h3>
                <ul>
                  {rows.map((row) => (
                    <li key={row.key}>
                      <button
                        onClick={() =>
                          onNavigate(
                            "investigate",
                            null,
                            queryFor(category, row),
                          )
                        }
                        type="button"
                      >
                        <span>
                          {kindForCategory(category)
                            ? display(kindForCategory(category)!, row.key)
                            : row.key}
                        </span>
                        <strong
                          className={
                            row.delta > 0 ? "state-positive" : "state-negative"
                          }
                        >
                          {row.delta > 0 ? "+" : ""}
                          {row.delta}
                        </strong>
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            );
          })}
          {listenerChanges > 0 && (
            <section aria-labelledby="comparison-listeners">
              <h3 id="comparison-listeners">{t("Listeners")}</h3>
              <ul>
                {[
                  ["Added", comparison.listener_added.length],
                  ["Removed", comparison.listener_removed.length],
                  ["Widened", comparison.listener_widened.length],
                ].map(([label, count]) =>
                  Number(count) > 0 ? (
                    <li key={label}>
                      <button
                        onClick={() =>
                          onNavigate("exposure", null, "state:listening")
                        }
                        type="button"
                      >
                        <span>{t(String(label))}</span>
                        <strong>{count}</strong>
                      </button>
                    </li>
                  ) : null,
                )}
              </ul>
            </section>
          )}
        </div>
      )}
    </section>
  );
}
