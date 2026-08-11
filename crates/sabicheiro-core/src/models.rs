use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::IpAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProtocol {
    Tcp,
    Udp,
}

impl NetworkProtocol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Inbound,
    Outbound,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Listening,
    Connecting,
    Established,
    Closing,
    Closed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    Off,
    Demo,
    Live,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "message")]
pub enum CaptureStatus {
    Stopped,
    Starting,
    Capturing,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Globe,
    Map,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Endpoint {
    pub address: IpAddr,
    pub port: u16,
}

impl Endpoint {
    pub const fn new(address: IpAddr, port: u16) -> Self {
        Self { address, port }
    }

    pub fn label(&self) -> String {
        match self.address {
            IpAddr::V4(_) => format!("{}:{}", self.address, self.port),
            IpAddr::V6(_) => format!("[{}]:{}", self.address, self.port),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionKey {
    pub pid: Option<u32>,
    pub local: Endpoint,
    pub remote: Endpoint,
    pub protocol: NetworkProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: Option<u32>,
    pub name: String,
    pub executable: Option<String>,
    pub executable_path: Option<String>,
    pub started_at_ms: Option<i64>,
    pub icon_data_url: Option<String>,
}

impl Default for ProcessInfo {
    fn default() -> Self {
        Self {
            pid: None,
            name: "Unknown process".into(),
            executable: None,
            executable_path: None,
            started_at_ms: None,
            icon_data_url: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoLocation {
    pub country_code: String,
    pub country_name: String,
    pub region: Option<String>,
    pub city: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsnInfo {
    pub number: u32,
    pub organization: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrafficCounters {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
}

impl TrafficCounters {
    pub fn add_assign(&mut self, delta: Self) {
        self.bytes_sent = self.bytes_sent.saturating_add(delta.bytes_sent);
        self.bytes_received = self.bytes_received.saturating_add(delta.bytes_received);
        self.packets_sent = self.packets_sent.saturating_add(delta.packets_sent);
        self.packets_received = self.packets_received.saturating_add(delta.packets_received);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TransferRates {
    pub down_bytes_per_second: f64,
    pub up_bytes_per_second: f64,
    pub packets_per_second: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    NewDestination,
    NewCountry,
    UnusualPort,
    ConnectionBurst,
    HighBandwidth,
    PersistentConnection,
    NewNetworkProcess,
    DestinationDiversity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnusualSignal {
    pub kind: SignalKind,
    pub title: String,
    pub reason: String,
    pub typical: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub key: ConnectionKey,
    pub process: ProcessInfo,
    pub protocol: NetworkProtocol,
    pub direction: Direction,
    pub state: ConnectionState,
    pub interface: Option<String>,
    pub hostname: Option<String>,
    pub domain: Option<String>,
    pub geo: Option<GeoLocation>,
    pub asn: Option<AsnInfo>,
    pub traffic: TrafficCounters,
    pub rates: TransferRates,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
    pub duration_ms: u64,
    pub active: bool,
    pub first_seen_destination: bool,
    pub repeat_count: u32,
    pub unusual: Vec<UnusualSignal>,
    pub visual_expires_ms: Option<i64>,
}

impl Connection {
    pub fn placeholder(timestamp_ms: i64) -> Self {
        let unspecified = "0.0.0.0".parse().expect("valid constant IP");
        let key = ConnectionKey {
            pid: None,
            local: Endpoint::new(unspecified, 0),
            remote: Endpoint::new(unspecified, 0),
            protocol: NetworkProtocol::Tcp,
        };
        Self {
            id: "placeholder".into(),
            key,
            process: ProcessInfo::default(),
            protocol: NetworkProtocol::Tcp,
            direction: Direction::Unknown,
            state: ConnectionState::Unknown,
            interface: None,
            hostname: None,
            domain: None,
            geo: None,
            asn: None,
            traffic: TrafficCounters::default(),
            rates: TransferRates::default(),
            first_seen_ms: timestamp_ms,
            last_seen_ms: timestamp_ms,
            duration_ms: 0,
            active: false,
            first_seen_destination: false,
            repeat_count: 1,
            unusual: Vec::new(),
            visual_expires_ms: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkEventKind {
    Opened,
    Updated,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedNetworkEvent {
    pub kind: NetworkEventKind,
    pub key: ConnectionKey,
    pub timestamp_ms: i64,
    pub process: ProcessInfo,
    pub direction: Direction,
    pub state: ConnectionState,
    pub interface: Option<String>,
    pub hostname: Option<String>,
    pub domain: Option<String>,
    pub geo: Option<GeoLocation>,
    pub asn: Option<AsnInfo>,
    pub traffic_delta: TrafficCounters,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessSummary {
    pub key: String,
    pub process: ProcessInfo,
    pub active_connections: usize,
    pub total_connections: usize,
    pub traffic: TrafficCounters,
    pub countries: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DestinationSummary {
    pub key: String,
    pub ip: IpAddr,
    pub domain: Option<String>,
    pub country_code: Option<String>,
    pub country_name: Option<String>,
    pub asn: Option<AsnInfo>,
    pub active_connections: usize,
    pub total_connections: usize,
    pub process_names: Vec<String>,
    pub traffic: TrafficCounters,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DashboardStats {
    pub active_connections: usize,
    pub down_bytes_per_second: f64,
    pub up_bytes_per_second: f64,
    pub packets_per_second: f64,
    pub countries: usize,
    pub processes: usize,
    pub total_connections: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Diagnostics {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub render_fps: f32,
    pub capture_events_per_second: f64,
    pub database_size_bytes: u64,
    pub capture_queue_depth: usize,
    pub capture_queue_capacity: usize,
    pub dropped_capture_events: u64,
    pub dropped_ui_events: u64,
    pub queue_saturation: f32,
    pub backend: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSummary {
    pub mode: CaptureMode,
    pub status: CaptureStatus,
    pub selected_interface: Option<String>,
    pub paused_view_events: u64,
    pub vpn_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub connection_count: usize,
    pub process_count: usize,
    pub country_count: usize,
    #[serde(default)]
    pub destination_count: usize,
    #[serde(default)]
    pub recording_count: usize,
    pub traffic: TrafficCounters,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingSummary {
    pub id: i64,
    pub session_id: Option<String>,
    pub path: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub bytes_written: u64,
    pub status: RecordingStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub generated_at_ms: i64,
    pub revision: u64,
    pub connections: Vec<Connection>,
    pub processes: Vec<ProcessSummary>,
    pub destinations: Vec<DestinationSummary>,
    pub stats: DashboardStats,
    pub diagnostics: Diagnostics,
    pub capture: CaptureSummary,
    pub session: Option<SessionSummary>,
}

impl DashboardSnapshot {
    pub fn empty(generated_at_ms: i64) -> Self {
        Self {
            generated_at_ms,
            revision: 0,
            connections: Vec::new(),
            processes: Vec::new(),
            destinations: Vec::new(),
            stats: DashboardStats::default(),
            diagnostics: Diagnostics::default(),
            capture: CaptureSummary {
                mode: CaptureMode::Off,
                status: CaptureStatus::Stopped,
                selected_interface: None,
                paused_view_events: 0,
                vpn_active: false,
            },
            session: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub id: String,
    pub name: String,
    pub description: String,
    pub addresses: Vec<IpAddr>,
    pub is_up: bool,
    pub is_loopback: bool,
    pub is_virtual: bool,
    pub is_vpn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryQuery {
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub session_id: Option<String>,
    pub query: Option<String>,
    pub limit: usize,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self {
            from_ms: None,
            to_ms: None,
            session_id: None,
            query: None,
            limit: 1_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub session_id: String,
    pub connection: Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    Off,
    OneDay,
    SevenDays,
    ThirtyDays,
    NinetyDays,
    Unlimited,
}

impl RetentionPolicy {
    pub const fn days(self) -> Option<i64> {
        match self {
            Self::Off => Some(0),
            Self::OneDay => Some(1),
            Self::SevenDays => Some(7),
            Self::ThirtyDays => Some(30),
            Self::NinetyDays => Some(90),
            Self::Unlimited => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub retention: RetentionPolicy,
    pub reduce_motion: bool,
    pub auto_focus: bool,
    pub show_local_traffic: bool,
    pub show_listening_sockets: bool,
    pub reverse_dns_enabled: bool,
    pub selected_interface: Option<String>,
    pub home_latitude: f64,
    pub home_longitude: f64,
    pub ui_language: String,
    pub demo_seed: u64,
    pub feed_group_window_seconds: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            retention: RetentionPolicy::SevenDays,
            reduce_motion: false,
            auto_focus: true,
            show_local_traffic: false,
            show_listening_sockets: false,
            reverse_dns_enabled: false,
            selected_interface: None,
            home_latitude: 40.4168,
            home_longitude: -3.7038,
            ui_language: "en".into(),
            demo_seed: 0x5A81_C4E1,
            feed_group_window_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    Process,
    Domain,
    Ip,
    Country,
    Localhost,
}

impl RuleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::Domain => "domain",
            Self::Ip => "ip",
            Self::Country => "country",
            Self::Localhost => "localhost",
        }
    }
}

impl std::str::FromStr for RuleKind {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "process" => Ok(Self::Process),
            "domain" => Ok(Self::Domain),
            "ip" => Ok(Self::Ip),
            "country" => Ok(Self::Country),
            "localhost" => Ok(Self::Localhost),
            _ => Err("unknown rule kind"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub kind: RuleKind,
    pub value: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoreRule {
    pub id: i64,
    pub kind: RuleKind,
    pub value: String,
    pub enabled: bool,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusRules {
    pub bookmarks: Vec<Bookmark>,
    pub ignore_rules: Vec<IgnoreRule>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionHistoryStats {
    pub first_ever_seen_ms: Option<i64>,
    pub previous_connections: usize,
    pub sessions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoDatabaseStatus {
    pub installed: bool,
    pub dataset: String,
    pub version: Option<String>,
    pub last_update_ms: Option<i64>,
    pub next_recommended_update_ms: Option<i64>,
    pub path: Option<String>,
    pub attribution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingStatus {
    Idle,
    Starting,
    Recording,
    Stopping,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingState {
    pub status: RecordingStatus,
    pub output_path: Option<String>,
    pub started_at_ms: Option<i64>,
    pub bytes_written: u64,
    pub max_file_size_mb: u32,
    pub duration_limit_seconds: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingRequest {
    pub output_path: String,
    pub max_file_size_mb: u32,
    pub rotate: bool,
    pub duration_limit_seconds: Option<u64>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            status: RecordingStatus::Idle,
            output_path: None,
            started_at_ms: None,
            bytes_written: 0,
            max_file_size_mb: 512,
            duration_limit_seconds: None,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub path: String,
    pub session_id: Option<String>,
    pub process_key: Option<String>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub query: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub records: usize,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CountrySummary {
    pub country_code: String,
    pub country_name: String,
    pub active_connections: usize,
    pub process_names: Vec<String>,
    pub destination_count: usize,
    pub asn_count: usize,
    pub traffic: TrafficCounters,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TopSummary {
    pub processes: Vec<ProcessSummary>,
    pub destinations: Vec<DestinationSummary>,
    pub countries: Vec<CountrySummary>,
    pub ports: BTreeMap<u16, usize>,
    pub asns: BTreeMap<u32, usize>,
}
