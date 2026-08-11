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
export type PrimaryView =
  | "globe"
  | "processes"
  | "destinations"
  | "history"
  | "settings";

export type CaptureStatus =
  | { state: "stopped" | "starting" | "capturing" }
  | { state: "error"; message: string };

export interface Endpoint {
  address: string;
  port: number;
}

export interface ConnectionKey {
  pid: number | null;
  local: Endpoint;
  remote: Endpoint;
  protocol: NetworkProtocol;
}

export interface ProcessInfo {
  pid: number | null;
  name: string;
  executable: string | null;
  executable_path: string | null;
  started_at_ms: number | null;
  icon_data_url: string | null;
}

export interface GeoLocation {
  country_code: string;
  country_name: string;
  region: string | null;
  city: string | null;
  latitude: number;
  longitude: number;
  source: string;
}

export interface AsnInfo {
  number: number;
  organization: string;
}

export interface TrafficCounters {
  bytes_sent: number;
  bytes_received: number;
  packets_sent: number;
  packets_received: number;
}

export interface TransferRates {
  down_bytes_per_second: number;
  up_bytes_per_second: number;
  packets_per_second: number;
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
  last_error: string | null;
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

export interface NetworkInterface {
  id: string;
  name: string;
  description: string;
  addresses: string[];
  is_up: boolean;
  is_loopback: boolean;
  is_virtual: boolean;
  is_vpn: boolean;
}

export interface HistoryQuery {
  from_ms: number | null;
  to_ms: number | null;
  session_id: string | null;
  query: string | null;
  limit: number;
}

export interface HistoryEntry {
  session_id: string;
  connection: Connection;
}

export interface HistoryResponse {
  entries: HistoryEntry[];
  sessions: SessionSummary[];
  recordings: RecordingSummary[];
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
  selected_interface: string | null;
  home_latitude: number;
  home_longitude: number;
  ui_language: string;
  demo_seed: number;
  feed_group_window_seconds: number;
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
  attribution: string;
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
}

export interface ExportResult {
  path: string;
  records: number;
  bytes_written: number;
}

export interface CommandFailure {
  code: string;
  message: string;
  recoverable: boolean;
}
