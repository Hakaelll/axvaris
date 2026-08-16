import { useEffect, useMemo, useState } from "react";
import { useI18n } from "../i18n";
import type {
  HistoryQuery,
  HistoryResponse,
  ImportedSabiBundle,
  DerivedGeoEnrichment,
  ReenrichmentJobStatus,
  SavedInvestigation,
  SessionComparison,
  TimelineResolution,
  TrafficSample,
} from "../types";
import { formatBytes, formatDateTime, formatDuration } from "../lib/format";
import { desktopApi } from "../lib/backend";
import { EvidenceTable } from "../components/EvidenceTable";
import { QueryBuilder } from "../components/QueryBuilder";
import { ContextProfiles } from "../components/ContextProfiles";

interface HistoryViewProps {
  currentQuery: string;
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
  onQueryChange: (query: string) => void;
}

const ranges = [
  { label: "1 hour", milliseconds: 3_600_000 },
  { label: "24 hours", milliseconds: 86_400_000 },
  { label: "7 days", milliseconds: 604_800_000 },
  { label: "All", milliseconds: null },
] as const;

export function HistoryView({
  currentQuery,
  loadHistory,
  onReconstruct,
  onExport,
  onQueryChange,
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
  const [investigations, setInvestigations] = useState<SavedInvestigation[]>([]);
  const [investigationName, setInvestigationName] = useState("");
  const [investigationNotes, setInvestigationNotes] = useState("");
  const [investigationTags, setInvestigationTags] = useState("");
  const [activeInvestigation, setActiveInvestigation] =
    useState<SavedInvestigation | null>(null);
  const [sabiImportPath, setSabiImportPath] = useState("");
  const [importedBundle, setImportedBundle] =
    useState<ImportedSabiBundle | null>(null);
  const [sabiStatus, setSabiStatus] = useState<string | null>(null);
  const [pinnedConnectionIds, setPinnedConnectionIds] = useState<string[]>([]);
  const [investigationWindow, setInvestigationWindow] = useState<{
    from_ms: number | null;
    to_ms: number | null;
  } | null>(null);
  const [reenrichmentJob, setReenrichmentJob] =
    useState<ReenrichmentJobStatus | null>(null);
  const [geoView, setGeoView] = useState<DerivedGeoEnrichment[]>([]);

  useEffect(() => {
    void desktopApi.listInvestigations().then(setInvestigations);
  }, []);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    const fromMs = investigationWindow
      ? investigationWindow.from_ms
      : selectedSession || range.milliseconds === null
        ? null
        : Date.now() - range.milliseconds;
    void loadHistory({
      from_ms: fromMs,
      to_ms: investigationWindow?.to_ms ?? null,
      session_id: selectedSession,
      query: currentQuery || null,
      limit: 100_000,
    }).then(
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
  }, [currentQuery, investigationWindow, loadHistory, range, selectedSession]);

  useEffect(() => {
    if (
      !reenrichmentJob ||
      (reenrichmentJob.state !== "queued" &&
        reenrichmentJob.state !== "running")
    )
      return;
    let cancelled = false;
    const timer = window.setInterval(() => {
      void desktopApi.getReenrichmentJob(reenrichmentJob.id).then((status) => {
        if (cancelled) return;
        setReenrichmentJob(status);
        if (status.state === "completed") {
          void desktopApi
            .getSessionGeoView(status.session_id)
            .then((view) => !cancelled && setGeoView(view));
        }
      });
    }, 500);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [reenrichmentJob?.id, reenrichmentJob?.state]);

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
  const pinnedEntries = (history?.entries ?? []).filter(({ connection }) =>
    pinnedConnectionIds.includes(connection.id),
  );
  const buildInvestigation = (
    existing: SavedInvestigation | null,
  ): SavedInvestigation => ({
    id: existing?.id ?? "",
    name: investigationName.trim(),
    notes: investigationNotes.trim(),
    tags: investigationTags
      .split(",")
      .map((tag) => tag.trim())
      .filter(Boolean),
    selected_connection_id: pinnedConnectionIds[0] ?? null,
    layout_json: JSON.stringify({
      range,
      selectedSession,
      scroll: "evidence-table",
    }),
    autosave_revision: existing?.autosave_revision ?? 0,
    query: {
      filter: currentQuery || null,
      from_ms: bounds?.start ?? null,
      to_ms: bounds?.end ?? null,
      session_id: selectedSession,
      pinned_connection_ids: pinnedConnectionIds,
      pinned_destination_ips: [
        ...new Set(
          pinnedEntries.map(({ connection }) => connection.key.remote.address),
        ),
      ],
      pinned_process_keys: [
        ...new Set(
          pinnedEntries.flatMap(({ connection }) =>
            connection.key.process_instance
              ? [connection.key.process_instance]
              : [],
          ),
        ),
      ],
      pinned_asns: [
        ...new Set(
          pinnedEntries.flatMap(({ connection }) =>
            connection.asn ? [connection.asn.number] : [],
          ),
        ),
      ],
    },
    created_at_ms: existing?.created_at_ms ?? 0,
    updated_at_ms: existing?.updated_at_ms ?? 0,
  });
  const saveInvestigation = async () => {
    const name = investigationName.trim();
    if (!name) return;
    const saved = await desktopApi.saveInvestigation(
      buildInvestigation(activeInvestigation),
    );
    setInvestigations((items) => [
      saved,
      ...items.filter((item) => item.id !== saved.id),
    ]);
    setActiveInvestigation(saved);
  };
  const openInvestigation = (investigation: SavedInvestigation) => {
    setSelectedSession(investigation.query.session_id);
    setRange(ranges[3]);
    setInvestigationWindow({
      from_ms: investigation.query.from_ms,
      to_ms: investigation.query.to_ms,
    });
    setPinnedConnectionIds(investigation.query.pinned_connection_ids);
    setInvestigationName(investigation.name);
    setInvestigationNotes(investigation.notes);
    setInvestigationTags(investigation.tags.join(", "));
    setActiveInvestigation(investigation);
    onQueryChange(investigation.query.filter ?? "");
  };
  const deleteInvestigation = async (id: string) => {
    if (!window.confirm(t("Delete this saved local investigation?"))) return;
    if (await desktopApi.deleteInvestigation(id)) {
      setInvestigations((items) => items.filter((item) => item.id !== id));
      if (activeInvestigation?.id === id) setActiveInvestigation(null);
    }
  };
  const startReenrichment = async () => {
    if (!selectedSession) return;
    setGeoView([]);
    setReenrichmentJob(
      await desktopApi.startSessionReenrichment(selectedSession),
    );
  };

  useEffect(() => {
    if (!activeInvestigation?.id || !investigationName.trim()) return;
    const timer = window.setTimeout(() => {
      void desktopApi
        .saveInvestigation(buildInvestigation(activeInvestigation))
        .then((saved) => {
          setInvestigations((items) => [
            saved,
            ...items.filter((item) => item.id !== saved.id),
          ]);
        });
    }, 700);
    return () => window.clearTimeout(timer);
  }, [
    activeInvestigation?.id,
    bounds?.end,
    bounds?.start,
    currentQuery,
    investigationName,
    investigationNotes,
    investigationTags,
    pinnedConnectionIds,
    range,
    selectedSession,
  ]);

  const exportSabi = async (investigation: SavedInvestigation) => {
    const safeName = investigation.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "investigation";
    const result = await desktopApi.exportSabi({
      path: `${safeName}-${Date.now()}.sabi`,
      investigation_id: investigation.id,
      redaction: {
        local_ip: true,
        executable_path: true,
        user_names: true,
      },
    });
    setSabiStatus(t("Exported {count} records to {path}", {
      count: result.records,
      path: result.path,
    }));
  };

  const importSabi = async () => {
    const path = sabiImportPath.trim();
    if (!path) return;
    const imported = await desktopApi.importSabi(path);
    setImportedBundle(imported);
    setSabiStatus(t("Verified read-only bundle with {count} connections.", {
      count: imported.connections.length,
    }));
  };

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
                  setInvestigationWindow(null);
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
      <section className="investigation-panel">
        <header>
          <div>
            <span className="eyebrow">{t("SAVED INVESTIGATIONS")}</span>
            <h2>{t("Local investigation workspaces")}</h2>
            <p>
              {t(
                "Save the active query, exact time range, session, pinned evidence and notes on this device. Nothing is synced.",
              )}
            </p>
          </div>
          <code>{currentQuery || t("No active filter")}</code>
        </header>
        <div className="investigation-panel__editor">
          <input
            aria-label={t("Investigation name")}
            onChange={(event) => setInvestigationName(event.target.value)}
            placeholder={t("Investigation name")}
            value={investigationName}
          />
          <textarea
            aria-label={t("Local notes")}
            onChange={(event) => setInvestigationNotes(event.target.value)}
            placeholder={t("Local notes")}
            rows={2}
            value={investigationNotes}
          />
          <input
            aria-label={t("Investigation tags")}
            onChange={(event) => setInvestigationTags(event.target.value)}
            placeholder={t("Tags separated by commas")}
            value={investigationTags}
          />
          <div>
            <button
              className="button button--secondary"
              disabled={(history?.entries.length ?? 0) === 0}
              onClick={() =>
                setPinnedConnectionIds(
                  (history?.entries ?? []).map(
                    ({ connection }) => connection.id,
                  ),
                )
              }
              type="button"
            >
              {t("Pin visible evidence")}
            </button>
            <button
              className="button button--secondary"
              disabled={pinnedConnectionIds.length === 0}
              onClick={() => setPinnedConnectionIds([])}
              type="button"
            >
              {t("Clear pins")}
            </button>
            <button
              className="button button--primary"
              disabled={!investigationName.trim()}
              onClick={() => void saveInvestigation()}
              type="button"
            >
              {activeInvestigation
                ? t("Save now")
                : t("Save investigation")}
            </button>
            <button
              className="button button--secondary"
              onClick={() => {
                setActiveInvestigation(null);
                setInvestigationName("");
                setInvestigationNotes("");
                setInvestigationTags("");
                setPinnedConnectionIds([]);
              }}
              type="button"
            >
              {t("New workspace")}
            </button>
          </div>
          <small>
            {t("{count} pinned connections", {
              count: pinnedConnectionIds.length,
            })}
          </small>
        </div>
        <div className="investigation-list">
          {investigations.map((investigation) => (
            <article key={investigation.id}>
              <div>
                <strong>{investigation.name}</strong>
                <small>
                  {formatDateTime(investigation.updated_at_ms)} · {investigation.query.pinned_connection_ids.length} {t("pins")}
                </small>
                <code>
                  {investigation.query.filter ?? t("No active filter")}
                </code>
                {investigation.notes && <p>{investigation.notes}</p>}
                {investigation.tags.length > 0 && (
                  <span className="investigation-tags">
                    {investigation.tags.map((tag) => (
                      <em key={tag}>{tag}</em>
                    ))}
                  </span>
                )}
              </div>
              <span>
                <button
                  className="button button--secondary"
                  onClick={() => openInvestigation(investigation)}
                  type="button"
                >
                  {t("Open exact view")}
                </button>
                <button
                  className="button button--secondary"
                  onClick={() => void exportSabi(investigation)}
                  type="button"
                >
                  {t("Export redacted .sabi")}
                </button>
                <button
                  aria-label={t("Delete investigation {name}", {
                    name: investigation.name,
                  })}
                  className="button button--danger"
                  onClick={() => void deleteInvestigation(investigation.id)}
                  type="button"
                >
                  {t("Delete")}
                </button>
              </span>
            </article>
          ))}
          {investigations.length === 0 && (
            <p className="list-empty">{t("No saved investigations yet.")}</p>
          )}
        </div>
        <div className="sabi-import-panel">
          <div>
            <strong>{t("Open portable investigation")}</strong>
            <p>{t("The ZIP manifest, exact entry set, CRC and SHA-256 hashes are verified before opening. Imported evidence stays read-only and is never merged or uploaded.")}</p>
          </div>
          <input
            aria-label={t(".sabi file path")}
            onChange={(event) => setSabiImportPath(event.target.value)}
            placeholder="C:\\Evidence\\investigation.sabi"
            value={sabiImportPath}
          />
          <button className="button button--secondary" disabled={!sabiImportPath.trim()} onClick={() => void importSabi()} type="button">
            {t("Verify and open read-only")}
          </button>
          {sabiStatus && <small role="status">{sabiStatus}</small>}
        </div>
        {importedBundle && (
          <section className="sabi-readonly" aria-label={t("Imported read-only investigation")}>
            <header>
              <div><strong>{importedBundle.investigation.name}</strong><small>{t("READ-ONLY .SABI / schema {version}", { version: importedBundle.manifest.schema_version })}</small></div>
              <button className="button button--secondary" onClick={() => setImportedBundle(null)} type="button">{t("Close bundle")}</button>
            </header>
            <p>{importedBundle.investigation.notes || t("No local notes in this bundle.")}</p>
            <dl>
              <div><dt>{t("Source path")}</dt><dd><code>{importedBundle.source_path}</code></dd></div>
              <div><dt>{t("Connections")}</dt><dd>{importedBundle.connections.length}</dd></div>
              <div><dt>{t("Timeline samples")}</dt><dd>{importedBundle.timeline.length}</dd></div>
              <div><dt>{t("Verified entries")}</dt><dd>{Object.keys(importedBundle.manifest.entry_sha256).length}</dd></div>
            </dl>
          </section>
        )}
      </section>
      <QueryBuilder
        connections={(history?.entries ?? []).map((entry) => entry.connection)}
        onChange={onQueryChange}
        query={currentQuery}
        rangeLabel={
          selectedSession
            ? t("Selected retained session")
            : investigationWindow
              ? t("Saved exact time window")
              : t(range.label)
        }
      />
      <ContextProfiles
        entries={history?.entries ?? []}
        onFilter={onQueryChange}
      />
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
      <EvidenceTable
        entries={history?.entries ?? []}
        loading={loading}
        onOpen={({ session_id: sessionId, connection }) =>
          onReconstruct(
            Math.min(connection.last_seen_ms, connection.first_seen_ms + 1),
            sessionId,
            connection.id,
          )
        }
        onTogglePin={(connectionId) =>
          setPinnedConnectionIds((items) =>
            items.includes(connectionId)
              ? items.filter((id) => id !== connectionId)
              : [...items, connectionId],
          )
        }
        pinnedIds={pinnedConnectionIds}
      />
      <div className="history-layout">
        <div className="history-sessions">
          <h2>{t("Sessions")}</h2>
          <button
            aria-pressed={selectedSession === null}
            className={`history-session${selectedSession === null ? " is-selected" : ""}`}
            onClick={() => {
              setSelectedSession(null);
              setInvestigationWindow(null);
            }}
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
              onClick={() => {
                setSelectedSession(session.id);
                setInvestigationWindow(null);
                setReenrichmentJob(null);
                setGeoView([]);
              }}
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
      <section className="reenrichment-panel">
        <header>
          <div>
            <span className="eyebrow">{t("NON-DESTRUCTIVE GEOIP")}</span>
            <h2>{t("Compare captured location with the current database")}</h2>
            <p>
              {t(
                "A cancelable local job creates a derived view. Original IPs, timestamps, evidence and observed enrichment are never overwritten.",
              )}
            </p>
          </div>
          <div>
            <button
              className="button button--primary"
              disabled={
                !selectedSession ||
                reenrichmentJob?.state === "queued" ||
                reenrichmentJob?.state === "running"
              }
              onClick={() => void startReenrichment()}
              type="button"
            >
              {t("Re-enrich selected session")}
            </button>
            {(reenrichmentJob?.state === "queued" ||
              reenrichmentJob?.state === "running") && (
              <button
                className="button button--secondary"
                onClick={() =>
                  void desktopApi.cancelReenrichmentJob(reenrichmentJob.id)
                }
                type="button"
              >
                {t("Cancel job")}
              </button>
            )}
          </div>
        </header>
        {!selectedSession && (
          <p className="list-empty">
            {t("Select one retained session to enable re-enrichment.")}
          </p>
        )}
        {reenrichmentJob && (
          <div className="reenrichment-progress" role="status">
            <span>
              <strong>{t(reenrichmentJob.state)}</strong>
              <small>
                {reenrichmentJob.processed}/{reenrichmentJob.total}
              </small>
            </span>
            <progress
              max={Math.max(1, reenrichmentJob.total)}
              value={reenrichmentJob.processed}
            />
            {reenrichmentJob.message && <p>{t(reenrichmentJob.message)}</p>}
          </div>
        )}
        {geoView.length > 0 && (
          <div className="geo-view-comparison">
            <header>
              <strong>{t("Observed versus current GeoIP")}</strong>
              <small>
                {t("{changed} of {total} derived rows changed", {
                  changed: geoView.filter(
                    (item) =>
                      item.observed_geo?.country_code !==
                        item.current_geo?.country_code ||
                      item.observed_geo?.city !== item.current_geo?.city ||
                      item.observed_asn?.number !== item.current_asn?.number,
                  ).length,
                  total: geoView.length,
                })}
              </small>
            </header>
            <div role="table">
              {geoView.slice(0, 100).map((item) => (
                <div key={item.connection_id} role="row">
                  <code>{item.connection_id}</code>
                  <span>
                    <small>{t("At capture")}</small>
                    <strong>
                      {item.observed_geo?.city ??
                        item.observed_geo?.country_name ??
                        t("Unknown")}
                    </strong>
                    <em>
                      {item.observed_asn
                        ? `AS${item.observed_asn.number}`
                        : t("No ASN")}
                    </em>
                  </span>
                  <span>
                    <small>{t("Current database")}</small>
                    <strong>
                      {item.current_geo?.city ??
                        item.current_geo?.country_name ??
                        t("Unknown")}
                    </strong>
                    <em>
                      {item.current_asn
                        ? `AS${item.current_asn.number}`
                        : t("No ASN")}
                    </em>
                  </span>
                </div>
              ))}
            </div>
            {geoView.length > 100 && (
              <small>
                {t("Showing the first 100 of {count} rows.", {
                  count: geoView.length,
                })}
              </small>
            )}
          </div>
        )}
      </section>
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
