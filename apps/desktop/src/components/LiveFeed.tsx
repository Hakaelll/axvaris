import { useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "../i18n";
import type { Connection } from "../types";
import {
  endpointLabel,
  formatBytes,
  formatDateTime,
  formatDuration,
  formatTime,
  processInitials,
} from "../lib/format";

interface LiveFeedProps {
  connections: Connection[];
  selectedId: string | null;
  paused: boolean;
  queued: number;
  groupWindowSeconds: number;
  flowTrafficAvailable: boolean;
  onSelect: (id: string) => void;
  onPause: (paused: boolean) => void;
}

interface FeedGroup {
  id: string;
  items: Connection[];
  firstSeenMs: number;
  lastSeenMs: number;
}

function conceptualKey(connection: Connection): string {
  return [
    connection.process.name.toLowerCase(),
    connection.process.pid ?? "unknown",
    connection.key.remote.address,
    connection.key.remote.port,
    connection.protocol,
  ].join("|");
}

function groupConnections(
  connections: Connection[],
  windowSeconds: number,
): FeedGroup[] {
  const windowMs = Math.max(1, windowSeconds) * 1_000;
  const groups = new Map<string, FeedGroup[]>();
  for (const connection of [...connections].sort(
    (left, right) => left.first_seen_ms - right.first_seen_ms,
  )) {
    const key = conceptualKey(connection);
    const candidates = groups.get(key) ?? [];
    const current = candidates.at(-1);
    if (current && connection.first_seen_ms - current.lastSeenMs <= windowMs) {
      current.items.push(connection);
      current.firstSeenMs = Math.min(
        current.firstSeenMs,
        connection.first_seen_ms,
      );
      current.lastSeenMs = Math.max(
        current.lastSeenMs,
        connection.last_seen_ms,
      );
    } else {
      candidates.push({
        id: `${key}|${connection.first_seen_ms}`,
        items: [connection],
        firstSeenMs: connection.first_seen_ms,
        lastSeenMs: connection.last_seen_ms,
      });
      groups.set(key, candidates);
    }
  }
  return [...groups.values()].flat().sort((left, right) => {
    const leftActive = left.items.some((item) => item.active);
    const rightActive = right.items.some((item) => item.active);
    return (
      Number(rightActive) - Number(leftActive) ||
      right.lastSeenMs - left.lastSeenMs
    );
  });
}

export function LiveFeed({
  connections,
  selectedId,
  paused,
  queued,
  groupWindowSeconds,
  flowTrafficAvailable,
  onSelect,
  onPause,
}: LiveFeedProps) {
  const t = useI18n();
  const listRef = useRef<HTMLDivElement>(null);
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const groups = useMemo(
    () => groupConnections(connections, groupWindowSeconds),
    [connections, groupWindowSeconds],
  );

  useEffect(() => {
    if (!selectedId) return;
    listRef.current
      ?.querySelector<HTMLElement>(".feed-group.is-selected")
      ?.scrollIntoView({ block: "nearest" });
  }, [groups, selectedId]);

  const toggleExpanded = (id: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <aside aria-label={t("Live connections")} className="live-feed">
      <header className="panel-heading">
        <div>
          <span className="eyebrow">{t("LIVE CONNECTIONS")}</span>
          <strong>{groups.length.toLocaleString()}</strong>
          <small>
            {connections.length !== groups.length
              ? ` / ${connections.length} sockets`
              : ""}
          </small>
        </div>
        <button
          className={`pause-button${paused ? " is-active" : ""}`}
          onClick={() => void onPause(!paused)}
          type="button"
        >
          <span>{paused ? t("Resume") : t("Pause view")}</span>
          {paused && queued > 0 && <b>+{queued}</b>}
        </button>
      </header>
      <div
        aria-live="polite"
        className="connection-list"
        ref={listRef}
        role="list"
      >
        {groups.length === 0 ? (
          <div className="feed-empty">
            <span className="feed-empty__radar" />
            <strong>{t("Waiting for network activity")}</strong>
            <p>{t("Start live capture or use deterministic demo data.")}</p>
          </div>
        ) : (
          groups.map((group) => {
            const selectedItem = group.items.find(
              (item) => item.id === selectedId,
            );
            const representative =
              selectedItem ??
              [...group.items].sort(
                (left, right) => right.last_seen_ms - left.last_seen_ms,
              )[0]!;
            const selected = Boolean(selectedItem);
            const active = group.items.some((item) => item.active);
            const destination =
              representative.domain ??
              representative.hostname ??
              representative.key.remote.address;
            const place =
              [representative.geo?.city, representative.geo?.country_code]
                .filter(Boolean)
                .join(", ") || t("Unknown location");
            const totalRate = group.items.reduce(
              (sum, item) =>
                sum +
                item.rates.down_bytes_per_second +
                item.rates.up_bytes_per_second,
              0,
            );
            const bytesDown = group.items.reduce(
              (sum, item) => sum + item.traffic.bytes_received,
              0,
            );
            const bytesUp = group.items.reduce(
              (sum, item) => sum + item.traffic.bytes_sent,
              0,
            );
            const hasFlowTraffic =
              flowTrafficAvailable ||
              totalRate > 0 ||
              bytesDown > 0 ||
              bytesUp > 0;
            const observations = group.items.reduce(
              (sum, item) => sum + item.repeat_count,
              0,
            );
            const isExpanded = expanded.has(group.id);
            return (
              <article
                className={`feed-group${selected ? " is-selected" : ""}`}
                key={group.id}
                role="listitem"
              >
                <button
                  aria-pressed={selected}
                  className={`connection-row${selected ? " is-selected" : ""}${active ? " is-active" : " is-closed"}`}
                  onClick={() => onSelect(representative.id)}
                  type="button"
                >
                  <span className="process-avatar">
                    {processInitials(representative.process.name)}
                  </span>
                  <span className="connection-row__body">
                    <span className="connection-row__title">
                      <strong>{representative.process.name}</strong>
                      <time>{formatTime(group.lastSeenMs)}</time>
                    </span>
                    <span
                      className="connection-row__destination"
                      title={endpointLabel(
                        representative.key.remote.address,
                        representative.key.remote.port,
                      )}
                    >
                      {destination}
                      <em>:{representative.key.remote.port}</em>
                      {observations > 1 && <b>× {observations}</b>}
                    </span>
                    <span className="connection-row__endpoint">
                      {representative.key.remote.address}
                    </span>
                    <span className="connection-row__meta">
                      <span>{place}</span>
                      <span>{representative.protocol.toUpperCase()}</span>
                      <span>
                        {formatDuration(group.lastSeenMs - group.firstSeenMs)}
                      </span>
                    </span>
                    <span className="connection-row__traffic">
                      {hasFlowTraffic ? (
                        <>
                          ↓ {formatBytes(bytesDown)} <i /> ↑{" "}
                          {formatBytes(bytesUp)}
                        </>
                      ) : (
                        t("Socket metadata only")
                      )}
                    </span>
                  </span>
                  <span className="connection-row__rate">
                    <i
                      style={
                        {
                          "--rate": hasFlowTraffic
                            ? `${Math.min(100, Math.max(5, Math.log10(totalRate + 1) * 17))}%`
                            : "0%",
                        } as React.CSSProperties
                      }
                    />
                    <small>
                      {hasFlowTraffic
                        ? formatBytes(totalRate, true)
                        : t("Endpoint only")}
                    </small>
                  </span>
                  {representative.unusual.length > 0 && (
                    <span
                      aria-label={t("Observed deviation")}
                      className="signal-mark"
                      title={representative.unusual[0]?.reason}
                    >
                      !
                    </span>
                  )}
                </button>
                <button
                  aria-expanded={isExpanded}
                  className="feed-group__toggle"
                  onClick={() => toggleExpanded(group.id)}
                  type="button"
                >
                  {isExpanded ? t("Hide details") : t("Expand group")}
                  <span>
                    {t(
                      group.items.length === 1
                        ? "{count} socket / {seconds}s window"
                        : "{count} sockets / {seconds}s window",
                      {
                        count: group.items.length,
                        seconds: groupWindowSeconds,
                      },
                    )}
                  </span>
                </button>
                {isExpanded && (
                  <div className="feed-group__details">
                    <div>
                      <span>{t("First")}</span>
                      <strong>{formatDateTime(group.firstSeenMs)}</strong>
                    </div>
                    <div>
                      <span>{t("Last")}</span>
                      <strong>{formatDateTime(group.lastSeenMs)}</strong>
                    </div>
                    {group.items.map((item) => (
                      <button
                        key={item.id}
                        onClick={() => onSelect(item.id)}
                        type="button"
                      >
                        <span>
                          {endpointLabel(
                            item.key.local.address,
                            item.key.local.port,
                          )}
                        </span>
                        <strong>{t(item.state)}</strong>
                        <time>{formatTime(item.last_seen_ms)}</time>
                      </button>
                    ))}
                  </div>
                )}
              </article>
            );
          })
        )}
      </div>
    </aside>
  );
}
