import { useEffect, useMemo, useState } from "react";
import { useI18n } from "../i18n";
import type {
  HistoryQuery,
  HistoryResponse,
  SessionComparison,
  TimelineResolution,
  TrafficSample,
} from "../types";
import { formatBytes, formatDateTime, formatDuration } from "../lib/format";
import { desktopApi } from "../lib/backend";

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
  const [traffic, setTraffic] = useState<TrafficSample[]>([]);
  const [trafficResolution, setTrafficResolution] =
    useState<TimelineResolution>("one_minute");
  const [timelinePaused, setTimelinePaused] = useState(false);
  const [leftSession, setLeftSession] = useState("");
  const [rightSession, setRightSession] = useState("");
  const [comparison, setComparison] = useState<SessionComparison | null>(null);
  const [comparisonCategory, setComparisonCategory] = useState<
    "processes" | "destinations" | "countries" | "ports"
  >("processes");
  const [comparisonExported, setComparisonExported] = useState<string | null>(
    null,
  );

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

  useEffect(() => {
    if (timelinePaused) return;
    const now = Date.now();
    const fromMs =
      bounds?.start ??
      (range.milliseconds === null
        ? now - 7 * 86_400_000
        : now - range.milliseconds);
    const toMs = bounds?.end ?? now;
    let cancelled = false;
    void desktopApi
      .queryTrafficTimeline({
        from_ms: fromMs,
        to_ms: toMs,
        session_id: selectedSession,
        process_key: null,
        resolution: trafficResolution,
      })
      .then((samples) => {
        if (!cancelled) setTraffic(samples);
      });
    return () => {
      cancelled = true;
    };
  }, [
    bounds?.end,
    bounds?.start,
    range.milliseconds,
    selectedSession,
    timelinePaused,
    trafficResolution,
  ]);

  useEffect(() => {
    const sessions = history?.sessions ?? [];
    if (!leftSession && sessions[1]) setLeftSession(sessions[1].id);
    if (!rightSession && sessions[0]) setRightSession(sessions[0].id);
  }, [history?.sessions, leftSession, rightSession]);

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
  const trafficBars = useMemo(() => {
    if (traffic.length <= 96)
      return traffic.map((sample) => ({
        timestamp_ms: sample.timestamp_ms,
        rate:
          sample.rates.down_bytes_per_second + sample.rates.up_bytes_per_second,
      }));
    const size = Math.ceil(traffic.length / 96);
    const bars: { timestamp_ms: number; rate: number }[] = [];
    for (let index = 0; index < traffic.length; index += size) {
      const group = traffic.slice(index, index + size);
      bars.push({
        timestamp_ms: group.at(-1)?.timestamp_ms ?? 0,
        rate:
          group.reduce(
            (sum, sample) =>
              sum +
              sample.rates.down_bytes_per_second +
              sample.rates.up_bytes_per_second,
            0,
          ) / Math.max(1, group.length),
      });
    }
    return bars;
  }, [traffic]);
  const trafficMaximum = Math.max(1, ...trafficBars.map((item) => item.rate));
  const trafficSource = traffic[0]?.source ?? "unavailable";
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
      <section className="traffic-timeline-panel">
        <header>
          <div>
            <span className="eyebrow">{t("TRAFFIC TIMELINE")}</span>
            <strong>
              {t("Stored rates / {source}", { source: trafficSource })}
            </strong>
          </div>
          <div className="traffic-timeline-panel__controls">
            <div className="segmented" aria-label={t("Traffic resolution")}>
              {(
                [
                  ["one_second", "1 s"],
                  ["one_minute", "1 min"],
                  ["one_hour", "1 h"],
                ] as const
              ).map(([value, label]) => (
                <button
                  aria-pressed={trafficResolution === value}
                  className={
                    trafficResolution === value ? "is-active" : undefined
                  }
                  key={value}
                  onClick={() => setTrafficResolution(value)}
                  type="button"
                >
                  {label}
                </button>
              ))}
            </div>
            <button
              aria-pressed={timelinePaused}
              className="button button--secondary"
              onClick={() => setTimelinePaused((value) => !value)}
              type="button"
            >
              {timelinePaused ? t("Resume timeline") : t("Pause timeline")}
            </button>
          </div>
        </header>
        <div
          aria-label={t("Traffic rates over selected time range")}
          className="traffic-timeline-panel__bars"
          role="img"
        >
          {trafficBars.map((bar, index) => (
            <span
              key={`${bar.timestamp_ms}-${index}`}
              style={{
                height: `${Math.max(2, (bar.rate / trafficMaximum) * 100)}%`,
              }}
              title={`${formatDateTime(bar.timestamp_ms)} / ${formatBytes(bar.rate)}/s`}
            />
          ))}
          {trafficBars.length === 0 && (
            <p>{t("No traffic samples in this range.")}</p>
          )}
        </div>
      </section>
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
      <section className="session-comparison">
        <header>
          <div>
            <span className="eyebrow">{t("SESSION COMPARISON")}</span>
            <h2>{t("Compare retained evidence")}</h2>
            <p>
              {t(
                "Compare processes, destinations, countries and ports without leaving local storage.",
              )}
            </p>
          </div>
          <div className="session-comparison__selectors">
            <select
              aria-label={t("Earlier session")}
              onChange={(event) => setLeftSession(event.target.value)}
              value={leftSession}
            >
              <option value="">{t("Earlier session")}</option>
              {(history?.sessions ?? []).map((session) => (
                <option key={session.id} value={session.id}>
                  {formatDateTime(session.started_at_ms)}
                </option>
              ))}
            </select>
            <select
              aria-label={t("Later session")}
              onChange={(event) => setRightSession(event.target.value)}
              value={rightSession}
            >
              <option value="">{t("Later session")}</option>
              {(history?.sessions ?? []).map((session) => (
                <option key={session.id} value={session.id}>
                  {formatDateTime(session.started_at_ms)}
                </option>
              ))}
            </select>
            <button
              className="button button--primary"
              disabled={
                !leftSession || !rightSession || leftSession === rightSession
              }
              onClick={() =>
                void desktopApi
                  .compareSessions(leftSession, rightSession)
                  .then(setComparison)
              }
              type="button"
            >
              {t("Compare sessions")}
            </button>
          </div>
        </header>
        {comparison && (
          <>
            <div className="session-comparison__toolbar">
              <div className="segmented" aria-label={t("Comparison category")}>
                {(
                  ["processes", "destinations", "countries", "ports"] as const
                ).map((category) => (
                  <button
                    aria-pressed={comparisonCategory === category}
                    className={
                      comparisonCategory === category ? "is-active" : undefined
                    }
                    key={category}
                    onClick={() => setComparisonCategory(category)}
                    type="button"
                  >
                    {t(category)}
                  </button>
                ))}
              </div>
              <div>
                {(["csv", "json"] as const).map((format) => (
                  <button
                    className="button button--secondary"
                    key={format}
                    onClick={() =>
                      void desktopApi
                        .exportSessionComparison({
                          left_session_id: leftSession,
                          right_session_id: rightSession,
                          format,
                          path: `sabicheiro-session-comparison-${Date.now()}.${format}`,
                        })
                        .then((result) => setComparisonExported(result.path))
                    }
                    type="button"
                  >
                    {t("Export {format}", { format: format.toUpperCase() })}
                  </button>
                ))}
              </div>
            </div>
            {comparisonExported && (
              <p className="session-comparison__exported">
                {t("Saved comparison to {path}", { path: comparisonExported })}
              </p>
            )}
            <div className="session-comparison__table" role="table">
              <div role="row">
                <strong role="columnheader">{t("Value")}</strong>
                <strong role="columnheader">{t("Earlier")}</strong>
                <strong role="columnheader">{t("Later")}</strong>
                <strong role="columnheader">Δ</strong>
              </div>
              {comparison[comparisonCategory].slice(0, 100).map((row) => (
                <div key={row.key} role="row">
                  <span role="cell">{row.key}</span>
                  <span role="cell">{row.left_count}</span>
                  <span role="cell">{row.right_count}</span>
                  <span
                    className={
                      row.delta > 0
                        ? "is-up"
                        : row.delta < 0
                          ? "is-down"
                          : undefined
                    }
                    role="cell"
                  >
                    {row.delta > 0 ? "+" : ""}
                    {row.delta}
                  </span>
                </div>
              ))}
            </div>
          </>
        )}
      </section>
    </section>
  );
}
