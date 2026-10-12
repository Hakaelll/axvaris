import { invoke } from "@tauri-apps/api/core";
import type { DesktopApi } from "./backend-contract";
import type {
  DnsImportRequest,
  DnsImportPreview,
  DnsImportReport,
  ExternalDnsFilter,
  ExternalDnsPage,
  AppSettings,
  CaptureSelfTest,
  CaptureMode,
  CaptureSummary,
  ConnectionHistoryStats,
  ConnectionContextResponse,
  Connection,
  ComparisonExportRequest,
  DashboardUpdate,
  DashboardSnapshot,
  DerivedGeoEnrichment,
  DestinationSummary,
  ExportRequest,
  ExportResult,
  ExecutableInspectionStatus,
  FocusRules,
  GeoDatabaseStatus,
  HistoryQuery,
  HistoryResponse,
  IdentityTimelineEvent,
  ImportedSabiBundle,
  ListenerSnapshot,
  LocalDataCategory,
  LocalDataRemovalResult,
  LocalNeighbor,
  NetworkInterface,
  NetworkProfile,
  NotificationState,
  ProcessSummary,
  ReenrichmentJobStatus,
  RecordingRequest,
  RecordingState,
  RetentionPolicy,
  RuleKind,
  SavedInvestigation,
  SabiExportRequest,
  LocalNotificationRecord,
  LocalRule,
  RuleMatch,
  SessionComparison,
  StorageHealth,
  SupportBundleRequest,
  TimelineQuery,
  TrafficSample,
  TrafficCounters,
  TodayCardState,
  TodayBootstrap,
} from "../types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isDesktop =
  typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;

const zeroTraffic = (): TrafficCounters => ({
  bytes_sent: 0,
  bytes_received: 0,
  packets_sent: 0,
  packets_received: 0,
});

const previewDestinations = [
  [
    "Demo Browser",
    4812,
    "demo-browser.exe",
    "192.0.2.10",
    "portal.invalid",
    "ES",
    "Spain",
    "Madrid",
    40.4168,
    -3.7038,
    64496,
    "Example Network A",
    "tcp",
    443,
  ],
  [
    "Demo Browser",
    4812,
    "demo-browser.exe",
    "198.51.100.20",
    "media.invalid",
    "US",
    "United States",
    "New York",
    40.7128,
    -74.006,
    64497,
    "Example Network B",
    "udp",
    443,
  ],
  [
    "Demo Updater",
    7204,
    "demo-updater.exe",
    "203.0.113.30",
    "updates.invalid",
    "JP",
    "Japan",
    "Tokyo",
    35.6762,
    139.6503,
    64498,
    "Example Network C",
    "tcp",
    443,
  ],
  [
    "Demo Updater",
    7204,
    "demo-updater.exe",
    "192.0.2.40",
    "status.invalid",
    "DE",
    "Germany",
    "Frankfurt",
    50.1109,
    8.6821,
    64499,
    "Example Network D",
    "udp",
    53,
  ],
] as const;

let previewMode: CaptureMode = "off";
let previewPaused = false;
let previewStarted = Date.now();
let previewSequence = 0;
let previewSettings: AppSettings = {
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
let previewFocusRules: FocusRules = { bookmarks: [], ignore_rules: [] };
let previewInvestigations: SavedInvestigation[] = [];
let previewLocalRules: LocalRule[] = [];
let previewLocalNotifications: LocalNotificationRecord[] = [];
let previewTodayCards: TodayCardState[] = [];
let previewNetworkProfiles: NetworkProfile[] = [
  {
    id: "network-preview-home",
    display_name: "Home",
    vpn_observed: false,
    created_at_ms: previewStarted,
    sensitive_fields_redacted: true,
  },
];
let previewRuleId = 1;
let previewRecording: RecordingState = {
  status: "idle",
  output_path: null,
  started_at_ms: null,
  bytes_written: 0,
  max_file_size_mb: 512,
  duration_limit_seconds: null,
  message: null,
};

function previewConnections(now: number): Connection[] {
  const elapsed = Math.max(0, now - previewStarted);
  const cycle = Math.floor(elapsed / 1_200);
  return previewDestinations.map((item, index) => {
    const [
      process,
      pid,
      executable,
      ip,
      domain,
      countryCode,
      countryName,
      city,
      latitude,
      longitude,
      asn,
      organization,
      protocol,
      port,
    ] = item;
    const active = (cycle + index) % 9 !== 8;
    const pulse = 0.65 + 0.35 * Math.sin(elapsed / 850 + index * 0.9);
    const downRate = Math.max(0, (index + 1) * 82_000 * pulse);
    const upRate = Math.max(0, (index + 1) * 13_000 * (1.15 - pulse / 2));
    const firstSeen = previewStarted - (index + 1) * 91_000;
    const unusual =
      index === 6
        ? [
            {
              kind: "new_country" as const,
              title: "Unusual for this process",
              reason: "First observed connection from Spotify to Sweden.",
              typical: "Previously observed: DE, NL, US",
            },
          ]
        : [];
    return {
      id: `preview-${index}`,
      key: {
        pid,
        process_instance: `pid:${pid}:start:${previewStarted - 7_200_000}`,
        local: {
          address: "192.168.1.42",
          port: 45000 + index * 137,
          observed_family: "ipv4",
          scope: "private",
          zone: null,
        },
        remote: {
          address: ip,
          port,
          observed_family: "ipv4",
          scope: "global",
          zone: null,
        },
        protocol,
        interface_id: "wifi",
        capture_generation: 1,
        flow_generation: index + 1,
      },
      evidence: {
        endpoint_role: "remote_peer",
        role_reason: "connected_global_peer",
        observation_source: "demo",
        correlation_quality: "confirmed",
        observed_at_ms: firstSeen,
        sequence: index + 1,
      },
      process: {
        pid,
        name: process,
        executable,
        executable_path: `C:\\Program Files\\${executable}`,
        started_at_ms: previewStarted - 7_200_000,
        icon_data_url: null,
        parent_pid: null,
        parent_instance: null,
        user_id: "preview-user",
        session_id: 1,
        cgroup: null,
        network_namespace: null,
        executable_revision_id: `preview-revision-${executable}`,
      },
      protocol,
      direction: "outbound",
      state: active ? "established" : "closed",
      interface: "Wi-Fi",
      hostname: domain,
      domain,
      geo: {
        country_code: countryCode,
        country_name: countryName,
        region: null,
        city,
        latitude,
        longitude,
        source: "Axvaris deterministic development preview",
        precision: "coordinates",
        placement: "database_coordinate",
        accuracy_radius_km: 24,
        dataset_kind: "city",
        dataset_edition: "Axvaris-City-Preview",
        dataset_build_epoch_ms: 1_776_000_000_000,
        dataset_sha256: "preview-only",
        matched_prefix: 24,
        city_generation: 1,
        asn_generation: 1,
      },
      asn: {
        number: asn,
        organization,
        dataset_edition: "Axvaris-ASN-Preview",
        dataset_build_epoch_ms: 1_776_000_000_000,
        dataset_sha256: "preview-only",
        matched_prefix: 24,
        generation: 1,
      },
      traffic: {
        bytes_sent: Math.round(upRate * (44 + index * 11)),
        bytes_received: Math.round(downRate * (44 + index * 11)),
        packets_sent: Math.round(upRate / 1_100),
        packets_received: Math.round(downRate / 1_100),
      },
      counter_source: "demo",
      rates: {
        down_bytes_per_second: active ? downRate : 0,
        up_bytes_per_second: active ? upRate : 0,
        packets_per_second: active ? (downRate + upRate) / 1_100 : 0,
      },
      first_seen_ms: firstSeen,
      last_seen_ms: active ? now : now - 4_000,
      duration_ms: now - firstSeen,
      active,
      first_seen_destination: index === 6,
      repeat_count: 1 + ((cycle + index * 3) % 19),
      unusual,
      visual_expires_ms: active ? null : now + 4_000,
    };
  });
}

function summarizeProcesses(connections: Connection[]): ProcessSummary[] {
  const values = new Map<
    string,
    ProcessSummary & { countrySet: Set<string> }
  >();
  for (const connection of connections) {
    const key = `${connection.process.name}:${connection.process.pid ?? "unknown"}`;
    const current = values.get(key) ?? {
      key,
      process: connection.process,
      active_connections: 0,
      total_connections: 0,
      traffic: zeroTraffic(),
      countries: 0,
      countrySet: new Set<string>(),
    };
    current.active_connections += connection.active ? 1 : 0;
    current.total_connections += 1;
    current.traffic.bytes_sent += connection.traffic.bytes_sent;
    current.traffic.bytes_received += connection.traffic.bytes_received;
    current.traffic.packets_sent += connection.traffic.packets_sent;
    current.traffic.packets_received += connection.traffic.packets_received;
    if (connection.geo?.country_code)
      current.countrySet.add(connection.geo.country_code);
    current.countries = current.countrySet.size;
    values.set(key, current);
  }
  return [...values.values()]
    .map(({ countrySet: _countrySet, ...summary }) => summary)
    .sort(
      (left, right) =>
        right.traffic.bytes_received - left.traffic.bytes_received,
    );
}

function summarizeDestinations(
  connections: Connection[],
): DestinationSummary[] {
  return connections.map((connection) => ({
    key: connection.domain ?? connection.key.remote.address,
    ip: connection.key.remote.address,
    domain: connection.domain,
    country_code: connection.geo?.country_code ?? null,
    country_name: connection.geo?.country_name ?? null,
    asn: connection.asn,
    active_connections: connection.active ? 1 : 0,
    total_connections: connection.repeat_count,
    process_names: [connection.process.name],
    traffic: connection.traffic,
    ports: [connection.key.remote.port],
    first_seen_ms: connection.first_seen_ms,
    last_seen_ms: connection.last_seen_ms,
    recurrence: connection.repeat_count,
    geo_precision: connection.geo?.precision ?? "unknown",
    placement: connection.geo?.placement ?? "not_plotted",
  }));
}

function makePreviewSnapshot(): DashboardSnapshot {
  const now = Date.now();
  const connections = previewMode === "off" ? [] : previewConnections(now);
  const processes = summarizeProcesses(connections);
  const destinations = summarizeDestinations(connections);
  const active = connections.filter((connection) => connection.active);
  const countries = new Set(
    active.flatMap((connection) => connection.geo?.country_code ?? []),
  );
  const captureStatus: DashboardSnapshot["capture"]["status"] =
    previewMode === "off" ? { state: "stopped" } : { state: "capturing" };
  return {
    generated_at_ms: now,
    revision: Math.floor((now - previewStarted) / 400),
    connections,
    processes,
    destinations,
    stats: {
      active_connections: active.length,
      down_bytes_per_second: active.reduce(
        (sum, connection) => sum + connection.rates.down_bytes_per_second,
        0,
      ),
      up_bytes_per_second: active.reduce(
        (sum, connection) => sum + connection.rates.up_bytes_per_second,
        0,
      ),
      packets_per_second: active.reduce(
        (sum, connection) => sum + connection.rates.packets_per_second,
        0,
      ),
      countries: countries.size,
      processes: processes.length,
      total_connections: connections.length,
    },
    diagnostics: {
      cpu_percent: previewMode === "off" ? 0.2 : 1.7,
      memory_bytes: 68_812_800,
      render_fps: 60,
      capture_events_per_second: previewMode === "off" ? 0 : 18.4,
      database_size_bytes: 2_281_472,
      capture_queue_depth: 0,
      capture_queue_capacity: 1024,
      dropped_capture_events: 0,
      dropped_ui_events: 0,
      queue_saturation: 0,
      backend: previewMode === "off" ? "stopped" : "deterministic-demo",
      capture_event_source:
        previewMode === "off" ? "none" : "deterministic_demo",
      flow_counters_available: previewMode !== "off",
      packet_events_observed:
        previewMode === "off" ? 0 : Math.floor((now - previewStarted) / 120),
      dns_correlations_observed: previewMode === "off" ? 0 : 8,
      correlated_capture_events: previewMode === "off" ? 0 : 128,
      approximate_capture_events: 0,
      orphan_capture_events: 0,
      capture_generation: previewMode === "off" ? 0 : 1,
      sequence_gaps: 0,
      snapshot_resyncs: 0,
      dashboard_requests: Math.max(
        1,
        Math.floor((now - previewStarted) / 5_000),
      ),
      dashboard_bytes_serialized: 48_000,
      dashboard_empty_deltas: Math.max(
        0,
        Math.floor((now - previewStarted) / 7_500),
      ),
      dashboard_overlaps: 0,
      storage_quick_check_ok: true,
      storage_last_check_ms: now,
      storage_wal_pages: 0,
      last_error: null,
      subsystems: [
        {
          subsystem: "capture",
          status: previewMode === "off" ? "disabled" : "healthy",
          since_ms: previewStarted,
          message:
            previewMode === "off"
              ? "Capture is stopped"
              : "Deterministic preview is capturing",
          action: previewMode === "off" ? "Start Demo capture." : null,
          queue_depth: 0,
          queue_capacity: 0,
          dropped_events: 0,
        },
        {
          subsystem: "storage",
          status: "healthy",
          since_ms: previewStarted,
          message: "SQLite quick_check passed",
          action: null,
          queue_depth: 0,
          queue_capacity: 0,
          dropped_events: 0,
        },
      ],
    },
    capture: {
      mode: previewMode,
      status: captureStatus,
      selected_interface: previewSettings.selected_interface,
      paused_view_events: previewPaused ? 37 : 0,
      vpn_active: false,
    },
    session:
      previewMode === "off"
        ? null
        : {
            id: `preview-session-${previewStarted}`,
            started_at_ms: previewStarted,
            ended_at_ms: null,
            connection_count: connections.length,
            process_count: processes.length,
            country_count: countries.size,
            destination_count: destinations.length,
            recording_count: previewRecording.started_at_ms === null ? 0 : 1,
            ephemeral: previewSettings.ephemeral_capture,
            traffic: connections.reduce<TrafficCounters>(
              (total, connection) => ({
                bytes_sent: total.bytes_sent + connection.traffic.bytes_sent,
                bytes_received:
                  total.bytes_received + connection.traffic.bytes_received,
                packets_sent:
                  total.packets_sent + connection.traffic.packets_sent,
                packets_received:
                  total.packets_received + connection.traffic.packets_received,
              }),
              zeroTraffic(),
            ),
          },
  };
}

const previewInterfaces: NetworkInterface[] = [
  {
    id: "all",
    stable_id: "all",
    name: "All interfaces",
    system_name: "all",
    description: "Observe every available interface",
    luid: null,
    network_guid: null,
    addresses: [],
    is_up: true,
    is_loopback: false,
    is_virtual: false,
    is_vpn: false,
  },
  {
    id: "wifi",
    stable_id: "windows-adapter:preview-wifi",
    name: "Wi-Fi",
    system_name: "preview-wifi",
    description: "Wireless network interface",
    luid: 101,
    network_guid: "00000000-0000-0000-0000-000000000101",
    addresses: ["192.168.1.42"],
    is_up: true,
    is_loopback: false,
    is_virtual: false,
    is_vpn: false,
  },
  {
    id: "ethernet",
    stable_id: "windows-adapter:preview-ethernet",
    name: "Ethernet",
    system_name: "preview-ethernet",
    description: "Wired network interface",
    luid: 102,
    network_guid: "00000000-0000-0000-0000-000000000102",
    addresses: [],
    is_up: false,
    is_loopback: false,
    is_virtual: false,
    is_vpn: false,
  },
];

async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return invoke<T>(command, args);
}

export const desktopApi = {
  async previewDnsImport(request: DnsImportRequest): Promise<DnsImportPreview> {
    if (isDesktop) return call("preview_dns_import", { request });
    throw new Error(
      "DNS imports require the desktop application. The browser preview does not persist network evidence.",
    );
  },
  async importDnsQueries(
    request: DnsImportRequest,
    jobId: string,
  ): Promise<DnsImportReport> {
    return call("import_dns_queries", { request, jobId });
  },
  async cancelDnsImport(jobId: string): Promise<boolean> {
    return isDesktop ? call("cancel_dns_import", { jobId }) : false;
  },
  async queryExternalDns(filter: ExternalDnsFilter): Promise<ExternalDnsPage> {
    return isDesktop
      ? call("query_external_dns", { filter })
      : { rows: [], next_id: null };
  },
  async getDashboardUpdate(
    afterSequence: number | null,
  ): Promise<DashboardUpdate> {
    if (isDesktop) return call("get_dashboard_update", { afterSequence });
    const snapshot = makePreviewSnapshot();
    const expected = previewSequence;
    previewSequence += 1;
    const resync = afterSequence !== expected;
    return {
      sequence: previewSequence,
      base_revision: Math.max(0, snapshot.revision - 1),
      revision: snapshot.revision,
      generated_at_ms: snapshot.generated_at_ms,
      resync_required: resync,
      snapshot: resync ? snapshot : null,
      upserted_connections: resync ? [] : snapshot.connections,
      removed_connection_ids: [],
      processes: snapshot.processes,
      destinations: snapshot.destinations,
      stats: snapshot.stats,
      diagnostics: snapshot.diagnostics,
      capture: snapshot.capture,
      session: snapshot.session,
    };
  },
  async setCaptureState(
    mode: CaptureMode,
    selectedInterface: string | null,
  ): Promise<CaptureSummary> {
    if (isDesktop)
      return call("set_capture_state", {
        request: { mode, selectedInterface },
      });
    previewMode = mode;
    previewStarted = Date.now();
    previewSettings.selected_interface = selectedInterface;
    return makePreviewSnapshot().capture;
  },
  async listInterfaces(): Promise<NetworkInterface[]> {
    return isDesktop ? call("list_interfaces") : previewInterfaces;
  },
  async listLocalNeighbors(): Promise<LocalNeighbor[]> {
    if (isDesktop) return call("list_local_neighbors");
    return [
      {
        address: "192.168.1.1",
        mac_address: "02:00:00:00:00:01",
        interface_id: "wifi",
        is_router: true,
        state: "reachable",
      },
      {
        address: "192.168.1.50",
        mac_address: "02:00:00:00:00:50",
        interface_id: "wifi",
        is_router: false,
        state: "reachable",
      },
      {
        address: "fe80::50",
        mac_address: "02:00:00:00:00:50",
        interface_id: "wifi",
        is_router: false,
        state: "stale",
      },
    ];
  },
  async runCaptureSelfTest(): Promise<CaptureSelfTest> {
    if (isDesktop) return call("run_capture_self_test");
    return {
      generated_at_ms: Date.now(),
      checks: [
        {
          id: "interfaces",
          level: "pass",
          title: "Network interfaces are discoverable",
          detail: "Development preview adapter is available.",
          diagnostic_code: null,
          action: null,
        },
        {
          id: "socket_table",
          level: "pass",
          title: "Socket tables are readable",
          detail: "Development preview uses deterministic local data.",
          diagnostic_code: null,
          action: null,
        },
        {
          id: "dns",
          level: "pass",
          title: "Passive DNS correlation is active",
          detail: "Preview hostnames are deterministic and local.",
          diagnostic_code: null,
          action: null,
        },
        {
          id: "geoip",
          level: "pass",
          title: "Local GeoIP database is ready",
          detail: "Development preview dataset.",
          diagnostic_code: null,
          action: null,
        },
      ],
      coverage: ["tcp", "udp"].flatMap((protocol) =>
        ["ipv4", "ipv6"].map((family) => ({
          protocol: protocol as "tcp" | "udp",
          family: family as "ipv4" | "ipv6",
          endpoint: "pass" as const,
          process: "pass" as const,
          interface: "pass" as const,
          close_event: "pass" as const,
          limitation: null,
        })),
      ),
      redacted_report: "preview: PASS - deterministic local fixture",
    };
  },
  async setViewPaused(paused: boolean): Promise<void> {
    if (isDesktop) return call("set_view_paused", { paused });
    previewPaused = paused;
  },
  async clearSession(): Promise<CaptureSummary> {
    if (isDesktop) return call("clear_session");
    previewStarted = Date.now();
    return makePreviewSnapshot().capture;
  },
  async queryHistory(query: HistoryQuery): Promise<HistoryResponse> {
    if (isDesktop) return call("query_history", { query });
    const snapshot = makePreviewSnapshot();
    return {
      entries: snapshot.connections.map((connection) => ({
        session_id: snapshot.session?.id ?? "preview-session",
        connection,
      })),
      sessions: snapshot.session
        ? [{ ...snapshot.session, ended_at_ms: Date.now() }]
        : [],
      recordings:
        previewRecording.started_at_ms === null ||
        previewRecording.output_path === null
          ? []
          : [
              {
                id: 1,
                session_id: snapshot.session?.id ?? null,
                path: previewRecording.output_path,
                started_at_ms: previewRecording.started_at_ms,
                ended_at_ms:
                  previewRecording.status === "recording" ? null : Date.now(),
                bytes_written: previewRecording.bytes_written,
                status: previewRecording.status,
              },
            ],
      next_cursor: null,
      has_more: false,
    };
  },
  async saveInvestigation(
    investigation: SavedInvestigation,
  ): Promise<SavedInvestigation> {
    if (isDesktop) return call("save_investigation", { investigation });
    const saved = {
      ...structuredClone(investigation),
      id: investigation.id || `investigation-${Date.now()}`,
      created_at_ms: investigation.created_at_ms || Date.now(),
      updated_at_ms: Date.now(),
      autosave_revision: investigation.autosave_revision + 1,
    };
    previewInvestigations = [
      saved,
      ...previewInvestigations.filter((item) => item.id !== saved.id),
    ];
    return structuredClone(saved);
  },
  async listInvestigations(): Promise<SavedInvestigation[]> {
    return isDesktop
      ? call("list_investigations")
      : structuredClone(previewInvestigations);
  },
  async deleteInvestigation(id: string): Promise<boolean> {
    if (isDesktop) return call("delete_investigation", { id });
    const previous = previewInvestigations.length;
    previewInvestigations = previewInvestigations.filter(
      (item) => item.id !== id,
    );
    return previous !== previewInvestigations.length;
  },
  async saveLocalRule(rule: LocalRule): Promise<LocalRule> {
    if (isDesktop) return call("save_local_rule", { rule });
    previewLocalRules = [
      structuredClone(rule),
      ...previewLocalRules.filter((item) => item.id !== rule.id),
    ];
    return structuredClone(rule);
  },
  async deleteLocalRule(id: string): Promise<boolean> {
    if (isDesktop) return call("delete_local_rule", { id });
    const previous = previewLocalRules.length;
    previewLocalRules = previewLocalRules.filter((item) => item.id !== id);
    return previous !== previewLocalRules.length;
  },
  async previewLocalRule(
    rule: LocalRule,
    query: HistoryQuery,
  ): Promise<RuleMatch[]> {
    if (isDesktop) return call("preview_local_rule", { rule, query });
    return makePreviewSnapshot()
      .connections.filter((connection) =>
        rule.conditions.every((condition) =>
          condition.field === "port"
            ? connection.key.remote.port === condition.value
            : condition.field === "process"
              ? connection.process.name.toLowerCase() ===
                condition.value.toLowerCase()
              : true,
        ),
      )
      .map((connection) => ({
        rule_id: rule.id,
        rule_version: rule.version,
        connection_id: connection.id,
        matched: true,
        reasons: ["Development preview match"],
        actions: rule.actions,
      }));
  },
  async updateLocalNotification(
    id: string,
    notificationState: NotificationState,
    snoozedUntilMs: number | null,
  ): Promise<boolean> {
    if (isDesktop)
      return call("update_notification_state", {
        id,
        notificationState,
        snoozedUntilMs,
      });
    const item = previewLocalNotifications.find((entry) => entry.id === id);
    if (!item) return false;
    item.state = notificationState;
    item.snoozed_until_ms = snoozedUntilMs;
    return true;
  },
  async listNetworkProfiles(): Promise<NetworkProfile[]> {
    if (isDesktop) return call("list_network_profiles");
    return structuredClone(previewNetworkProfiles);
  },
  async renameNetworkProfile(
    id: string,
    displayName: string | null,
  ): Promise<NetworkProfile> {
    if (isDesktop) return call("rename_network_profile", { id, displayName });
    const profile = (await this.listNetworkProfiles()).find(
      (item) => item.id === id,
    );
    if (!profile) throw new Error("Network profile not found");
    const updated = { ...profile, display_name: displayName?.trim() || null };
    previewNetworkProfiles = [
      updated,
      ...previewNetworkProfiles.filter((item) => item.id !== id),
    ];
    return structuredClone(updated);
  },
  async saveTodayCardState(card: TodayCardState): Promise<TodayCardState> {
    if (isDesktop) return call("save_today_card_state", { card });
    previewTodayCards = [
      structuredClone(card),
      ...previewTodayCards.filter((item) => item.id !== card.id),
    ];
    return structuredClone(card);
  },
  async getTodayBootstrap(): Promise<TodayBootstrap> {
    if (isDesktop) return call("get_today_bootstrap");
    const snapshot = makePreviewSnapshot();
    const limits = {
      examples_per_category: 3,
      comparison_rows_per_category: 8,
      card_states: 500,
      profiles: 32,
      rules: 200,
      notifications: 50,
    };
    const examples = snapshot.connections.slice(
      0,
      limits.examples_per_category,
    );
    const group = (
      kind: TodayBootstrap["changes"][number]["kind"],
      values: typeof examples,
      primary: (connection: Connection) => string,
      secondary: (connection: Connection) => string | null,
      query: (connection: Connection) => string | null,
    ): TodayBootstrap["changes"][number] => ({
      kind,
      count: values.length,
      examples: values.map((connection) => ({
        id: `${kind}:${connection.id}`,
        kind,
        primary: primary(connection),
        secondary: secondary(connection),
        connection_id: connection.id,
        query: query(connection),
        target: "investigate",
        observed_at_ms: connection.last_seen_ms,
      })),
    });
    const comparison = await this.compareSessions(
      "preview-previous-session",
      snapshot.session?.id ?? "preview-current-session",
    );
    const currentTraffic = snapshot.session?.traffic ?? zeroTraffic();
    return {
      generated_at_ms: Date.now(),
      history_scope: "all_retained",
      limits,
      changes: [
        group(
          "process",
          examples.slice(0, 2),
          (connection) => connection.process.name,
          () => null,
          (connection) => `process:${JSON.stringify(connection.process.name)}`,
        ),
        group(
          "application_destination",
          examples,
          (connection) => connection.domain ?? connection.key.remote.address,
          (connection) => connection.process.name,
          (connection) =>
            JSON.stringify(connection.domain ?? connection.key.remote.address),
        ),
        group(
          "domain",
          examples,
          (connection) => connection.domain ?? connection.key.remote.address,
          (connection) => connection.process.name,
          (connection) =>
            connection.domain
              ? `domain:${JSON.stringify(connection.domain)}`
              : null,
        ),
        group(
          "asn",
          examples,
          (connection) => `AS${connection.asn?.number ?? 0}`,
          (connection) => connection.asn?.organization ?? null,
          (connection) =>
            connection.asn ? `asn:${connection.asn.number}` : null,
        ),
        group(
          "country",
          examples,
          (connection) => connection.geo?.country_name ?? "Unknown geography",
          (connection) => connection.geo?.country_code ?? null,
          (connection) =>
            connection.geo?.country_code
              ? `country:${connection.geo.country_code}`
              : null,
        ),
        group(
          "executable_revision",
          examples,
          (connection) =>
            connection.process.executable_revision_id ?? "Unavailable",
          (connection) => connection.process.name,
          (connection) => `process:${JSON.stringify(connection.process.name)}`,
        ),
        { kind: "network_profile", count: 0, examples: [] },
        { kind: "listener_added", count: 0, examples: [] },
        { kind: "listener_removed", count: 0, examples: [] },
        { kind: "listener_widened", count: 0, examples: [] },
        { kind: "provider_degradation", count: 0, examples: [] },
      ],
      card_states: structuredClone(
        previewTodayCards.slice(0, limits.card_states),
      ),
      profiles: structuredClone(
        previewNetworkProfiles.slice(0, limits.profiles),
      ),
      rules: structuredClone(previewLocalRules.slice(0, limits.rules)),
      notifications: structuredClone(
        previewLocalNotifications.slice(0, limits.notifications),
      ),
      active_profile_id: previewNetworkProfiles[0]?.id ?? null,
      previous_session: {
        previous_session: {
          id: "preview-previous-session",
          started_at_ms: previewStarted - 3_600_000,
          ended_at_ms: previewStarted - 60_000,
          connection_count: 3,
          process_count: 2,
          country_count: 2,
          destination_count: 3,
          recording_count: 0,
          ephemeral: false,
          traffic: {
            bytes_sent: Math.round(currentTraffic.bytes_sent * 0.8),
            bytes_received: Math.round(currentTraffic.bytes_received * 0.7),
            packets_sent: 0,
            packets_received: 0,
          },
        },
        current_session_id: snapshot.session?.id ?? "preview-current-session",
        comparison,
        previous_traffic: {
          bytes_sent: Math.round(currentTraffic.bytes_sent * 0.8),
          bytes_received: Math.round(currentTraffic.bytes_received * 0.7),
          packets_sent: 0,
          packets_received: 0,
        },
        current_traffic: currentTraffic,
        traffic_comparable: true,
        traffic_limitation: null,
        listener_added: [],
        listener_removed: [],
        listener_widened: [],
      },
    };
  },
  async captureListenerSnapshot(): Promise<ListenerSnapshot> {
    if (isDesktop) return call("capture_listener_snapshot");
    return {
      id: 1,
      session_id: `preview-session-${previewStarted}`,
      observed_at_ms: Date.now(),
      snapshot_sha256: "preview-listener-snapshot",
      listeners: [],
    };
  },
  async listListenerSnapshots(limit = 100): Promise<ListenerSnapshot[]> {
    return isDesktop ? call("list_listener_snapshots", { limit }) : [];
  },
  async getConnectionContext(
    connectionId: string,
  ): Promise<ConnectionContextResponse> {
    if (isDesktop) return call("get_connection_context", { connectionId });
    const connection = makePreviewSnapshot().connections.find(
      (item) => item.id === connectionId,
    );
    if (!connection) throw new Error("Connection not found");
    return {
      connectionId,
      dns: connection.domain
        ? {
            confirmed: {
              answer_id: `preview-dns-${connection.id}`,
              query_name: connection.domain,
              quality: "confirmed",
              reason: "Same preview process instance and valid TTL",
            },
            candidates: [],
            limitation: null,
          }
        : null,
      path: {
        id: `preview-route-${connection.id}`,
        destination_prefix: connection.key.remote.address,
        prefix_length: 32,
        gateway_pseudonym: "gateway-preview",
        interface_id: connection.interface,
        metric: 25,
        network_generation: 1,
        profile_id: "network-preview-home",
        kind: "gateway",
        observed_at_ms: connection.last_seen_ms,
        source: "demo",
        quality: "snapshot_only",
        limitation: "Development preview route snapshot.",
      },
      listener: null,
      serviceHint: {
        observed_port: connection.key.remote.port,
        observed_protocol: connection.protocol,
        service_name: connection.key.remote.port === 443 ? "https" : "unknown",
        description: "Common local catalog hint",
        wording:
          "Habitual service; observed endpoint remains primary evidence.",
        catalog: {
          name: "IANA service names (preview)",
          snapshot_date: "2026-08-13",
          source: "IANA",
          license: "IANA licensing terms",
          sha256: "preview",
        },
      },
      registrableDomain: connection.domain,
    };
  },
  async getIdentityTimeline(
    connectionId: string,
    sessionId: string | null,
  ): Promise<IdentityTimelineEvent[]> {
    if (isDesktop)
      return call("get_identity_timeline", { connectionId, sessionId });
    const connection = makePreviewSnapshot().connections.find(
      (item) => item.id === connectionId,
    );
    if (!connection) return [];
    return [
      {
        id: `${connection.id}-opened`,
        timestamp_ms: connection.first_seen_ms,
        kind: "connection_opened",
        title: "Connection opened",
        detail: connection.evidence.role_reason,
        source: connection.evidence.observation_source,
        confidence: connection.evidence.correlation_quality,
        connection_id: connection.id,
        session_id: sessionId,
      },
      {
        id: `${connection.id}-geo`,
        timestamp_ms: connection.last_seen_ms,
        kind: "geo_enriched",
        title: "Estimated network location enriched",
        detail: connection.geo?.country_name ?? "Unknown",
        source: "local_mmdb",
        confidence: "correlated",
        connection_id: connection.id,
        session_id: sessionId,
      },
    ];
  },
  async queryTrafficTimeline(query: TimelineQuery): Promise<TrafficSample[]> {
    if (isDesktop) return call("query_traffic_timeline", { query });
    const bucket =
      query.resolution === "one_second"
        ? 1_000
        : query.resolution === "one_minute"
          ? 60_000
          : 3_600_000;
    const count = Math.min(
      120,
      Math.max(1, Math.floor((query.to_ms - query.from_ms) / bucket)),
    );
    const snapshot = makePreviewSnapshot();
    return Array.from({ length: count }, (_, index) => {
      const pulse = 0.55 + Math.sin(index / 4) * 0.25;
      const rates = {
        down_bytes_per_second: snapshot.stats.down_bytes_per_second * pulse,
        up_bytes_per_second:
          snapshot.stats.up_bytes_per_second * (1.1 - pulse / 2),
        packets_per_second: snapshot.stats.packets_per_second * pulse,
      };
      return {
        timestamp_ms: query.to_ms - (count - index - 1) * bucket,
        session_id: query.session_id,
        process_key: query.process_key,
        sample_interval_ms: bucket,
        rates,
        counters: {
          bytes_sent: Math.round(rates.up_bytes_per_second * (bucket / 1_000)),
          bytes_received: Math.round(
            rates.down_bytes_per_second * (bucket / 1_000),
          ),
          packets_sent: 0,
          packets_received: Math.round(
            rates.packets_per_second * (bucket / 1_000),
          ),
        },
        source: "demo",
      };
    });
  },
  async compareSessions(
    leftSessionId: string,
    rightSessionId: string,
  ): Promise<SessionComparison> {
    if (isDesktop)
      return call("compare_sessions", { leftSessionId, rightSessionId });
    const rows = (keys: string[]) =>
      [...new Set(keys)].slice(0, 8).map((key, index) => ({
        key,
        left_count: index + 1,
        right_count: index + 2,
        delta: 1,
      }));
    return {
      left_session_id: leftSessionId,
      right_session_id: rightSessionId,
      processes: rows(previewDestinations.map((item) => String(item[0]))),
      destinations: rows(previewDestinations.map((item) => String(item[4]))),
      countries: rows(previewDestinations.map((item) => String(item[5]))),
      ports: rows(previewDestinations.map((item) => String(item[13]))),
    };
  },
  async exportSessionComparison(
    request: ComparisonExportRequest,
  ): Promise<ExportResult> {
    if (isDesktop)
      return call("export_session_comparison", {
        request: {
          leftSessionId: request.left_session_id,
          rightSessionId: request.right_session_id,
          format: request.format,
          path: request.path,
        },
      });
    return { path: request.path, records: 14, bytes_written: 0 };
  },
  async getSettings(): Promise<AppSettings> {
    return isDesktop ? call("get_settings") : structuredClone(previewSettings);
  },
  async updateSettings(settings: AppSettings): Promise<AppSettings> {
    if (isDesktop) return call("update_settings", { settings });
    previewSettings = structuredClone(settings);
    return structuredClone(previewSettings);
  },
  async setRetention(retention: RetentionPolicy): Promise<number> {
    if (isDesktop) return call("set_retention", { retention });
    previewSettings.retention = retention;
    return 0;
  },
  async getFocusRules(): Promise<FocusRules> {
    return isDesktop
      ? call("get_focus_rules")
      : structuredClone(previewFocusRules);
  },
  async addBookmark(
    kind: Exclude<RuleKind, "localhost">,
    value: string,
  ): Promise<FocusRules["bookmarks"][number]> {
    if (isDesktop) return call("add_bookmark", { kind, value });
    const existing = previewFocusRules.bookmarks.find(
      (item) =>
        item.kind === kind &&
        item.value.toLowerCase() === value.trim().toLowerCase(),
    );
    if (existing) return structuredClone(existing);
    const bookmark = {
      id: previewRuleId++,
      kind,
      value: value.trim(),
      created_at_ms: Date.now(),
    };
    previewFocusRules.bookmarks.unshift(bookmark);
    return structuredClone(bookmark);
  },
  async removeBookmark(id: number): Promise<boolean> {
    if (isDesktop) return call("remove_bookmark", { id });
    const previous = previewFocusRules.bookmarks.length;
    previewFocusRules.bookmarks = previewFocusRules.bookmarks.filter(
      (item) => item.id !== id,
    );
    return previewFocusRules.bookmarks.length !== previous;
  },
  async addIgnoreRule(
    kind: RuleKind,
    value: string,
  ): Promise<FocusRules["ignore_rules"][number]> {
    if (isDesktop) return call("add_ignore_rule", { kind, value });
    const existing = previewFocusRules.ignore_rules.find(
      (item) =>
        item.kind === kind &&
        item.value.toLowerCase() === value.trim().toLowerCase(),
    );
    if (existing) {
      existing.enabled = true;
      return structuredClone(existing);
    }
    const rule = {
      id: previewRuleId++,
      kind,
      value: kind === "localhost" ? "localhost" : value.trim(),
      enabled: true,
      created_at_ms: Date.now(),
    };
    previewFocusRules.ignore_rules.unshift(rule);
    return structuredClone(rule);
  },
  async setIgnoreRuleEnabled(id: number, enabled: boolean): Promise<boolean> {
    if (isDesktop) return call("set_ignore_rule_enabled", { id, enabled });
    const rule = previewFocusRules.ignore_rules.find((item) => item.id === id);
    if (!rule) return false;
    rule.enabled = enabled;
    return true;
  },
  async removeIgnoreRule(id: number): Promise<boolean> {
    if (isDesktop) return call("remove_ignore_rule", { id });
    const previous = previewFocusRules.ignore_rules.length;
    previewFocusRules.ignore_rules = previewFocusRules.ignore_rules.filter(
      (item) => item.id !== id,
    );
    return previewFocusRules.ignore_rules.length !== previous;
  },
  async getConnectionHistoryStats(
    remoteIp: string,
  ): Promise<ConnectionHistoryStats> {
    if (isDesktop) return call("get_connection_history_stats", { remoteIp });
    const connection = makePreviewSnapshot().connections.find(
      (item) => item.key.remote.address === remoteIp,
    );
    return {
      first_ever_seen_ms: connection?.first_seen_ms ?? null,
      previous_connections: Math.max(0, (connection?.repeat_count ?? 1) - 1),
      sessions: connection ? 1 : 0,
    };
  },
  async reconstructHistory(
    atMs: number,
    sessionId: string | null,
    query: string | null,
  ): Promise<Connection[]> {
    if (isDesktop)
      return call("reconstruct_history", { atMs, sessionId, query });
    return makePreviewSnapshot()
      .connections.filter(
        (connection) =>
          connection.first_seen_ms <= atMs && connection.last_seen_ms >= atMs,
      )
      .map((connection) => ({ ...connection, active: true }));
  },
  async startSessionReenrichment(
    sessionId: string,
  ): Promise<ReenrichmentJobStatus> {
    if (isDesktop) return call("start_session_reenrichment", { sessionId });
    const total = makePreviewSnapshot().connections.length;
    return {
      id: `reenrichment-${Date.now()}`,
      session_id: sessionId,
      state: "completed",
      processed: total,
      total,
      started_at_ms: Date.now(),
      finished_at_ms: Date.now(),
      message: "Derived preview view saved; observations were unchanged.",
    };
  },
  async getReenrichmentJob(id: string): Promise<ReenrichmentJobStatus> {
    if (isDesktop) return call("get_reenrichment_job", { id });
    return {
      id,
      session_id: `preview-session-${previewStarted}`,
      state: "completed",
      processed: makePreviewSnapshot().connections.length,
      total: makePreviewSnapshot().connections.length,
      started_at_ms: Date.now(),
      finished_at_ms: Date.now(),
      message: "Preview job complete.",
    };
  },
  async cancelReenrichmentJob(id: string): Promise<boolean> {
    return isDesktop ? call("cancel_reenrichment_job", { id }) : true;
  },
  async startExecutableInspection(
    processInstance: string,
    maximumMegabytes = 256,
  ): Promise<ExecutableInspectionStatus> {
    if (isDesktop)
      return call("start_executable_inspection", {
        processInstance,
        maximumMegabytes,
      });
    const connection = makePreviewSnapshot().connections.find(
      (item) => item.key.process_instance === processInstance,
    );
    const now = Date.now();
    return {
      id: `preview-executable-${now}`,
      process_instance: processInstance,
      state: "completed",
      identity: {
        process_instance: processInstance,
        path:
          connection?.process.executable_path ??
          "preview executable path unavailable",
        revision_id:
          connection?.process.executable_revision_id ??
          `preview-revision-${now}`,
        size_bytes: 0,
        modified_at_ms: null,
        sha256: null,
        signature_status: "unsupported",
        package: null,
        format: "preview",
        computed_at_ms: now,
        limitation:
          "Development preview does not read or hash a local executable.",
      },
      started_at_ms: now,
      finished_at_ms: now,
      message:
        "Development preview identity completed without filesystem access.",
    };
  },
  async getExecutableInspection(
    id: string,
  ): Promise<ExecutableInspectionStatus> {
    if (isDesktop) return call("get_executable_inspection", { id });
    throw new Error("Preview executable inspection already completed");
  },
  async cancelExecutableInspection(id: string): Promise<boolean> {
    return isDesktop ? call("cancel_executable_inspection", { id }) : true;
  },
  async getSessionGeoView(sessionId: string): Promise<DerivedGeoEnrichment[]> {
    if (isDesktop) return call("get_session_geo_view", { sessionId });
    return makePreviewSnapshot().connections.map((connection) => ({
      connection_id: connection.id,
      observed_geo: connection.geo,
      observed_asn: connection.asn,
      current_geo: connection.geo,
      current_asn: connection.asn,
    }));
  },
  async compactStorage(): Promise<StorageHealth> {
    if (isDesktop) return call("compact_storage");
    return {
      quick_check_ok: true,
      detail: "ok",
      wal_busy_pages: 0,
      wal_log_pages: 0,
      wal_checkpointed_pages: 0,
      database_size_bytes: 2_281_472,
      checked_at_ms: Date.now(),
    };
  },
  async clearLocalData(
    category: LocalDataCategory,
  ): Promise<LocalDataRemovalResult> {
    if (isDesktop) return call("clear_local_data", { category });
    if (category === "sessions" || category === "all")
      previewStarted = Date.now();
    if (category === "all") {
      previewInvestigations = [];
      previewFocusRules = { bookmarks: [], ignore_rules: [] };
      previewLocalRules = [];
      previewLocalNotifications = [];
      previewTodayCards = [];
    }
    return {
      category,
      removed_items: 1,
      requires_capture_restart: category === "sessions" || category === "all",
    };
  },
  async getGeoStatus(): Promise<GeoDatabaseStatus> {
    if (isDesktop) return call("get_geo_database_status");
    return {
      installed: true,
      dataset: "DB-IP City Lite (development preview)",
      version: "2026-08",
      last_update_ms: Date.now() - 7 * 86_400_000,
      next_recommended_update_ms: Date.now() + 23 * 86_400_000,
      path: "Development preview only",
      edition: "DBIP-City-Lite",
      sha256: "preview-only",
      attribution: "IP geolocation by DB-IP.com, licensed under CC BY 4.0",
      city: {
        kind: "city",
        installed: true,
        dataset: "Axvaris-City-Preview",
        version: "2026-08",
        build_epoch_ms: 1_776_000_000_000,
        last_update_ms: Date.now() - 7 * 86_400_000,
        next_recommended_update_ms: Date.now() + 23 * 86_400_000,
        path: "Development preview only",
        edition: "Axvaris-City-Preview",
        sha256: "preview-only",
        generation: 1,
        backup_available: true,
        attribution: "Local deterministic preview dataset",
      },
      asn: {
        kind: "asn",
        installed: true,
        dataset: "Axvaris-ASN-Preview",
        version: "2026-08",
        build_epoch_ms: 1_776_000_000_000,
        last_update_ms: Date.now() - 7 * 86_400_000,
        next_recommended_update_ms: Date.now() + 23 * 86_400_000,
        path: "Development preview only",
        edition: "Axvaris-ASN-Preview",
        sha256: "preview-only",
        generation: 1,
        backup_available: true,
        attribution: "Local deterministic preview dataset",
      },
      last_operation: null,
    };
  },
  async updateGeoDatabase(): Promise<GeoDatabaseStatus> {
    return isDesktop ? call("update_geo_database") : this.getGeoStatus();
  },
  async importGeoDatabase(
    sourcePath: string,
    kind: "city" | "asn",
  ): Promise<GeoDatabaseStatus> {
    return isDesktop
      ? call("import_geo_database", { sourcePath, kind })
      : this.getGeoStatus();
  },
  async rollbackGeoDatabase(kind: "city" | "asn"): Promise<GeoDatabaseStatus> {
    return isDesktop
      ? call("rollback_geo_database", { kind })
      : this.getGeoStatus();
  },
  async removeGeoDatabase(kind: "city" | "asn"): Promise<GeoDatabaseStatus> {
    return isDesktop
      ? call("remove_geo_database", { kind })
      : this.getGeoStatus();
  },
  async exportView(request: ExportRequest): Promise<ExportResult> {
    if (isDesktop) return call("export_view", { request });
    return {
      path: request.path,
      records: makePreviewSnapshot().connections.length,
      bytes_written: 0,
    };
  },
  async exportSabi(request: SabiExportRequest): Promise<ExportResult> {
    if (isDesktop) return call("export_sabi", { request });
    return {
      path: request.path,
      records: makePreviewSnapshot().connections.length,
      bytes_written: 0,
    };
  },
  async importSabi(path: string): Promise<ImportedSabiBundle> {
    if (isDesktop) return call("import_sabi", { path });
    const investigation = previewInvestigations[0];
    if (!investigation) throw new Error("Create an investigation first");
    return {
      source_path: path,
      read_only: true,
      manifest: {
        schema_version: 1,
        app_version: "preview",
        created_at_ms: Date.now(),
        investigation_id: investigation.id,
        query: investigation.query,
        from_ms: investigation.query.from_ms,
        to_ms: investigation.query.to_ms,
        redaction: {
          local_ip: true,
          executable_path: true,
          user_names: true,
        },
        dataset_sha256: ["preview"],
        entry_sha256: {},
      },
      investigation: structuredClone(investigation),
      connections: (
        await this.queryHistory({
          from_ms: investigation.query.from_ms,
          to_ms: investigation.query.to_ms,
          session_id: investigation.query.session_id,
          query: investigation.query.filter,
          limit: 100_000,
        })
      ).entries,
      timeline: [],
    };
  },
  async exportSupportBundle(
    request: SupportBundleRequest,
  ): Promise<ExportResult> {
    if (isDesktop) return call("export_support_bundle", { request });
    return { path: request.path, records: 3, bytes_written: 0 };
  },
  async startRecording(request: RecordingRequest): Promise<RecordingState> {
    if (isDesktop) return call("start_recording", { request });
    previewRecording = {
      status: "recording",
      output_path: request.output_path,
      started_at_ms: Date.now(),
      bytes_written: 0,
      max_file_size_mb: request.max_file_size_mb,
      duration_limit_seconds: request.duration_limit_seconds,
      message: "Development preview recording",
    };
    return previewRecording;
  },
  async stopRecording(): Promise<RecordingState> {
    if (isDesktop) return call("stop_recording");
    previewRecording = {
      ...previewRecording,
      status: "idle",
      message: "Preview recording stopped",
    };
    return previewRecording;
  },
  async getRecordingState(): Promise<RecordingState> {
    return isDesktop ? call("get_recording_state") : previewRecording;
  },
  async reportUiDiagnostics(
    renderFps: number,
    droppedUiEvents: number,
  ): Promise<void> {
    if (isDesktop)
      return call("report_ui_diagnostics", { renderFps, droppedUiEvents });
  },
} satisfies DesktopApi;
