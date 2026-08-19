import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  CaptureMode,
  CaptureSelfTest,
  CaptureSummary,
  ComparisonExportRequest,
  Connection,
  ConnectionContextResponse,
  ConnectionHistoryStats,
  DashboardUpdate,
  DerivedGeoEnrichment,
  ExecutableInspectionStatus,
  ExportRequest,
  ExportResult,
  FocusRules,
  GeoDatabaseStatus,
  HistoryQuery,
  HistoryResponse,
  IdentityTimelineEvent,
  ImportedSabiBundle,
  ListenerSnapshot,
  LocalDataCategory,
  LocalDataRemovalResult,
  LocalNotificationRecord,
  LocalRule,
  NetworkInterface,
  NetworkProfile,
  NotificationState,
  RecordingRequest,
  RecordingState,
  ReenrichmentJobStatus,
  RetentionPolicy,
  RuleKind,
  RuleMatch,
  SabiExportRequest,
  SavedInvestigation,
  SessionComparison,
  StorageHealth,
  SupportBundleRequest,
  TimelineQuery,
  TodayCardState,
  TrafficSample,
} from "../types";

export const isDesktop = true;

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export const desktopApi = {
  getDashboardUpdate: (afterSequence: number | null) =>
    call<DashboardUpdate>("get_dashboard_update", { afterSequence }),
  setCaptureState: (mode: CaptureMode, selectedInterface: string | null) =>
    call<CaptureSummary>("set_capture_state", { request: { mode, selectedInterface } }),
  listInterfaces: () => call<NetworkInterface[]>("list_interfaces"),
  runCaptureSelfTest: () => call<CaptureSelfTest>("run_capture_self_test"),
  setViewPaused: (paused: boolean) => call<void>("set_view_paused", { paused }),
  clearSession: () => call<CaptureSummary>("clear_session"),
  queryHistory: (query: HistoryQuery) => call<HistoryResponse>("query_history", { query }),
  saveInvestigation: (investigation: SavedInvestigation) =>
    call<SavedInvestigation>("save_investigation", { investigation }),
  listInvestigations: () => call<SavedInvestigation[]>("list_investigations"),
  deleteInvestigation: (id: string) => call<boolean>("delete_investigation", { id }),
  saveLocalRule: (rule: LocalRule) => call<LocalRule>("save_local_rule", { rule }),
  listLocalRules: () => call<LocalRule[]>("list_local_rules"),
  deleteLocalRule: (id: string) => call<boolean>("delete_local_rule", { id }),
  previewLocalRule: (rule: LocalRule, query: HistoryQuery) =>
    call<RuleMatch[]>("preview_local_rule", { rule, query }),
  listLocalNotifications: (limit = 200) =>
    call<LocalNotificationRecord[]>("list_notifications", { limit }),
  updateLocalNotification: (
    id: string,
    notificationState: NotificationState,
    snoozedUntilMs: number | null,
  ) => call<boolean>("update_notification_state", { id, notificationState, snoozedUntilMs }),
  listNetworkProfiles: () => call<NetworkProfile[]>("list_network_profiles"),
  renameNetworkProfile: (id: string, displayName: string | null) =>
    call<NetworkProfile>("rename_network_profile", { id, displayName }),
  saveTodayCardState: (card: TodayCardState) =>
    call<TodayCardState>("save_today_card_state", { card }),
  listTodayCardStates: () => call<TodayCardState[]>("list_today_card_states"),
  captureListenerSnapshot: () => call<ListenerSnapshot>("capture_listener_snapshot"),
  listListenerSnapshots: (limit = 100) =>
    call<ListenerSnapshot[]>("list_listener_snapshots", { limit }),
  getConnectionContext: (connectionId: string) =>
    call<ConnectionContextResponse>("get_connection_context", { connectionId }),
  getIdentityTimeline: (connectionId: string, sessionId: string | null) =>
    call<IdentityTimelineEvent[]>("get_identity_timeline", { connectionId, sessionId }),
  queryTrafficTimeline: (query: TimelineQuery) =>
    call<TrafficSample[]>("query_traffic_timeline", { query }),
  compareSessions: (leftSessionId: string, rightSessionId: string) =>
    call<SessionComparison>("compare_sessions", { leftSessionId, rightSessionId }),
  exportSessionComparison: (request: ComparisonExportRequest) =>
    call<ExportResult>("export_session_comparison", {
      request: {
        leftSessionId: request.left_session_id,
        rightSessionId: request.right_session_id,
        format: request.format,
        path: request.path,
      },
    }),
  getSettings: () => call<AppSettings>("get_settings"),
  updateSettings: (settings: AppSettings) => call<AppSettings>("update_settings", { settings }),
  setRetention: (retention: RetentionPolicy) => call<number>("set_retention", { retention }),
  getFocusRules: () => call<FocusRules>("get_focus_rules"),
  addBookmark: (kind: Exclude<RuleKind, "localhost">, value: string) =>
    call<FocusRules["bookmarks"][number]>("add_bookmark", { kind, value }),
  removeBookmark: (id: number) => call<boolean>("remove_bookmark", { id }),
  addIgnoreRule: (kind: RuleKind, value: string) =>
    call<FocusRules["ignore_rules"][number]>("add_ignore_rule", { kind, value }),
  setIgnoreRuleEnabled: (id: number, enabled: boolean) =>
    call<boolean>("set_ignore_rule_enabled", { id, enabled }),
  removeIgnoreRule: (id: number) => call<boolean>("remove_ignore_rule", { id }),
  getConnectionHistoryStats: (remoteIp: string) =>
    call<ConnectionHistoryStats>("get_connection_history_stats", { remoteIp }),
  reconstructHistory: (atMs: number, sessionId: string | null, query: string | null) =>
    call<Connection[]>("reconstruct_history", { atMs, sessionId, query }),
  startSessionReenrichment: (sessionId: string) =>
    call<ReenrichmentJobStatus>("start_session_reenrichment", { sessionId }),
  getReenrichmentJob: (id: string) =>
    call<ReenrichmentJobStatus>("get_reenrichment_job", { id }),
  cancelReenrichmentJob: (id: string) =>
    call<boolean>("cancel_reenrichment_job", { id }),
  startExecutableInspection: (processInstance: string, maximumMegabytes = 256) =>
    call<ExecutableInspectionStatus>("start_executable_inspection", {
      processInstance,
      maximumMegabytes,
    }),
  getExecutableInspection: (id: string) =>
    call<ExecutableInspectionStatus>("get_executable_inspection", { id }),
  cancelExecutableInspection: (id: string) =>
    call<boolean>("cancel_executable_inspection", { id }),
  getSessionGeoView: (sessionId: string) =>
    call<DerivedGeoEnrichment[]>("get_session_geo_view", { sessionId }),
  compactStorage: () => call<StorageHealth>("compact_storage"),
  clearLocalData: (category: LocalDataCategory) =>
    call<LocalDataRemovalResult>("clear_local_data", { category }),
  getGeoStatus: () => call<GeoDatabaseStatus>("get_geo_database_status"),
  updateGeoDatabase: () => call<GeoDatabaseStatus>("update_geo_database"),
  importGeoDatabase: (sourcePath: string, kind: "city" | "asn") =>
    call<GeoDatabaseStatus>("import_geo_database", { sourcePath, kind }),
  rollbackGeoDatabase: (kind: "city" | "asn") =>
    call<GeoDatabaseStatus>("rollback_geo_database", { kind }),
  removeGeoDatabase: (kind: "city" | "asn") =>
    call<GeoDatabaseStatus>("remove_geo_database", { kind }),
  exportView: (request: ExportRequest) => call<ExportResult>("export_view", { request }),
  exportSabi: (request: SabiExportRequest) => call<ExportResult>("export_sabi", { request }),
  importSabi: (path: string) => call<ImportedSabiBundle>("import_sabi", { path }),
  exportSupportBundle: (request: SupportBundleRequest) =>
    call<ExportResult>("export_support_bundle", { request }),
  startRecording: (request: RecordingRequest) =>
    call<RecordingState>("start_recording", { request }),
  stopRecording: () => call<RecordingState>("stop_recording"),
  getRecordingState: () => call<RecordingState>("get_recording_state"),
  reportUiDiagnostics: (renderFps: number, droppedUiEvents: number) =>
    call<void>("report_ui_diagnostics", { renderFps, droppedUiEvents }),
};

export type DesktopApi = typeof desktopApi;
