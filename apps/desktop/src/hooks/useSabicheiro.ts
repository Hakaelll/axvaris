import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { desktopApi } from "../lib/backend";
import {
  isListenerOnly,
  isLocalAddress,
  matchesQuery,
  parseQuery,
} from "../lib/filter";
import type {
  AppSettings,
  CaptureMode,
  CommandFailure,
  Connection,
  DashboardSnapshot,
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
    last_error: null,
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
  selected_interface: null,
  home_latitude: 40.4168,
  home_longitude: -3.7038,
  ui_language: "en",
  demo_seed: 1518458081,
  feed_group_window_seconds: 30,
};

const emptyFocusRules: FocusRules = { bookmarks: [], ignore_rules: [] };

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
        return connection.geo?.country_code.toLowerCase() === value;
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
    if (connection.geo) summary.countryCodes.add(connection.geo.country_code);
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
      processes: new Set<string>(),
    };
    summary.active_connections += connection.active ? 1 : 0;
    summary.total_connections += connection.repeat_count;
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

export function useSabicheiro() {
  const [liveSnapshot, setLiveSnapshot] = useState(emptySnapshot);
  const [visualSnapshot, setVisualSnapshot] = useState(emptySnapshot);
  const [settings, setSettings] = useState(defaultSettings);
  const [interfaces, setInterfaces] = useState<NetworkInterface[]>([]);
  const [geoStatus, setGeoStatus] = useState<GeoDatabaseStatus | null>(null);
  const [recording, setRecording] = useState<RecordingState | null>(null);
  const [focusRules, setFocusRules] = useState<FocusRules>(emptyFocusRules);
  const [timeMachine, setTimeMachine] = useState<TimeMachineState | null>(null);
  const [primaryView, setPrimaryView] = useState<PrimaryView>("globe");
  const [viewMode, setViewMode] = useState<ViewMode>("globe");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [paused, setPausedState] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [recordDialogOpen, setRecordDialogOpen] = useState(false);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const pausedRevision = useRef(0);

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      desktopApi.getSettings(),
      desktopApi.listInterfaces(),
      desktopApi.getGeoStatus(),
      desktopApi.getRecordingState(),
      desktopApi.getFocusRules(),
    ])
      .then(
        ([
          nextSettings,
          nextInterfaces,
          nextGeo,
          nextRecording,
          nextFocusRules,
        ]) => {
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
          setGeoStatus(nextGeo);
          setRecording(nextRecording);
          setFocusRules(nextFocusRules);
          if (hasStaleInterface) {
            void desktopApi.updateSettings(normalizedSettings).catch(() => {
              // Capture remains safely scoped to all interfaces even if persisting
              // this one-time migration fails.
            });
          }
        },
      )
      .catch((cause: unknown) => setError(normalizeError(cause)));
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const next = await desktopApi.getSnapshot();
        if (cancelled) return;
        setLiveSnapshot(next);
        if (!paused) setVisualSnapshot(next);
        setLoading(false);
      } catch (cause) {
        if (!cancelled) {
          setError(normalizeError(cause));
          setLoading(false);
        }
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 400);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [paused]);

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
  const connections = useMemo(
    () =>
      (timeMachine?.connections ?? visualSnapshot.connections).filter(
        (connection) =>
          (settings.show_listening_sockets || !isListenerOnly(connection)) &&
          (isListenerOnly(connection) ||
            settings.show_local_traffic ||
            !isLocalAddress(connection.key.remote.address)) &&
          !ignoredByRule(connection, focusRules) &&
          matchesQuery(connection, parsedQuery),
      ),
    [
      focusRules,
      parsedQuery,
      settings.show_listening_sockets,
      settings.show_local_traffic,
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
          limit: 2_000,
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
        setPrimaryView("globe");
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
  };
}
