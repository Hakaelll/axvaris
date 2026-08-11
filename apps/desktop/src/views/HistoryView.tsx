import { useEffect, useMemo, useState } from "react";
import { useI18n } from "../i18n";
import type { HistoryQuery, HistoryResponse } from "../types";
import { formatBytes, formatDateTime, formatDuration } from "../lib/format";

interface HistoryViewProps {
  loadHistory: (
    query?: Partial<HistoryQuery>,
  ) => Promise<HistoryResponse | null>;
  onReconstruct: (
    atMs: number,
    sessionId: string | null,
    preferredConnectionId?: string,
  ) => void;
  onExport: (
    sessionId: string | null,
    fromMs: number | null,
    toMs: number | null,
  ) => void;
}

const ranges = [
  { label: "1 hour", milliseconds: 3_600_000 },
  { label: "24 hours", milliseconds: 86_400_000 },
  { label: "7 days", milliseconds: 604_800_000 },
  { label: "All", milliseconds: null },
] as const;

export function HistoryView({
  loadHistory,
  onReconstruct,
  onExport,
}: HistoryViewProps) {
  const t = useI18n();
  const [history, setHistory] = useState<HistoryResponse | null>(null);
  const [range, setRange] = useState<(typeof ranges)[number]>(ranges[1]);
  const [selectedSession, setSelectedSession] = useState<string | null>(null);
  const [cursorMs, setCursorMs] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    const fromMs =
      selectedSession || range.milliseconds === null
        ? null
        : Date.now() - range.milliseconds;
    void loadHistory({ from_ms: fromMs, session_id: selectedSession }).then(
      (result) => {
        if (!cancelled) {
          setHistory(result);
          setLoading(false);
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [loadHistory, range, selectedSession]);

  const bounds = useMemo(() => {
    const entries = history?.entries ?? [];
    const recordings = history?.recordings ?? [];
    if (entries.length === 0 && recordings.length === 0) return null;
    const starts = [
      ...entries.map((entry) => entry.connection.first_seen_ms),
      ...recordings.map((recording) => recording.started_at_ms),
    ];
    const ends = [
      ...entries.map((entry) => entry.connection.last_seen_ms),
      ...recordings.map((recording) => recording.ended_at_ms ?? Date.now()),
    ];
    return {
      start: Math.min(...starts),
      end: Math.max(...ends),
    };
  }, [history]);

  useEffect(() => {
    if (bounds) setCursorMs(bounds.end);
    else setCursorMs(null);
  }, [bounds]);

  const histogram = useMemo(() => {
    const buckets = Array.from({ length: 32 }, () => 0);
    if (!bounds) return buckets;
    for (const entry of history?.entries ?? []) {
      const position = Math.min(
        31,
        Math.floor(
          ((entry.connection.first_seen_ms - bounds.start) /
            Math.max(1, bounds.end - bounds.start)) *
            32,
        ),
      );
      buckets[position] = (buckets[position] ?? 0) + 1;
    }
    return buckets;
  }, [bounds, history]);
  const maximum = Math.max(1, ...histogram);
  const reconstructedCount =
    cursorMs === null
      ? 0
      : (history?.entries ?? []).filter(
          ({ connection }) =>
            connection.first_seen_ms <= cursorMs &&
            connection.last_seen_ms >= cursorMs,
        ).length;

  return (
    <section className="data-view history-view">
      <header className="view-heading">
        <div>
          <span className="eyebrow">{t("LOCAL EVIDENCE")}</span>
          <h1>{t("History")}</h1>
          <p>
            {t(
              "Open retained sessions or scrub local metadata with Time Machine.",
            )}
          </p>
        </div>
        <div className="view-actions">
          <div className="segmented" aria-label={t("Timeline zoom range")}>
            {ranges.map((item) => (
              <button
                aria-pressed={item.label === range.label}
                className={item.label === range.label ? "is-active" : undefined}
                key={item.label}
                onClick={() => {
                  setRange(item);
                  setSelectedSession(null);
                }}
                type="button"
              >
                {t(item.label)}
              </button>
            ))}
          </div>
          <button
            className="button button--secondary"
            onClick={() =>
              onExport(
                selectedSession,
                bounds?.start ?? null,
                bounds?.end ?? null,
              )
            }
            type="button"
          >
            {t("Export view")}
          </button>
        </div>
      </header>
      <div
        aria-label={t("Connection activity over selected time range")}
        className="timeline"
      >
        <div className="timeline__axis">
          <span>{bounds ? formatDateTime(bounds.start) : t("EARLIER")}</span>
          <span>{t("CONNECTIONS OVER TIME")}</span>
          <span>{bounds ? formatDateTime(bounds.end) : t("NOW")}</span>
        </div>
        <div className="timeline__bars">
          {histogram.map((count, index) => {
            const timestamp = bounds
              ? Math.round(
                  bounds.start +
                    ((bounds.end - bounds.start) * (index + 0.5)) /
                      histogram.length,
                )
              : 0;
            return (
              <button
                aria-label={t(
                  "Move to timeline bucket {index}: {count} connections",
                  { index: index + 1, count },
                )}
                key={index}
                onClick={() => setCursorMs(timestamp)}
                style={{ height: `${Math.max(3, (count / maximum) * 100)}%` }}
                title={t("{count} connections", { count })}
                type="button"
              />
            );
          })}
        </div>
        {bounds && (
          <div
            aria-label={t("Packet recording events")}
            className="timeline__events"
          >
            {(history?.recordings ?? []).map((recording) => (
              <button
                aria-label={t("Recording started {date}", {
                  date: formatDateTime(recording.started_at_ms),
                })}
                key={recording.id}
                onClick={() => setCursorMs(recording.started_at_ms)}
                style={{
                  left: `${((recording.started_at_ms - bounds.start) / Math.max(1, bounds.end - bounds.start)) * 100}%`,
                }}
                title={`PCAPNG recording: ${recording.path}`}
                type="button"
              />
            ))}
          </div>
        )}
        <div className="timeline__scrubber">
          <input
            aria-label={t("Time Machine moment")}
            disabled={!bounds}
            max={bounds?.end ?? 1}
            min={bounds?.start ?? 0}
            onChange={(event) => setCursorMs(Number(event.target.value))}
            step={1_000}
            type="range"
            value={cursorMs ?? 0}
          />
          <span>
            <strong>
              {cursorMs ? formatDateTime(cursorMs) : t("No retained moment")}
            </strong>
            <small>
              {t("{count} approximately active", {
                count: reconstructedCount,
              })}
            </small>
          </span>
          <button
            className="button button--primary"
            disabled={cursorMs === null}
            onClick={() =>
              cursorMs !== null && onReconstruct(cursorMs, selectedSession)
            }
            type="button"
          >
            {t("Reconstruct globe")}
          </button>
        </div>
      </div>
      <div className="history-layout">
        <div className="history-sessions">
          <h2>{t("Sessions")}</h2>
          <button
            aria-pressed={selectedSession === null}
            className={`history-session${selectedSession === null ? " is-selected" : ""}`}
            onClick={() => setSelectedSession(null)}
            type="button"
          >
            <span className="session-dot" />
            <div>
              <strong>{t("All retained sessions")}</strong>
              <small>{t("Current time range")}</small>
            </div>
            <div>
              <strong>{history?.sessions.length ?? 0}</strong>
              <small>{t("sessions")}</small>
            </div>
          </button>
          {(history?.sessions ?? []).map((session) => (
            <button
              aria-pressed={selectedSession === session.id}
              className={`history-session${selectedSession === session.id ? " is-selected" : ""}`}
              key={session.id}
              onClick={() => setSelectedSession(session.id)}
              type="button"
            >
              <span className="session-dot" />
              <div>
                <strong>{formatDateTime(session.started_at_ms)}</strong>
                <small>
                  {session.ended_at_ms
                    ? formatDuration(
                        session.ended_at_ms - session.started_at_ms,
                      )
                    : t("Current session")}
                </small>
              </div>
              <div>
                <strong>
                  {session.connection_count}
                  {session.recording_count > 0
                    ? ` / ${session.recording_count}`
                    : ""}
                </strong>
                <small>
                  {session.recording_count > 0
                    ? t("connections / PCAPNG")
                    : t("connections")}
                </small>
              </div>
            </button>
          ))}
          {!loading && (history?.sessions.length ?? 0) === 0 && (
            <p className="list-empty">{t("No retained sessions.")}</p>
          )}
        </div>
        <div className="history-entries">
          <div className="history-entries__heading">
            <h2>{t("Connections")}</h2>
            <span>
              {t("{count} records", { count: history?.entries.length ?? 0 })}
            </span>
          </div>
          {loading ? (
            <div className="loading-line" />
          ) : (
            (history?.entries ?? [])
              .slice(0, 300)
              .map(({ session_id: sessionId, connection }) => (
                <button
                  key={`${sessionId}-${connection.id}`}
                  onClick={() =>
                    onReconstruct(
                      Math.min(
                        connection.last_seen_ms,
                        connection.first_seen_ms + 1,
                      ),
                      sessionId,
                      connection.id,
                    )
                  }
                  type="button"
                >
                  <time>{formatDateTime(connection.first_seen_ms)}</time>
                  <span>
                    <strong>{connection.process.name}</strong>
                    <small>PID {connection.process.pid ?? "?"}</small>
                  </span>
                  <span>
                    <strong>
                      {connection.domain ?? connection.key.remote.address}
                    </strong>
                    <small>
                      {connection.geo?.country_name ?? t("Unknown location")}
                    </small>
                  </span>
                  <span>
                    <strong>
                      {connection.protocol.toUpperCase()}{" "}
                      {connection.key.remote.port}
                    </strong>
                    <small>
                      {formatBytes(
                        connection.traffic.bytes_received +
                          connection.traffic.bytes_sent,
                      )}
                    </small>
                  </span>
                </button>
              ))
          )}
        </div>
      </div>
    </section>
  );
}
