import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { desktopApi } from "#backend";
import {
  isListenerOnly,
  isLocalAddress,
  matchesQuery,
  parseQuery,
} from "../lib/filter";
import type {
  AppSettings,
  CaptureSelfTest,
  CaptureMode,
  CommandFailure,
  Connection,
  DashboardSnapshot,
  DashboardUpdate,
  ExportRequest,
  FocusRules,
  GeoDatabaseStatus,
  HistoryQuery,
  HistoryResponse,
  NetworkInterface,
  PrimaryView,
  RecordingRequest,
  RecordingState,
  RuleKind,
  SupportBundleRequest,
  TimeMachineState,
  ViewMode,
} from "../types";

const emptySnapshot: DashboardSnapshot = {
  generated_at_ms: Date.now(),
  revision: 0,
  connections: [],
  processes: [],
  destinations: [],
  stats: {
    active_connections: 0,
    down_bytes_per_second: 0,
    up_bytes_per_second: 0,
    packets_per_second: 0,
    countries: 0,
    processes: 0,
    total_connections: 0,
  },
  diagnostics: {
    cpu_percent: 0,
    memory_bytes: 0,
    render_fps: 0,
    capture_events_per_second: 0,
    database_size_bytes: 0,
    capture_queue_depth: 0,
    capture_queue_capacity: 1024,
    dropped_capture_events: 0,
    dropped_ui_events: 0,
    queue_saturation: 0,
    backend: "initializing",
    capture_event_source: "none",
    flow_counters_available: false,
    packet_events_observed: 0,
    dns_correlations_observed: 0,
    correlated_capture_events: 0,
    approximate_capture_events: 0,
    orphan_capture_events: 0,
    capture_generation: 0,
    sequence_gaps: 0,
    snapshot_resyncs: 0,
    dashboard_requests: 0,
    dashboard_bytes_serialized: 0,
    dashboard_empty_deltas: 0,
    dashboard_overlaps: 0,
    storage_quick_check_ok: true,
    storage_last_check_ms: null,
    storage_wal_pages: 0,
    last_error: null,
    subsystems: [],
  },
  capture: {
    mode: "off",
    status: { state: "stopped" },
    selected_interface: null,
    paused_view_events: 0,
    vpn_active: false,
  },
  session: null,
};

const defaultSettings: AppSettings = {
  retention: "seven_days",
  reduce_motion: false,
  auto_focus: true,
  show_local_traffic: false,
  show_listening_sockets: false,
  reverse_dns_enabled: false,
  offline_enforced: false,
  selected_interface: null,
  home_latitude: 40.4168,
  home_longitude: -3.7038,
  ui_language: "en",
  demo_seed: 1518458081,
  feed_group_window_seconds: 30,
  active_workspace: "remote",
  notification_rules: [],
  notification_quiet_hours_enabled: false,
  notification_quiet_start_minute: 1320,
  notification_quiet_end_minute: 420,
  ephemeral_capture: false,
  ui_density: "comfortable",
  operation_profile: "visible",
  autostart_enabled: false,
  handshake_metadata_enabled: false,
  enhanced_contrast: false,
  text_scale_percent: 100,
};

const emptyFocusRules: FocusRules = { bookmarks: [], ignore_rules: [] };

export interface LocalNotification {
  id: string;
  title: string;
  detail: string;
  created_at_ms: number;
}

function normalizeError(error: unknown): CommandFailure {
  if (typeof error === "object" && error !== null && "message" in error) {
    const candidate = error as Partial<CommandFailure>;
    return {
      code: candidate.code ?? "operation_failed",
      message: String(candidate.message),
      recoverable: candidate.recoverable ?? true,
    };
  }
  return {
    code: "operation_failed",
    message: String(error),
    recoverable: true,
  };
}

function filteredStats(connections: Connection[]) {
  const active = connections.filter((connection) => connection.active);
  return {
    active_connections: active.length,
    down_bytes_per_second: active.reduce(
      (sum, item) => sum + item.rates.down_bytes_per_second,
      0,
    ),
    up_bytes_per_second: active.reduce(
      (sum, item) => sum + item.rates.up_bytes_per_second,
      0,
    ),
    packets_per_second: active.reduce(
      (sum, item) => sum + item.rates.packets_per_second,
      0,
    ),
    countries: new Set(active.flatMap((item) => item.geo?.country_code ?? []))
      .size,
    processes: new Set(
      active.map((item) => `${item.process.name}:${item.process.pid ?? "?"}`),
    ).size,
    total_connections: connections.length,
  };
}

function ignoredByRule(connection: Connection, rules: FocusRules): boolean {
  return rules.ignore_rules.some((rule) => {
    if (!rule.enabled) return false;
    const value = rule.value.toLowerCase();
    switch (rule.kind) {
      case "process":
        return connection.process.name.toLowerCase() === value;
      case "domain": {
        const domain = (
          connection.domain ??
          connection.hostname ??
          ""
        ).toLowerCase();
        return domain === value || domain.endsWith(`.${value}`);
      }
      case "ip":
        return connection.key.remote.address.toLowerCase() === value;
      case "country":
        return connection.geo?.country_code?.toLowerCase() === value;
      case "localhost":
        return isLocalAddress(connection.key.remote.address);
    }
  });
}

function summarizeProcesses(
  connections: Connection[],
): DashboardSnapshot["processes"] {
  const summaries = new Map<
    string,
    DashboardSnapshot["processes"][number] & { countryCodes: Set<string> }
  >();
  for (const connection of connections) {
    const key = `${connection.process.name}:${connection.process.pid ?? "unknown"}`;
    const summary = summaries.get(key) ?? {
      key,
      process: connection.process,
      active_connections: 0,
      total_connections: 0,
      traffic: {
        bytes_sent: 0,
        bytes_received: 0,
        packets_sent: 0,
        packets_received: 0,
      },
      countries: 0,
      countryCodes: new Set<string>(),
    };
    summary.active_connections += connection.active ? 1 : 0;
    summary.total_connections += connection.repeat_count;
    summary.traffic.bytes_sent += connection.traffic.bytes_sent;
    summary.traffic.bytes_received += connection.traffic.bytes_received;
    summary.traffic.packets_sent += connection.traffic.packets_sent;
    summary.traffic.packets_received += connection.traffic.packets_received;
    if (connection.geo?.country_code)
      summary.countryCodes.add(connection.geo.country_code);
    summary.countries = summary.countryCodes.size;
    summaries.set(key, summary);
  }
  return [...summaries.values()]
    .map(({ countryCodes: _countryCodes, ...summary }) => summary)
    .sort(
      (left, right) =>
        right.active_connections - left.active_connections ||
        right.traffic.bytes_received - left.traffic.bytes_received,
    );
}

function summarizeDestinations(
  connections: Connection[],
): DashboardSnapshot["destinations"] {
  const summaries = new Map<
    string,
    DashboardSnapshot["destinations"][number] & { processes: Set<string> }
  >();
  for (const connection of connections) {
    const key =
      connection.domain ?? connection.hostname ?? connection.key.remote.address;
    const summary = summaries.get(key) ?? {
      key,
      ip: connection.key.remote.address,
      domain: connection.domain ?? connection.hostname,
      country_code: connection.geo?.country_code ?? null,
      country_name: connection.geo?.country_name ?? null,
      asn: connection.asn,
      active_connections: 0,
      total_connections: 0,
      process_names: [],
      traffic: {
        bytes_sent: 0,
        bytes_received: 0,
        packets_sent: 0,
        packets_received: 0,
      },
      ports: [],
      first_seen_ms: connection.first_seen_ms,
      last_seen_ms: connection.last_seen_ms,
      recurrence: 0,
      geo_precision: connection.geo?.precision ?? "unknown",
      placement: connection.geo?.placement ?? "not_plotted",
      processes: new Set<string>(),
    };
    summary.active_connections += connection.active ? 1 : 0;
    summary.total_connections += connection.repeat_count;
    if (!summary.ports.includes(connection.key.remote.port)) {
      summary.ports.push(connection.key.remote.port);
      summary.ports.sort((left, right) => left - right);
    }
    summary.first_seen_ms = Math.min(
      summary.first_seen_ms,
      connection.first_seen_ms,
    );
    summary.last_seen_ms = Math.max(
      summary.last_seen_ms,
      connection.last_seen_ms,
    );
    summary.recurrence += connection.repeat_count;
    summary.processes.add(connection.process.name);
    summary.process_names = [...summary.processes].sort();
    summary.traffic.bytes_sent += connection.traffic.bytes_sent;
    summary.traffic.bytes_received += connection.traffic.bytes_received;
    summary.traffic.packets_sent += connection.traffic.packets_sent;
    summary.traffic.packets_received += connection.traffic.packets_received;
    summaries.set(key, summary);
  }
  return [...summaries.values()]
    .map(({ processes: _processes, ...summary }) => summary)
    .sort(
      (left, right) =>
        right.active_connections - left.active_connections ||
        right.total_connections - left.total_connections,
    );
}

function mergeDashboardUpdate(
  current: DashboardSnapshot,
  update: DashboardUpdate,
): DashboardSnapshot | null {
  if (update.snapshot) return update.snapshot;
  if (update.resync_required || update.base_revision !== current.revision)
    return null;
  const hasConnectionChanges =
    update.removed_connection_ids.length > 0 ||
    update.upserted_connections.length > 0;
  if (
    !hasConnectionChanges &&
    update.processes === undefined &&
    update.destinations === undefined &&
    update.stats === undefined &&
    update.diagnostics === undefined &&
    update.capture === undefined &&
    update.session === undefined
  ) {
    return current;
  }
  const connections = new Map(
    current.connections.map((connection) => [connection.id, connection]),
  );
  for (const id of update.removed_connection_ids) connections.delete(id);
  for (const connection of update.upserted_connections)
    connections.set(connection.id, connection);
  return {
    generated_at_ms: update.generated_at_ms,
    revision: update.revision,
    connections: [...connections.values()],
    processes: update.processes ?? current.processes,
    destinations: update.destinations ?? current.destinations,
    stats: update.stats ?? current.stats,
    diagnostics: update.diagnostics ?? current.diagnostics,
    capture: update.capture ?? current.capture,
    session: update.session === undefined ? current.session : update.session,
  };
}

export function useSabicheiro() {
  const [liveSnapshot, setLiveSnapshot] = useState(emptySnapshot);
  const [visualSnapshot, setVisualSnapshot] = useState(emptySnapshot);
  const [settings, setSettings] = useState(defaultSettings);
  const [interfaces, setInterfaces] = useState<NetworkInterface[]>([]);
  const [geoStatus, setGeoStatus] = useState<GeoDatabaseStatus | null>(null);
  const [recording, setRecording] = useState<RecordingState | null>(null);
  const [focusRules, setFocusRules] = useState<FocusRules>(emptyFocusRules);
  const [timeMachine, setTimeMachine] = useState<TimeMachineState | null>(null);
  const [primaryView, setPrimaryView] = useState<PrimaryView>("today");
  const [viewMode, setViewMode] = useState<ViewMode>("globe");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [paused, setPausedState] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [recordDialogOpen, setRecordDialogOpen] = useState(false);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [notifications, setNotifications] = useState<LocalNotification[]>([]);
  const liveSnapshotRef = useRef<DashboardSnapshot>(emptySnapshot);
  const pausedRef = useRef(paused);
  const dashboardWakeRef = useRef<() => void>(() => undefined);
  const ipcSequence = useRef<number | null>(null);
  const pausedRevision = useRef(0);
  const settingsRef = useRef(settings);
  const notificationReady = useRef(false);
  const seenConnectionIds = useRef(new Set<string>());
  const seenProcesses = useRef(new Set<string>());
  const seenCountries = useRef(new Set<string>());
  const notificationCooldowns = useRef(new Map<string, number>());

  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  useEffect(() => {
    pausedRef.current = paused;
  }, [paused]);

  useEffect(() => {
    let cancelled = false;
    Promise.all([desktopApi.getSettings(), desktopApi.listInterfaces()])
      .then(([nextSettings, nextInterfaces]) => {
        if (cancelled) return;
        const hasStaleInterface =
          nextInterfaces.length > 0 &&
          nextSettings.selected_interface !== null &&
          !nextInterfaces.some(
            (item) => item.id === nextSettings.selected_interface,
          );
        const normalizedSettings = hasStaleInterface
          ? { ...nextSettings, selected_interface: null }
          : nextSettings;
        setSettings(normalizedSettings);
        setInterfaces(nextInterfaces);
        if (hasStaleInterface) {
          void desktopApi.updateSettings(normalizedSettings).catch(() => {
            // Capture remains safely scoped to all interfaces even if persisting
            // this one-time migration fails.
          });
        }
      })
      .catch((cause: unknown) => setError(normalizeError(cause)));
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let secondFrame = 0;
    const firstFrame = window.requestAnimationFrame(() => {
      secondFrame = window.requestAnimationFrame(() => {
        void Promise.allSettled([
          desktopApi.getGeoStatus().then((value) => !cancelled && setGeoStatus(value)),
          desktopApi
            .getRecordingState()
            .then((value) => !cancelled && setRecording(value)),
          desktopApi.getFocusRules().then((value) => !cancelled && setFocusRules(value)),
        ]).then((results) => {
          if (cancelled) return;
          const rejected = results.find(
            (result): result is PromiseRejectedResult => result.status === "rejected",
          );
          if (rejected) setError(normalizeError(rejected.reason));
        });
      });
    });
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(firstFrame);
      window.cancelAnimationFrame(secondFrame);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let timer = 0;
    let inFlight = false;
    let wakeRequested = false;
    const poll = async () => {
      if (cancelled) return;
      if (inFlight) {
        wakeRequested = true;
        return;
      }
      inFlight = true;
      try {
        const update = await desktopApi.getDashboardUpdate(ipcSequence.current);
        if (cancelled) return;
        const next = mergeDashboardUpdate(liveSnapshotRef.current, update);
        if (!next) {
          ipcSequence.current = null;
          return;
        }
        ipcSequence.current = update.sequence;
        if (next !== liveSnapshotRef.current) {
          liveSnapshotRef.current = next;
          setLiveSnapshot(next);
          if (!pausedRef.current) setVisualSnapshot(next);
        }
        const active = next.connections.filter(
          (connection) => connection.active,
        );
        if (!notificationReady.current || next.capture.mode === "off") {
          seenConnectionIds.current = new Set(active.map((item) => item.id));
          seenProcesses.current = new Set(
            active.map(
              (item) => `${item.process.name}:${item.process.pid ?? "?"}`,
            ),
          );
          seenCountries.current = new Set(
            active.flatMap((item) => item.geo?.country_code ?? []),
          );
          notificationReady.current = next.capture.mode !== "off";
        } else {
          const now = Date.now();
          const created: LocalNotification[] = [];
          for (const connection of active) {
            if (seenConnectionIds.current.has(connection.id)) continue;
            const processKey = `${connection.process.name}:${connection.process.pid ?? "?"}`;
            const isNewProcess = !seenProcesses.current.has(processKey);
            const country = connection.geo?.country_code ?? null;
            const isNewCountry = Boolean(
              country && !seenCountries.current.has(country),
            );
            for (const rule of settingsRef.current.notification_rules) {
              if (!rule.enabled) continue;
              const value = rule.value?.trim().toLowerCase() ?? "";
              const matches =
                (rule.kind === "new_process" && isNewProcess) ||
                (rule.kind === "new_country" && isNewCountry) ||
                (rule.kind === "port" &&
                  String(connection.key.remote.port) === value) ||
                (rule.kind === "asn" &&
                  Boolean(
                    connection.asn &&
                      (String(connection.asn.number) === value ||
                        (connection.asn.organization ?? "")
                          .toLowerCase()
                          .includes(value)),
                  ));
              const previous = notificationCooldowns.current.get(rule.id) ?? 0;
              if (
                !matches ||
                now - previous < Math.max(5, rule.cooldown_seconds) * 1_000
              )
                continue;
              notificationCooldowns.current.set(rule.id, now);
              const title =
                rule.kind === "new_process"
                  ? "New network process"
                  : rule.kind === "new_country"
                    ? "New destination country"
                    : rule.kind === "asn"
                      ? "ASN rule matched"
                      : "Port rule matched";
              const detail = `${connection.process.name} → ${connection.domain ?? connection.hostname ?? connection.key.remote.address}:${connection.key.remote.port}`;
              created.push({
                id: `${rule.id}-${connection.id}-${now}`,
                title,
                detail,
                created_at_ms: now,
              });
              if (
                typeof Notification !== "undefined" &&
                Notification.permission === "granted"
              ) {
                new Notification(`Sabicheiro / ${title}`, { body: detail });
              }
            }
            seenConnectionIds.current.add(connection.id);
            seenProcesses.current.add(processKey);
            if (country) seenCountries.current.add(country);
          }
          if (created.length > 0) {
            setNotifications((current) =>
              [...created, ...current].slice(0, 20),
            );
          }
        }
        setLoading(false);
      } catch (cause) {
        if (!cancelled) {
          setError(normalizeError(cause));
          setLoading(false);
        }
      } finally {
        inFlight = false;
        if (!cancelled) {
          const delay = wakeRequested
            ? 0
            : document.visibilityState !== "visible"
              ? 15_000
              : liveSnapshotRef.current.capture.mode === "off"
                ? 5_000
                : 400;
          wakeRequested = false;
          timer = window.setTimeout(() => void poll(), delay);
        }
      }
    };
    const wake = () => {
      if (cancelled) return;
      window.clearTimeout(timer);
      void poll();
    };
    const handleVisibility = () => {
      if (document.visibilityState === "visible") wake();
    };
    dashboardWakeRef.current = wake;
    document.addEventListener("visibilitychange", handleVisibility);
    void poll();
    return () => {
      cancelled = true;
      dashboardWakeRef.current = () => undefined;
      document.removeEventListener("visibilitychange", handleVisibility);
      window.clearTimeout(timer);
    };
  }, []);

  useEffect(() => {
    if (recording?.status !== "recording" && recording?.status !== "starting")
      return;
    let cancelled = false;
    const refresh = async () => {
      try {
        const next = await desktopApi.getRecordingState();
        if (!cancelled) setRecording(next);
      } catch (cause) {
        if (!cancelled) setError(normalizeError(cause));
      }
    };
    const timer = window.setInterval(() => void refresh(), 1_000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [recording?.status]);

  const parsedQuery = useMemo(() => parseQuery(query), [query]);
  const workspaceCounts = useMemo(() => {
    const values = timeMachine?.connections ?? visualSnapshot.connections;
    return values.reduce(
      (counts, connection) => {
        if (isListenerOnly(connection)) counts.listeners += 1;
        else if (isLocalAddress(connection.key.remote.address))
          counts.local += 1;
        else counts.remote += 1;
        return counts;
      },
      { remote: 0, local: 0, listeners: 0 },
    );
  }, [timeMachine?.connections, visualSnapshot.connections]);
  const connections = useMemo(
    () =>
      (timeMachine?.connections ?? visualSnapshot.connections).filter(
        (connection) => {
          const listener = isListenerOnly(connection);
          const local =
            !listener && isLocalAddress(connection.key.remote.address);
          const inWorkspace =
            settings.active_workspace === "listeners"
              ? listener
              : settings.active_workspace === "local"
                ? local
                : !listener && !local;
          return (
            inWorkspace &&
            !ignoredByRule(connection, focusRules) &&
            matchesQuery(connection, parsedQuery)
          );
        },
      ),
    [
      focusRules,
      parsedQuery,
      settings.active_workspace,
      timeMachine,
      visualSnapshot.connections,
    ],
  );
  const displaySnapshot = useMemo<DashboardSnapshot>(() => {
    const queued = paused
      ? Math.max(0, liveSnapshot.revision - pausedRevision.current)
      : 0;
    const stats = filteredStats(connections);
    if (liveSnapshot.capture.mode === "live") {
      stats.down_bytes_per_second = liveSnapshot.stats.down_bytes_per_second;
      stats.up_bytes_per_second = liveSnapshot.stats.up_bytes_per_second;
      stats.packets_per_second = liveSnapshot.stats.packets_per_second;
    }
    return {
      ...visualSnapshot,
      generated_at_ms: liveSnapshot.generated_at_ms,
      connections,
      processes: summarizeProcesses(connections),
      destinations: summarizeDestinations(connections),
      stats,
      diagnostics: liveSnapshot.diagnostics,
      capture: {
        ...liveSnapshot.capture,
        paused_view_events: queued,
      },
    };
  }, [connections, liveSnapshot, paused, visualSnapshot]);

  const selected = useMemo(
    () =>
      displaySnapshot.connections.find(
        (connection) => connection.id === selectedId,
      ) ?? null,
    [displaySnapshot.connections, selectedId],
  );

  const startCapture = useCallback(
    async (mode: CaptureMode, selectedInterface: string | null) => {
      try {
        await desktopApi.setCaptureState(mode, selectedInterface);
        setTimeMachine(null);
        setError(null);
        dashboardWakeRef.current();
      } catch (cause) {
        setError(normalizeError(cause));
      }
    },
    [],
  );

  const setPaused = useCallback(
    async (next: boolean) => {
      try {
        await desktopApi.setViewPaused(next);
        setPausedState(next);
        if (next) {
          pausedRevision.current = liveSnapshot.revision;
        } else {
          setVisualSnapshot(liveSnapshot);
        }
      } catch (cause) {
        setError(normalizeError(cause));
      }
    },
    [liveSnapshot],
  );

  const updateSettings = useCallback(
    async (next: AppSettings) => {
      try {
        if (next.retention !== settings.retention)
          await desktopApi.setRetention(next.retention);
        const saved = await desktopApi.updateSettings(next);
        setSettings(saved);
        setError(null);
      } catch (cause) {
        setError(normalizeError(cause));
      }
    },
    [settings.retention],
  );

  const addBookmark = useCallback(
    async (kind: Exclude<RuleKind, "localhost">, value: string) => {
      try {
        const bookmark = await desktopApi.addBookmark(kind, value);
        setFocusRules((current) => ({
          ...current,
          bookmarks: [
            bookmark,
            ...current.bookmarks.filter((item) => item.id !== bookmark.id),
          ],
        }));
        setError(null);
      } catch (cause) {
        setError(normalizeError(cause));
      }
    },
    [],
  );

  const removeBookmark = useCallback(async (id: number) => {
    try {
      if (await desktopApi.removeBookmark(id)) {
        setFocusRules((current) => ({
          ...current,
          bookmarks: current.bookmarks.filter((item) => item.id !== id),
        }));
      }
      setError(null);
    } catch (cause) {
      setError(normalizeError(cause));
    }
  }, []);

  const addIgnoreRule = useCallback(async (kind: RuleKind, value: string) => {
    try {
      const rule = await desktopApi.addIgnoreRule(kind, value);
      setFocusRules((current) => ({
        ...current,
        ignore_rules: [
          rule,
          ...current.ignore_rules.filter((item) => item.id !== rule.id),
        ],
      }));
      setSelectedId(null);
      setError(null);
    } catch (cause) {
      setError(normalizeError(cause));
    }
  }, []);

  const setIgnoreRuleEnabled = useCallback(
    async (id: number, enabled: boolean) => {
      try {
        if (await desktopApi.setIgnoreRuleEnabled(id, enabled)) {
          setFocusRules((current) => ({
            ...current,
            ignore_rules: current.ignore_rules.map((item) =>
              item.id === id ? { ...item, enabled } : item,
            ),
          }));
        }
        setError(null);
      } catch (cause) {
        setError(normalizeError(cause));
      }
    },
    [],
  );

  const removeIgnoreRule = useCallback(async (id: number) => {
    try {
      if (await desktopApi.removeIgnoreRule(id)) {
        setFocusRules((current) => ({
          ...current,
          ignore_rules: current.ignore_rules.filter((item) => item.id !== id),
        }));
      }
      setError(null);
    } catch (cause) {
      setError(normalizeError(cause));
    }
  }, []);

  const updateGeo = useCallback(async () => {
    try {
      setGeoStatus(await desktopApi.updateGeoDatabase());
      setError(null);
    } catch (cause) {
      setError(normalizeError(cause));
    }
  }, []);

  const importGeo = useCallback(
    async (sourcePath: string, kind: "city" | "asn") => {
      try {
        setGeoStatus(await desktopApi.importGeoDatabase(sourcePath, kind));
        setError(null);
      } catch (cause) {
        setError(normalizeError(cause));
      }
    },
    [],
  );

  const rollbackGeo = useCallback(async (kind: "city" | "asn") => {
    try {
      const status = await desktopApi.rollbackGeoDatabase(kind);
      setGeoStatus(status);
      setError(null);
      return status;
    } catch (cause) {
      setError(normalizeError(cause));
      return null;
    }
  }, []);

  const removeGeo = useCallback(async (kind: "city" | "asn") => {
    try {
      const status = await desktopApi.removeGeoDatabase(kind);
      setGeoStatus(status);
      setError(null);
      return status;
    } catch (cause) {
      setError(normalizeError(cause));
      return null;
    }
  }, []);

  const compactStorage = useCallback(async () => {
    try {
      const health = await desktopApi.compactStorage();
      setError(null);
      return health;
    } catch (cause) {
      setError(normalizeError(cause));
      return null;
    }
  }, []);

  const clearLocalData = useCallback(
    async (category: Parameters<typeof desktopApi.clearLocalData>[0]) => {
      try {
        const result = await desktopApi.clearLocalData(category);
        if (category === "geo_ip" || category === "all")
          setGeoStatus(await desktopApi.getGeoStatus());
        if (category === "sessions" || category === "all") {
          setSelectedId(null);
          setTimeMachine(null);
        }
        if (category === "all") {
          const defaults = await desktopApi.getSettings();
          setSettings(defaults);
          setFocusRules(await desktopApi.getFocusRules());
        }
        setError(null);
        return result;
      } catch (cause) {
        setError(normalizeError(cause));
        return null;
      }
    },
    [],
  );

  const runCaptureSelfTest =
    useCallback(async (): Promise<CaptureSelfTest | null> => {
      try {
        const result = await desktopApi.runCaptureSelfTest();
        setError(null);
        return result;
      } catch (cause) {
        setError(normalizeError(cause));
        return null;
      }
    }, []);

  const exportSupportBundle = useCallback(
    async (request: SupportBundleRequest) => {
      try {
        const result = await desktopApi.exportSupportBundle(request);
        setError(null);
        return result;
      } catch (cause) {
        setError(normalizeError(cause));
        return null;
      }
    },
    [],
  );

  const startRecording = useCallback(async (request: RecordingRequest) => {
    try {
      setRecording(await desktopApi.startRecording(request));
      setRecordDialogOpen(false);
      setError(null);
    } catch (cause) {
      setError(normalizeError(cause));
    }
  }, []);

  const stopRecording = useCallback(async () => {
    try {
      setRecording(await desktopApi.stopRecording());
      setError(null);
    } catch (cause) {
      setError(normalizeError(cause));
    }
  }, []);

  const exportView = useCallback(async (request: ExportRequest) => {
    try {
      const result = await desktopApi.exportView(request);
      setExportDialogOpen(false);
      setError(null);
      return result;
    } catch (cause) {
      setError(normalizeError(cause));
      return null;
    }
  }, []);

  const loadHistory = useCallback(
    async (
      overrides: Partial<HistoryQuery> = {},
    ): Promise<HistoryResponse | null> => {
      try {
        return await desktopApi.queryHistory({
          from_ms: null,
          to_ms: null,
          session_id: null,
          query: query || null,
          limit: 500,
          cursor: null,
          ...overrides,
        });
      } catch (cause) {
        setError(normalizeError(cause));
        return null;
      }
    },
    [query],
  );

  const enterTimeMachine = useCallback(
    async (
      atMs: number,
      sessionId: string | null,
      preferredConnectionId?: string,
    ) => {
      try {
        const reconstructed = await desktopApi.reconstructHistory(
          atMs,
          sessionId,
          query || null,
        );
        setTimeMachine({
          at_ms: atMs,
          connections: reconstructed,
          session_id: sessionId,
        });
      setPrimaryView("live");
        setSelectedId(
          reconstructed.some(
            (connection) => connection.id === preferredConnectionId,
          )
            ? (preferredConnectionId ?? null)
            : (reconstructed[0]?.id ?? null),
        );
        setError(null);
      } catch (cause) {
        setError(normalizeError(cause));
      }
    },
    [query],
  );

  const clearSession = useCallback(async () => {
    try {
      await desktopApi.clearSession();
      setSelectedId(null);
      setError(null);
    } catch (cause) {
      setError(normalizeError(cause));
    }
  }, []);

  return {
    snapshot: displaySnapshot,
    settings,
    interfaces,
    geoStatus,
    recording,
    focusRules,
    timeMachine,
    selected,
    selectedId,
    setSelectedId,
    primaryView,
    setPrimaryView,
    viewMode,
    setViewMode,
    query,
    setQuery,
    queryError: parsedQuery.error,
    paused,
    setPaused,
    loading,
    error,
    clearError: () => setError(null),
    startCapture,
    updateSettings,
    addBookmark,
    removeBookmark,
    addIgnoreRule,
    setIgnoreRuleEnabled,
    removeIgnoreRule,
    updateGeo,
    importGeo,
    rollbackGeo,
    removeGeo,
    compactStorage,
    clearLocalData,
    runCaptureSelfTest,
    exportSupportBundle,
    startRecording,
    stopRecording,
    exportView,
    loadHistory,
    enterTimeMachine,
    exitTimeMachine: () => setTimeMachine(null),
    clearSession,
    commandPaletteOpen,
    setCommandPaletteOpen,
    recordDialogOpen,
    setRecordDialogOpen,
    exportDialogOpen,
    setExportDialogOpen,
    notifications,
    dismissNotification: (id: string) =>
      setNotifications((current) => current.filter((item) => item.id !== id)),
    clearNotifications: () => setNotifications([]),
    workspaceCounts,
  };
}
