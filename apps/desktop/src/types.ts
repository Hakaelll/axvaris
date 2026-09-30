export type NetworkProtocol = "tcp" | "udp";
export type Direction = "inbound" | "outbound" | "unknown";
export type ConnectionState =
  | "listening"
  | "connecting"
  | "established"
  | "closing"
  | "closed"
  | "unknown";
export type CaptureMode = "off" | "demo" | "live";
export type ViewMode = "globe" | "map";
export type AddressFamily = "ipv4" | "ipv6";
export type AddressScope =
  | "global"
  | "private"
  | "loopback"
  | "link_local"
  | "multicast"
  | "cgnat"
  | "documentation"
  | "benchmark"
  | "reserved"
  | "broadcast"
  | "unspecified";
export type PrimaryView =
  | "today"
  | "live"
  | "investigate"
  | "exposure"
  | "health";

export type CaptureStatus =
  | { state: "stopped" | "starting" | "capturing" }
  | { state: "error"; message: string };

export interface Endpoint {
  address: string;
  port: number;
  observed_family: AddressFamily;
  scope: AddressScope;
  zone: string | null;
}

export interface ConnectionKey {
  pid: number | null;
  process_instance: string | null;
  local: Endpoint;
  remote: Endpoint;
  protocol: NetworkProtocol;
  interface_id: string | null;
  capture_generation: number;
  flow_generation: number;
}

export type EndpointRole =
  | "remote_peer"
  | "local_peer"
  | "listener"
  | "unknown";
export type ObservationSource =
  | "windows_ip_helper"
  | "windows_etw"
  | "windows_helper"
  | "linux_inet_diag"
  | "linux_ebpf"
  | "local_dns"
  | "local_mmdb"
  | "handshake_metadata"
  | "offline_catalog"
  | "user_annotation"
  | "demo"
  | "historical"
  | "unknown";
export type CorrelationQuality =
  | "confirmed"
  | "correlated"
  | "candidate"
  | "snapshot_only"
  | "unknown"
  | "approximate"
  | "unavailable";

export interface ConnectionEvidence {
  endpoint_role: EndpointRole;
  role_reason: string;
  observation_source: ObservationSource;
  correlation_quality: CorrelationQuality;
  observed_at_ms: number;
  sequence: number;
}

export interface ProcessInfo {
  pid: number | null;
  name: string;
  executable: string | null;
  executable_path: string | null;
  started_at_ms: number | null;
  icon_data_url: string | null;
  parent_pid: number | null;
  parent_instance: string | null;
  user_id: string | null;
  session_id: number | null;
  cgroup: string | null;
  network_namespace: string | null;
  executable_revision_id: string | null;
}

export type ExecutableSignatureStatus =
  | "valid"
  | "invalid"
  | "unsigned"
  | "unsupported"
  | "not_checked";

export interface ExecutableIdentity {
  process_instance: string | null;
  path: string;
  revision_id: string;
  size_bytes: number;
  modified_at_ms: number | null;
  sha256: string | null;
  signature_status: ExecutableSignatureStatus;
  package: string | null;
  format: string | null;
  computed_at_ms: number;
  limitation: string | null;
}

export type ExecutableInspectionState =
  | "queued"
  | "running"
  | "completed"
  | "cancelled"
  | "failed";

export interface ExecutableInspectionStatus {
  id: string;
  process_instance: string;
  state: ExecutableInspectionState;
  identity: ExecutableIdentity | null;
  started_at_ms: number;
  finished_at_ms: number | null;
  message: string | null;
}

export interface GeoLocation {
  country_code: string | null;
  country_name: string | null;
  region: string | null;
  city: string | null;
  latitude: number | null;
  longitude: number | null;
  source: string;
  precision: GeoPrecision;
  placement: PlacementKind;
  accuracy_radius_km: number | null;
  dataset_kind: string | null;
  dataset_edition: string | null;
  dataset_build_epoch_ms: number | null;
  dataset_sha256: string | null;
  matched_prefix: number | null;
  city_generation: number;
  asn_generation: number;
}

export type GeoPrecision =
  | "country"
  | "region"
  | "city"
  | "coordinates"
  | "unknown";
export type PlacementKind =
  | "database_coordinate"
  | "country_centroid"
  | "not_plotted";

export interface AsnInfo {
  number: number;
  organization: string | null;
  dataset_edition: string | null;
  dataset_build_epoch_ms: number | null;
  dataset_sha256: string | null;
  matched_prefix: number | null;
  generation: number;
}

export interface TrafficCounters {
  bytes_sent: number;
  bytes_received: number;
  packets_sent: number;
  packets_received: number;
}

export type CounterSource =
  | "unavailable"
  | "interface_aggregate"
  | "etw_flow"
  | "ebpf_flow"
  | "demo";

export interface TransferRates {
  down_bytes_per_second: number;
  up_bytes_per_second: number;
  packets_per_second: number;
}

export interface TrafficTrendPoint {
  timestamp_ms: number;
  down_bytes_per_second: number;
  up_bytes_per_second: number;
}

export type SignalKind =
  | "new_destination"
  | "new_country"
  | "unusual_port"
  | "connection_burst"
  | "high_bandwidth"
  | "persistent_connection"
  | "new_network_process"
  | "destination_diversity";

export interface UnusualSignal {
  kind: SignalKind;
  title: string;
  reason: string;
  typical: string | null;
}

export interface Connection {
  id: string;
  key: ConnectionKey;
  evidence: ConnectionEvidence;
  process: ProcessInfo;
  protocol: NetworkProtocol;
  direction: Direction;
  state: ConnectionState;
  interface: string | null;
  hostname: string | null;
  domain: string | null;
  geo: GeoLocation | null;
  asn: AsnInfo | null;
  traffic: TrafficCounters;
  counter_source: CounterSource;
  rates: TransferRates;
  first_seen_ms: number;
  last_seen_ms: number;
  duration_ms: number;
  active: boolean;
  first_seen_destination: boolean;
  repeat_count: number;
  unusual: UnusualSignal[];
  visual_expires_ms: number | null;
}

export interface ProcessSummary {
  key: string;
  process: ProcessInfo;
  active_connections: number;
  total_connections: number;
  traffic: TrafficCounters;
  countries: number;
}

export interface DestinationSummary {
  key: string;
  ip: string;
  domain: string | null;
  country_code: string | null;
  country_name: string | null;
  asn: AsnInfo | null;
  active_connections: number;
  total_connections: number;
  process_names: string[];
  traffic: TrafficCounters;
  ports: number[];
  first_seen_ms: number;
  last_seen_ms: number;
  recurrence: number;
  geo_precision: GeoPrecision;
  placement: PlacementKind;
}

export interface AsnProfile {
  number: number;
  organization: string | null;
  observed_ips: string[];
  process_names: string[];
  session_ids: string[];
  connection_ids: string[];
  estimated_countries: string[];
  traffic: TrafficCounters;
  first_seen_ms: number | null;
  last_seen_ms: number | null;
}

export interface DashboardStats {
  active_connections: number;
  down_bytes_per_second: number;
  up_bytes_per_second: number;
  packets_per_second: number;
  countries: number;
  processes: number;
  total_connections: number;
}

export interface Diagnostics {
  cpu_percent: number;
  memory_bytes: number;
  render_fps: number;
  capture_events_per_second: number;
  database_size_bytes: number;
  capture_queue_depth: number;
  capture_queue_capacity: number;
  dropped_capture_events: number;
  dropped_ui_events: number;
  queue_saturation: number;
  backend: string;
  capture_event_source: string;
  flow_counters_available: boolean;
  packet_events_observed: number;
  dns_correlations_observed: number;
  correlated_capture_events: number;
  approximate_capture_events: number;
  orphan_capture_events: number;
  capture_generation: number;
  sequence_gaps: number;
  snapshot_resyncs: number;
  dashboard_requests: number;
  dashboard_bytes_serialized: number;
  dashboard_empty_deltas: number;
  dashboard_overlaps: number;
  storage_quick_check_ok: boolean;
  storage_last_check_ms: number | null;
  storage_wal_pages: number;
  last_error: string | null;
  subsystems: SubsystemDiagnostic[];
}

export type SubsystemStatus =
  | "healthy"
  | "degraded"
  | "unavailable"
  | "disabled";

export interface SubsystemDiagnostic {
  subsystem: string;
  status: SubsystemStatus;
  since_ms: number | null;
  message: string;
  action: string | null;
  queue_depth: number;
  queue_capacity: number;
  dropped_events: number;
}

export interface CaptureSummary {
  mode: CaptureMode;
  status: CaptureStatus;
  selected_interface: string | null;
  paused_view_events: number;
  vpn_active: boolean;
}

export interface SessionSummary {
  id: string;
  started_at_ms: number;
  ended_at_ms: number | null;
  connection_count: number;
  process_count: number;
  country_count: number;
  destination_count: number;
  recording_count: number;
  ephemeral: boolean;
  traffic: TrafficCounters;
}

export interface RecordingSummary {
  id: number;
  session_id: string | null;
  path: string;
  started_at_ms: number;
  ended_at_ms: number | null;
  bytes_written: number;
  status: RecordingStatus;
}

export interface DashboardSnapshot {
  generated_at_ms: number;
  revision: number;
  connections: Connection[];
  processes: ProcessSummary[];
  destinations: DestinationSummary[];
  stats: DashboardStats;
  diagnostics: Diagnostics;
  capture: CaptureSummary;
  session: SessionSummary | null;
}

export interface DashboardUpdate {
  sequence: number;
  base_revision: number;
  revision: number;
  generated_at_ms: number;
  resync_required: boolean;
  snapshot: DashboardSnapshot | null;
  upserted_connections: Connection[];
  removed_connection_ids: string[];
  processes?: ProcessSummary[];
  destinations?: DestinationSummary[];
  stats?: DashboardStats;
  diagnostics?: Diagnostics;
  capture?: CaptureSummary;
  session?: SessionSummary | null;
}

export interface NetworkInterface {
  id: string;
  stable_id: string;
  name: string;
  system_name: string;
  description: string;
  luid: number | null;
  network_guid: string | null;
  addresses: string[];
  is_up: boolean;
  is_loopback: boolean;
  is_virtual: boolean;
  is_vpn: boolean;
}

export interface LocalNeighbor {
  address: string;
  mac_address: string | null;
  interface_id: string;
  is_router: boolean;
  state: string;
}

export interface HistoryQuery {
  from_ms: number | null;
  to_ms: number | null;
  session_id: string | null;
  query: string | null;
  limit: number;
  cursor?: HistoryCursor | null;
}

export interface HistoryCursor {
  last_seen_ms: number;
  connection_id: string;
}

export interface HistoryEntry {
  session_id: string;
  connection: Connection;
}

export interface HistoryResponse {
  entries: HistoryEntry[];
  sessions: SessionSummary[];
  recordings: RecordingSummary[];
  next_cursor: HistoryCursor | null;
  has_more: boolean;
}

export type RetentionPolicy =
  | "off"
  | "one_day"
  | "seven_days"
  | "thirty_days"
  | "ninety_days"
  | "unlimited";

export interface AppSettings {
  retention: RetentionPolicy;
  reduce_motion: boolean;
  auto_focus: boolean;
  show_local_traffic: boolean;
  show_listening_sockets: boolean;
  reverse_dns_enabled: boolean;
  offline_enforced: boolean;
  selected_interface: string | null;
  home_latitude: number;
  home_longitude: number;
  ui_language: string;
  demo_seed: number;
  feed_group_window_seconds: number;
  active_workspace: ConnectionWorkspace;
  notification_rules: NotificationRule[];
  notification_quiet_hours_enabled: boolean;
  notification_quiet_start_minute: number;
  notification_quiet_end_minute: number;
  ephemeral_capture: boolean;
  ui_density: UiDensity;
  operation_profile: OperationProfile;
  autostart_enabled: boolean;
  handshake_metadata_enabled: boolean;
  enhanced_contrast: boolean;
  text_scale_percent: number;
}

export type UiDensity = "compact" | "comfortable";
export type OperationProfile =
  | "visible"
  | "background_low_resource"
  | "investigation_high_fidelity"
  | "ephemeral";

export type ConnectionWorkspace = "remote" | "local" | "listeners";
export type NotificationRuleKind =
  | "new_process"
  | "new_country"
  | "asn"
  | "port";

export interface NotificationRule {
  id: string;
  kind: NotificationRuleKind;
  value: string | null;
  enabled: boolean;
  cooldown_seconds: number;
}

export type RuleKind = "process" | "domain" | "ip" | "country" | "localhost";

export interface Bookmark {
  id: number;
  kind: Exclude<RuleKind, "localhost">;
  value: string;
  created_at_ms: number;
}

export interface IgnoreRule {
  id: number;
  kind: RuleKind;
  value: string;
  enabled: boolean;
  created_at_ms: number;
}

export interface FocusRules {
  bookmarks: Bookmark[];
  ignore_rules: IgnoreRule[];
}

export interface ConnectionHistoryStats {
  first_ever_seen_ms: number | null;
  previous_connections: number;
  sessions: number;
}

export interface TimeMachineState {
  at_ms: number;
  connections: Connection[];
  session_id: string | null;
}

export interface GeoDatabaseStatus {
  installed: boolean;
  dataset: string;
  version: string | null;
  last_update_ms: number | null;
  next_recommended_update_ms: number | null;
  path: string | null;
  edition: string | null;
  sha256: string | null;
  attribution: string;
  city: GeoDatasetStatus;
  asn: GeoDatasetStatus;
  last_operation: string | null;
}

export interface GeoDatasetStatus {
  kind: string;
  installed: boolean;
  dataset: string;
  version: string | null;
  build_epoch_ms: number | null;
  last_update_ms: number | null;
  next_recommended_update_ms: number | null;
  path: string | null;
  edition: string | null;
  sha256: string | null;
  generation: number;
  backup_available: boolean;
  attribution: string;
}

export type TimelineResolution = "one_second" | "one_minute" | "one_hour";

export interface TimelineQuery {
  from_ms: number;
  to_ms: number;
  session_id: string | null;
  process_key: string | null;
  resolution: TimelineResolution;
}

export interface TrafficSample {
  timestamp_ms: number;
  session_id: string | null;
  process_key: string | null;
  sample_interval_ms: number;
  counters: TrafficCounters;
  rates: TransferRates;
  source: CounterSource;
}

export interface SelfTestCheck {
  id: string;
  level: "pass" | "warn" | "fail";
  title: string;
  detail: string;
  diagnostic_code: string | null;
  action: string | null;
}

export interface CaptureSelfTest {
  generated_at_ms: number;
  checks: SelfTestCheck[];
  coverage: CaptureCapability[];
  redacted_report: string;
}

export interface CaptureCapability {
  protocol: NetworkProtocol;
  family: AddressFamily;
  endpoint: "pass" | "warn" | "fail";
  process: "pass" | "warn" | "fail";
  interface: "pass" | "warn" | "fail";
  close_event: "pass" | "warn" | "fail";
  limitation: string | null;
}

export type IdentityEventKind =
  | "process_observed"
  | "dns_resolved"
  | "connection_opened"
  | "connection_closed"
  | "geo_enriched";

export interface IdentityTimelineEvent {
  id: string;
  timestamp_ms: number;
  kind: IdentityEventKind;
  title: string;
  detail: string;
  source: ObservationSource;
  confidence: CorrelationQuality;
  connection_id: string;
  session_id: string | null;
}

export interface InvestigationQuery {
  filter: string | null;
  from_ms: number | null;
  to_ms: number | null;
  session_id: string | null;
  pinned_connection_ids: string[];
  pinned_destination_ips: string[];
  pinned_process_keys: string[];
  pinned_asns: number[];
}

export interface SavedInvestigation {
  id: string;
  name: string;
  query: InvestigationQuery;
  notes: string;
  tags: string[];
  selected_connection_id: string | null;
  layout_json: string;
  autosave_revision: number;
  created_at_ms: number;
  updated_at_ms: number;
}

export interface DerivedGeoEnrichment {
  connection_id: string;
  observed_geo: GeoLocation | null;
  observed_asn: AsnInfo | null;
  current_geo: GeoLocation | null;
  current_asn: AsnInfo | null;
}

export type ReenrichmentJobState =
  | "queued"
  | "running"
  | "completed"
  | "cancelled"
  | "failed";

export interface ReenrichmentJobStatus {
  id: string;
  session_id: string;
  state: ReenrichmentJobState;
  processed: number;
  total: number;
  started_at_ms: number;
  finished_at_ms: number | null;
  message: string | null;
}

export interface ComparisonRow {
  key: string;
  left_count: number;
  right_count: number;
  delta: number;
}

export interface SessionComparison {
  left_session_id: string;
  right_session_id: string;
  processes: ComparisonRow[];
  destinations: ComparisonRow[];
  countries: ComparisonRow[];
  ports: ComparisonRow[];
}

export interface ComparisonExportRequest {
  left_session_id: string;
  right_session_id: string;
  format: "csv" | "json";
  path: string;
}

export interface SupportBundleRequest {
  path: string;
  include_settings: boolean;
  include_logs: boolean;
}

export type RecordingStatus =
  | "idle"
  | "starting"
  | "recording"
  | "stopping"
  | "error";

export interface RecordingState {
  status: RecordingStatus;
  output_path: string | null;
  started_at_ms: number | null;
  bytes_written: number;
  max_file_size_mb: number;
  duration_limit_seconds: number | null;
  message: string | null;
}

export interface RecordingRequest {
  output_path: string;
  max_file_size_mb: number;
  rotate: boolean;
  duration_limit_seconds: number | null;
}

export interface ExportRequest {
  format: "csv" | "json";
  path: string;
  session_id: string | null;
  process_key: string | null;
  from_ms: number | null;
  to_ms: number | null;
  query: string | null;
  redaction: ExportRedaction;
}

export interface ExportRedaction {
  local_ip: boolean;
  executable_path: boolean;
  user_names: boolean;
}

export interface ExportResult {
  path: string;
  records: number;
  bytes_written: number;
}

export interface StorageHealth {
  quick_check_ok: boolean;
  detail: string;
  wal_busy_pages: number;
  wal_log_pages: number;
  wal_checkpointed_pages: number;
  database_size_bytes: number;
  checked_at_ms: number;
}

export type LocalDataCategory =
  | "sessions"
  | "geo_ip"
  | "dns_cache"
  | "logs"
  | "all";

export interface LocalDataRemovalResult {
  category: LocalDataCategory;
  removed_items: number;
  requires_capture_restart: boolean;
}

export interface CommandFailure {
  code: string;
  message: string;
  recoverable: boolean;
}

export type NetworkPathKind =
  | "direct"
  | "gateway"
  | "vpn"
  | "system_proxy"
  | "unknown";

export type ListenerScope = "loopback" | "lan" | "vpn" | "wildcard" | "unknown";
export type FirewallEvidence =
  | "allowed"
  | "blocked"
  | "ambiguous"
  | "not_queryable";

export interface NetworkProfile {
  id: string;
  display_name: string | null;
  vpn_observed: boolean;
  created_at_ms: number;
  sensitive_fields_redacted: boolean;
}

export interface NetworkPathEvidence {
  id: string;
  destination_prefix: string;
  prefix_length: number;
  gateway_pseudonym: string | null;
  interface_id: string | null;
  metric: number | null;
  network_generation: number;
  profile_id: string;
  kind: NetworkPathKind;
  observed_at_ms: number;
  source: ObservationSource;
  quality: CorrelationQuality;
  limitation: string | null;
}

export interface DnsCausalCandidate {
  answer_id: string;
  query_name: string;
  quality: CorrelationQuality;
  reason: string;
}

export interface DnsCausalRelation {
  confirmed: DnsCausalCandidate | null;
  candidates: DnsCausalCandidate[];
  limitation: string | null;
}

export interface ListenerExposure {
  endpoint: Endpoint;
  protocol: NetworkProtocol;
  process_instance: string | null;
  scope: ListenerScope;
  firewall: FirewallEvidence;
  first_observed_ms: number;
  last_observed_ms: number;
  limitation: string;
}

export interface OfflineCatalogIdentity {
  name: string;
  snapshot_date: string;
  source: string;
  license: string;
  sha256: string;
}

export interface OfflineServiceHint {
  observed_port: number;
  observed_protocol: NetworkProtocol;
  service_name: string;
  description: string;
  wording: string;
  catalog: OfflineCatalogIdentity;
}

export interface ConnectionContextResponse {
  connectionId: string;
  dns: DnsCausalRelation | null;
  path: NetworkPathEvidence;
  listener: ListenerExposure | null;
  serviceHint: OfflineServiceHint | null;
  registrableDomain: string | null;
}

export interface TodayCardState {
  id: string;
  dismissed: boolean;
  pinned: boolean;
  updated_at_ms: number;
}

export type TodayChangeKind =
  | "process"
  | "application_destination"
  | "domain"
  | "asn"
  | "country"
  | "executable_revision"
  | "network_profile"
  | "listener_added"
  | "listener_removed"
  | "listener_widened"
  | "provider_degradation";

export type TodayEvidenceTarget =
  | "live"
  | "investigate"
  | "exposure"
  | "health"
  | "settings";

export interface TodayChangeExample {
  id: string;
  kind: TodayChangeKind;
  primary: string;
  secondary: string | null;
  connection_id: string | null;
  query: string | null;
  target: TodayEvidenceTarget;
  observed_at_ms: number;
}

export interface TodayChangeGroup {
  kind: TodayChangeKind;
  count: number;
  examples: TodayChangeExample[];
}

export interface TodaySessionComparison {
  previous_session: SessionSummary;
  current_session_id: string;
  comparison: SessionComparison;
  previous_traffic: TrafficCounters;
  current_traffic: TrafficCounters;
  traffic_comparable: boolean;
  traffic_limitation: string | null;
  listener_added: ListenerExposure[];
  listener_removed: ListenerExposure[];
  listener_widened: ListenerExposure[];
}

export interface TodayPayloadLimits {
  examples_per_category: number;
  comparison_rows_per_category: number;
  card_states: number;
  profiles: number;
  rules: number;
  notifications: number;
}

export interface TodayBootstrap {
  generated_at_ms: number;
  history_scope: "all_retained";
  limits: TodayPayloadLimits;
  changes: TodayChangeGroup[];
  card_states: TodayCardState[];
  profiles: NetworkProfile[];
  rules: LocalRule[];
  notifications: LocalNotificationRecord[];
  active_profile_id: string | null;
  previous_session: TodaySessionComparison | null;
}

export interface ListenerSnapshot {
  id: number | null;
  session_id: string | null;
  observed_at_ms: number;
  snapshot_sha256: string;
  listeners: ListenerExposure[];
}

export type LocalRuleCondition =
  | { field: "process"; value: string }
  | { field: "executable_revision"; value: string }
  | { field: "domain"; value: string }
  | { field: "ip"; value: string }
  | { field: "cidr"; value: string }
  | { field: "asn"; value: number }
  | { field: "country"; value: string }
  | { field: "protocol"; value: NetworkProtocol }
  | { field: "port"; value: number }
  | { field: "listener"; value: ListenerScope }
  | { field: "network_profile"; value: string }
  | { field: "path"; value: NetworkPathKind }
  | { field: "first_seen"; value: boolean }
  | { field: "minimum_bytes"; value: number }
  | { field: "minimum_duration_ms"; value: number };

export type LocalRuleAction =
  | { action: "tag"; value: string }
  | { action: "pin" }
  | { action: "add_to_investigation"; value: string }
  | { action: "notify" };

export interface LocalRule {
  id: string;
  version: number;
  name: string;
  enabled: boolean;
  match_all: boolean;
  conditions: LocalRuleCondition[];
  actions: LocalRuleAction[];
  cooldown_seconds: number;
  created_at_ms: number;
  updated_at_ms: number;
}

export interface RuleMatch {
  rule_id: string;
  rule_version: number;
  connection_id: string;
  matched: boolean;
  reasons: string[];
  actions: LocalRuleAction[];
}

export type NotificationState = "unread" | "read" | "pinned" | "resolved";

export interface LocalNotificationRecord {
  id: string;
  rule_id: string;
  rule_version: number;
  evidence_ids: string[];
  network_profile_id: string | null;
  title: string;
  detail: string;
  group_key: string;
  count: number;
  first_at_ms: number;
  last_at_ms: number;
  snoozed_until_ms: number | null;
  state: NotificationState;
}

export interface ConnectionAnnotation {
  connection_id: string;
  tags: string[];
  pinned: boolean;
  investigation_ids: string[];
  updated_at_ms: number;
}

export interface SabiExportRequest {
  path: string;
  investigation_id: string;
  redaction: ExportRedaction;
}

export interface SabiManifest {
  schema_version: number;
  app_version: string;
  created_at_ms: number;
  investigation_id: string;
  query: InvestigationQuery;
  from_ms: number | null;
  to_ms: number | null;
  redaction: ExportRedaction;
  dataset_sha256: string[];
  entry_sha256: Record<string, string>;
}

export interface ImportedSabiBundle {
  source_path: string;
  read_only: true;
  manifest: SabiManifest;
  investigation: SavedInvestigation;
  connections: HistoryEntry[];
  timeline: TrafficSample[];
}
