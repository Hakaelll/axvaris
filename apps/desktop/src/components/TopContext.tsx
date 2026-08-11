import { useMemo } from "react";
import type { Connection } from "../types";
import { useI18n } from "../i18n";

interface RankedValue {
  value: string;
  score: number;
  query: string;
}

function top(values: RankedValue[]): RankedValue | null {
  return values.sort((left, right) => right.score - left.score)[0] ?? null;
}

function rank(
  connections: Connection[],
  value: (connection: Connection) => string | null,
  query: (value: string) => string,
): RankedValue | null {
  const totals = new Map<string, number>();
  for (const connection of connections) {
    const key = value(connection);
    if (!key) continue;
    const weight =
      connection.traffic.bytes_received +
      connection.traffic.bytes_sent +
      Math.max(1, connection.repeat_count);
    totals.set(key, (totals.get(key) ?? 0) + weight);
  }
  return top(
    [...totals].map(([item, score]) => ({
      value: item,
      score,
      query: query(item),
    })),
  );
}

export function TopContext({
  connections,
  onFilter,
}: {
  connections: Connection[];
  onFilter: (query: string) => void;
}) {
  const t = useI18n();
  const entries = useMemo(
    () =>
      [
        [
          "PROCESS",
          rank(
            connections,
            (item) => item.process.name,
            (value) => `process:${JSON.stringify(value)}`,
          ),
        ],
        [
          "COUNTRY",
          rank(
            connections,
            (item) => item.geo?.country_code ?? null,
            (value) => `country:${value}`,
          ),
        ],
        [
          "DESTINATION",
          rank(
            connections,
            (item) => item.domain ?? item.hostname ?? item.key.remote.address,
            (value) => JSON.stringify(value),
          ),
        ],
        [
          "DOMAIN",
          rank(
            connections,
            (item) => item.domain ?? item.hostname,
            (value) => `domain:${JSON.stringify(value)}`,
          ),
        ],
        [
          "PORT",
          rank(
            connections,
            (item) => String(item.key.remote.port),
            (value) => `port:${value}`,
          ),
        ],
        [
          "ASN",
          rank(
            connections,
            (item) => (item.asn ? String(item.asn.number) : null),
            (value) => `asn:${value}`,
          ),
        ],
      ] as const,
    [connections],
  );
  if (connections.length === 0) return null;
  return (
    <div aria-label={t("Top context")} className="top-context">
      {entries.map(
        ([label, entry]) =>
          entry && (
            <button
              key={label}
              onClick={() => onFilter(entry.query)}
              type="button"
            >
              <span>{t(label)}</span>
              <strong>
                {label === "ASN" ? `AS${entry.value}` : entry.value}
              </strong>
            </button>
          ),
      )}
    </div>
  );
}
