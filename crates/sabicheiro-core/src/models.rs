use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::BTreeMap,
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr},
};

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressFamily {
    #[default]
    Ipv4,
    Ipv6,
}

impl AddressFamily {
    pub const fn of(address: IpAddr) -> Self {
        match address {
            IpAddr::V4(_) => Self::Ipv4,
            IpAddr::V6(_) => Self::Ipv6,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressScope {
    Global,
    Private,
    Loopback,
    LinkLocal,
    Multicast,
    Cgnat,
    Documentation,
    Benchmark,
    Reserved,
    Broadcast,
    #[default]
    Unspecified,
}

impl AddressScope {
    pub const fn is_global(self) -> bool {
        matches!(self, Self::Global)
    }
}

fn ipv4_in_prefix(address: Ipv4Addr, network: Ipv4Addr, prefix: u32) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    u32::from(address) & mask == u32::from(network) & mask
}

fn ipv6_in_prefix(address: Ipv6Addr, network: Ipv6Addr, prefix: u32) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    u128::from(address) & mask == u128::from(network) & mask
}

/// Converts equivalent textual/network forms to a single address while preserving
/// the family exposed by the operating system separately on [`Endpoint`].
pub fn canonicalize_address(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(address) => address
            .to_ipv4_mapped()
            .map_or(IpAddr::V6(address), IpAddr::V4),
        address => address,
    }
}

/// Classifies every address locally. Only `Global` is eligible for GeoIP.
pub fn classify_address(address: IpAddr) -> AddressScope {
    match canonicalize_address(address) {
        IpAddr::V4(address) => {
            if address.is_unspecified() {
                AddressScope::Unspecified
            } else if address == Ipv4Addr::BROADCAST {
                AddressScope::Broadcast
            } else if ipv4_in_prefix(address, Ipv4Addr::new(127, 0, 0, 0), 8) {
                AddressScope::Loopback
            } else if ipv4_in_prefix(address, Ipv4Addr::new(10, 0, 0, 0), 8)
                || ipv4_in_prefix(address, Ipv4Addr::new(172, 16, 0, 0), 12)
                || ipv4_in_prefix(address, Ipv4Addr::new(192, 168, 0, 0), 16)
            {
                AddressScope::Private
            } else if ipv4_in_prefix(address, Ipv4Addr::new(169, 254, 0, 0), 16) {
                AddressScope::LinkLocal
            } else if ipv4_in_prefix(address, Ipv4Addr::new(100, 64, 0, 0), 10) {
                AddressScope::Cgnat
            } else if ipv4_in_prefix(address, Ipv4Addr::new(192, 0, 2, 0), 24)
                || ipv4_in_prefix(address, Ipv4Addr::new(198, 51, 100, 0), 24)
                || ipv4_in_prefix(address, Ipv4Addr::new(203, 0, 113, 0), 24)
            {
                AddressScope::Documentation
            } else if ipv4_in_prefix(address, Ipv4Addr::new(198, 18, 0, 0), 15) {
                AddressScope::Benchmark
            } else if ipv4_in_prefix(address, Ipv4Addr::new(224, 0, 0, 0), 4) {
                AddressScope::Multicast
            } else if ipv4_in_prefix(address, Ipv4Addr::UNSPECIFIED, 8)
                || (ipv4_in_prefix(address, Ipv4Addr::new(192, 0, 0, 0), 24)
                    && address != Ipv4Addr::new(192, 0, 0, 9)
                    && address != Ipv4Addr::new(192, 0, 0, 10))
                || ipv4_in_prefix(address, Ipv4Addr::new(192, 88, 99, 0), 24)
                || ipv4_in_prefix(address, Ipv4Addr::new(240, 0, 0, 0), 4)
            {
                AddressScope::Reserved
            } else {
                AddressScope::Global
            }
        }
        IpAddr::V6(address) => {
            if address.is_unspecified() {
                AddressScope::Unspecified
            } else if address.is_loopback() {
                AddressScope::Loopback
            } else if ipv6_in_prefix(address, Ipv6Addr::from(0xff00_u128 << 112), 8) {
                AddressScope::Multicast
            } else if ipv6_in_prefix(address, Ipv6Addr::from(0xfe80_u128 << 112), 10) {
                AddressScope::LinkLocal
            } else if ipv6_in_prefix(address, Ipv6Addr::from(0xfc00_u128 << 112), 7) {
                AddressScope::Private
            } else if ipv6_in_prefix(
                address,
                "2001:db8::".parse().expect("constant IPv6 prefix"),
                32,
            ) || ipv6_in_prefix(
                address,
                "3fff::".parse().expect("constant IPv6 prefix"),
                20,
            ) {
                AddressScope::Documentation
            } else if ipv6_in_prefix(
                address,
                "2001:2::".parse().expect("constant IPv6 prefix"),
                48,
            ) {
                AddressScope::Benchmark
            } else if ipv6_in_prefix(address, "2000::".parse().expect("constant IPv6 prefix"), 3) {
                AddressScope::Global
            } else {
                AddressScope::Reserved
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Endpoint {
    pub address: IpAddr,
    pub port: u16,
    pub observed_family: AddressFamily,
    pub scope: AddressScope,
    pub zone: Option<String>,
}

impl Endpoint {
    pub fn new(address: IpAddr, port: u16) -> Self {
        let observed_family = AddressFamily::of(address);
        let address = canonicalize_address(address);
        Self {
            address,
            port,
            observed_family,
            scope: classify_address(address),
            zone: None,
        }
    }

    pub fn with_zone(mut self, zone: impl Into<String>) -> Self {
        let zone = zone.into();
        self.zone = (!zone.trim().is_empty()).then_some(zone);
        self
    }

    pub fn parse(address: &str, port: u16) -> Result<Self, AddrParseError> {
        let (address, zone) = address
            .rsplit_once('%')
            .filter(|(address, zone)| address.contains(':') && !zone.trim().is_empty())
            .map_or((address, None), |(address, zone)| (address, Some(zone)));
        let endpoint = Self::new(address.parse()?, port);
        Ok(match zone {
            Some(zone) => endpoint.with_zone(zone),
            None => endpoint,
        })
    }

    pub fn label(&self) -> String {
        let address = self.zone.as_deref().map_or_else(
            || self.address.to_string(),
            |zone| format!("{}%{zone}", self.address),
        );
        match self.address {
            IpAddr::V4(_) => format!("{address}:{}", self.port),
            IpAddr::V6(_) => format!("[{address}]:{}", self.port),
        }
    }
}

#[derive(Deserialize)]
struct EndpointWire {
    address: IpAddr,
    port: u16,
    #[serde(default)]
    observed_family: Option<AddressFamily>,
    #[serde(default)]
    scope: Option<AddressScope>,
    #[serde(default)]
    zone: Option<String>,
}

impl<'de> Deserialize<'de> for Endpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EndpointWire::deserialize(deserializer)?;
        let observed_family = wire
            .observed_family
            .unwrap_or_else(|| AddressFamily::of(wire.address));
        let address = canonicalize_address(wire.address);
        Ok(Self {
            address,
            port: wire.port,
            observed_family,
            scope: wire.scope.unwrap_or_else(|| classify_address(address)),
            zone: wire.zone.filter(|zone| !zone.trim().is_empty()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionKey {
    pub pid: Option<u32>,
    #[serde(default)]
    pub process_instance: Option<String>,
    pub local: Endpoint,
    pub remote: Endpoint,
    pub protocol: NetworkProtocol,
    #[serde(default)]
    pub interface_id: Option<String>,
    #[serde(default)]
    pub capture_generation: u64,
    #[serde(default)]
    pub flow_generation: u64,
}

impl ConnectionKey {
    pub fn new(
        pid: Option<u32>,
        local: Endpoint,
        remote: Endpoint,
        protocol: NetworkProtocol,
    ) -> Self {
        Self {
            pid,
            process_instance: None,
            local,
            remote,
            protocol,
            interface_id: None,
            capture_generation: 0,
            flow_generation: 0,
        }
    }
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

impl ProcessInfo {
    pub fn instance_id(&self) -> Option<String> {
        self.pid.map(|pid| {
            self.started_at_ms.map_or_else(
                || format!("pid:{pid}:start:unknown"),
                |started| format!("pid:{pid}:start:{started}"),
            )
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointRole {
    RemotePeer,
    LocalPeer,
    Listener,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    WindowsIpHelper,
    WindowsEtw,
    LinuxInetDiag,
    LocalDns,
    LocalMmdb,
    Demo,
    Historical,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrelationQuality {
    Confirmed,
    Correlated,
    Approximate,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionEvidence {
    pub endpoint_role: EndpointRole,
    pub role_reason: String,
    pub observation_source: ObservationSource,
    pub correlation_quality: CorrelationQuality,
    pub observed_at_ms: i64,
    pub sequence: u64,
}

impl Default for ConnectionEvidence {
    fn default() -> Self {
        Self {
            endpoint_role: EndpointRole::Unknown,
            role_reason: "incomplete_record".into(),
            observation_source: ObservationSource::Unknown,
            correlation_quality: CorrelationQuality::Unavailable,
            observed_at_ms: 0,
            sequence: 0,
        }
    }
}

impl ConnectionEvidence {
    pub fn for_peer(
        remote: &Endpoint,
        state: ConnectionState,
        source: ObservationSource,
        observed_at_ms: i64,
    ) -> Self {
        let (endpoint_role, role_reason) = if state == ConnectionState::Listening {
            (EndpointRole::Listener, "listener_state")
        } else if remote.address.is_unspecified() || remote.port == 0 {
            (EndpointRole::Unknown, "missing_remote_peer")
        } else if remote.scope.is_global() {
            (EndpointRole::RemotePeer, "connected_global_peer")
        } else {
            (EndpointRole::LocalPeer, "connected_non_global_peer")
        };
        Self {
            endpoint_role,
            role_reason: role_reason.into(),
            observation_source: source,
            correlation_quality: CorrelationQuality::Confirmed,
            observed_at_ms,
            sequence: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeoPrecision {
    Country,
    Region,
    City,
    Coordinates,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementKind {
    DatabaseCoordinate,
    CountryCentroid,
    #[default]
    NotPlotted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeoLocation {
    pub country_code: Option<String>,
    pub country_name: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub source: String,
    pub precision: GeoPrecision,
    pub placement: PlacementKind,
    pub accuracy_radius_km: Option<u16>,
    pub dataset_kind: Option<String>,
    pub dataset_edition: Option<String>,
    pub dataset_build_epoch_ms: Option<i64>,
    pub dataset_sha256: Option<String>,
    pub matched_prefix: Option<u8>,
    pub city_generation: u64,
    pub asn_generation: u64,
}

impl Default for GeoLocation {
    fn default() -> Self {
        Self {
            country_code: None,
            country_name: None,
            region: None,
            city: None,
            latitude: None,
            longitude: None,
            source: "Local MMDB".into(),
            precision: GeoPrecision::Unknown,
            placement: PlacementKind::NotPlotted,
            accuracy_radius_km: None,
            dataset_kind: None,
            dataset_edition: None,
            dataset_build_epoch_ms: None,
            dataset_sha256: None,
            matched_prefix: None,
            city_generation: 0,
            asn_generation: 0,
        }
    }
}

impl GeoLocation {
    pub fn coordinates(&self) -> Option<(f64, f64)> {
        self.latitude
            .zip(self.longitude)
            .filter(|(latitude, longitude)| {
                latitude.is_finite()
                    && longitude.is_finite()
                    && (-90.0..=90.0).contains(latitude)
                    && (-180.0..=180.0).contains(longitude)
                    && self.placement != PlacementKind::NotPlotted
            })
    }

    pub fn country_label(&self) -> Option<String> {
        self.country_name
            .clone()
            .or_else(|| self.country_code.clone())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AsnInfo {
    pub number: u32,
    pub organization: Option<String>,
    pub dataset_edition: Option<String>,
    pub dataset_build_epoch_ms: Option<i64>,
    pub dataset_sha256: Option<String>,
    pub matched_prefix: Option<u8>,
    pub generation: u64,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterSource {
    #[default]
    Unavailable,
    InterfaceAggregate,
    EtwFlow,
    Demo,
}

impl CounterSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::InterfaceAggregate => "interface_aggregate",
            Self::EtwFlow => "etw_flow",
            Self::Demo => "demo",
        }
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
    #[serde(default)]
    pub evidence: ConnectionEvidence,
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
    #[serde(default)]
    pub counter_source: CounterSource,
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
            process_instance: None,
            local: Endpoint::new(unspecified, 0),
            remote: Endpoint::new(unspecified, 0),
            protocol: NetworkProtocol::Tcp,
            interface_id: None,
            capture_generation: 0,
            flow_generation: 0,
        };
        Self {
            id: "placeholder".into(),
            key,
            evidence: ConnectionEvidence::default(),
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
            counter_source: CounterSource::Unavailable,
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
    #[serde(default)]
    pub evidence: ConnectionEvidence,
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
    #[serde(default)]
    pub counter_source: CounterSource,
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
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub first_seen_ms: i64,
    #[serde(default)]
    pub last_seen_ms: i64,
    #[serde(default)]
    pub recurrence: u32,
    #[serde(default)]
    pub geo_precision: GeoPrecision,
    #[serde(default)]
    pub placement: PlacementKind,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AsnProfile {
    pub number: u32,
    pub organization: Option<String>,
    pub observed_ips: Vec<IpAddr>,
    pub process_names: Vec<String>,
    pub session_ids: Vec<String>,
    pub connection_ids: Vec<String>,
    pub estimated_countries: Vec<String>,
    pub traffic: TrafficCounters,
    pub first_seen_ms: Option<i64>,
    pub last_seen_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityEventKind {
    ProcessObserved,
    DnsResolved,
    ConnectionOpened,
    ConnectionClosed,
    GeoEnriched,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityTimelineEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub kind: IdentityEventKind,
    pub title: String,
    pub detail: String,
    pub source: ObservationSource,
    pub confidence: CorrelationQuality,
    pub connection_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InvestigationQuery {
    pub filter: Option<String>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub session_id: Option<String>,
    pub pinned_connection_ids: Vec<String>,
    pub pinned_destination_ips: Vec<IpAddr>,
    pub pinned_process_keys: Vec<String>,
    pub pinned_asns: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedInvestigation {
    pub id: String,
    pub name: String,
    pub query: InvestigationQuery,
    pub notes: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
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
    pub capture_event_source: String,
    pub flow_counters_available: bool,
    pub packet_events_observed: u64,
    pub dns_correlations_observed: u64,
    pub correlated_capture_events: u64,
    pub approximate_capture_events: u64,
    pub orphan_capture_events: u64,
    pub capture_generation: u64,
    pub sequence_gaps: u64,
    pub snapshot_resyncs: u64,
    pub storage_quick_check_ok: bool,
    pub storage_last_check_ms: Option<i64>,
    pub storage_wal_pages: i64,
    pub last_error: Option<String>,
    #[serde(default)]
    pub subsystems: Vec<SubsystemDiagnostic>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubsystemStatus {
    #[default]
    Healthy,
    Degraded,
    Unavailable,
    Disabled,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubsystemDiagnostic {
    pub subsystem: String,
    pub status: SubsystemStatus,
    pub since_ms: Option<i64>,
    pub message: String,
    pub action: Option<String>,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub dropped_events: u64,
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
    #[serde(default)]
    pub ephemeral: bool,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardUpdate {
    pub sequence: u64,
    pub base_revision: u64,
    pub revision: u64,
    pub generated_at_ms: i64,
    pub resync_required: bool,
    pub snapshot: Option<DashboardSnapshot>,
    pub upserted_connections: Vec<Connection>,
    pub removed_connection_ids: Vec<String>,
    pub processes: Vec<ProcessSummary>,
    pub destinations: Vec<DestinationSummary>,
    pub stats: DashboardStats,
    pub diagnostics: Diagnostics,
    pub capture: CaptureSummary,
    pub session: Option<SessionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub id: String,
    #[serde(default)]
    pub stable_id: String,
    pub name: String,
    #[serde(default)]
    pub system_name: String,
    pub description: String,
    #[serde(default)]
    pub luid: Option<u64>,
    #[serde(default)]
    pub network_guid: Option<String>,
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
    pub active_workspace: ConnectionWorkspace,
    pub notification_rules: Vec<NotificationRule>,
    pub ephemeral_capture: bool,
    pub ui_density: UiDensity,
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
            active_workspace: ConnectionWorkspace::Remote,
            notification_rules: Vec::new(),
            ephemeral_capture: false,
            ui_density: UiDensity::Comfortable,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiDensity {
    Compact,
    #[default]
    Comfortable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionWorkspace {
    #[default]
    Remote,
    Local,
    Listeners,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationRuleKind {
    NewProcess,
    NewCountry,
    Asn,
    Port,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRule {
    pub id: String,
    pub kind: NotificationRuleKind,
    pub value: Option<String>,
    pub enabled: bool,
    pub cooldown_seconds: u32,
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
#[serde(default)]
pub struct GeoDatabaseStatus {
    pub installed: bool,
    pub dataset: String,
    pub version: Option<String>,
    pub last_update_ms: Option<i64>,
    pub next_recommended_update_ms: Option<i64>,
    pub path: Option<String>,
    #[serde(default)]
    pub edition: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    pub attribution: String,
    pub city: GeoDatasetStatus,
    pub asn: GeoDatasetStatus,
    pub last_operation: Option<String>,
}

impl Default for GeoDatabaseStatus {
    fn default() -> Self {
        Self {
            installed: false,
            dataset: "City MMDB (not installed)".into(),
            version: None,
            last_update_ms: None,
            next_recommended_update_ms: None,
            path: None,
            edition: None,
            sha256: None,
            attribution: "Local dataset; no observed IP is uploaded".into(),
            city: GeoDatasetStatus::default(),
            asn: GeoDatasetStatus {
                kind: "asn".into(),
                ..GeoDatasetStatus::default()
            },
            last_operation: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoDatasetStatus {
    pub kind: String,
    pub installed: bool,
    pub dataset: String,
    pub version: Option<String>,
    pub build_epoch_ms: Option<i64>,
    pub last_update_ms: Option<i64>,
    pub next_recommended_update_ms: Option<i64>,
    pub path: Option<String>,
    pub edition: Option<String>,
    pub sha256: Option<String>,
    pub generation: u64,
    pub backup_available: bool,
    pub attribution: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageHealth {
    pub quick_check_ok: bool,
    pub detail: String,
    pub wal_busy_pages: i64,
    pub wal_log_pages: i64,
    pub wal_checkpointed_pages: i64,
    pub database_size_bytes: u64,
    pub checked_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineResolution {
    OneSecond,
    OneMinute,
    OneHour,
}

impl TimelineResolution {
    pub const fn bucket_ms(self) -> i64 {
        match self {
            Self::OneSecond => 1_000,
            Self::OneMinute => 60_000,
            Self::OneHour => 3_600_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineQuery {
    pub from_ms: i64,
    pub to_ms: i64,
    pub session_id: Option<String>,
    pub process_key: Option<String>,
    pub resolution: TimelineResolution,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrafficSample {
    pub timestamp_ms: i64,
    pub session_id: Option<String>,
    pub process_key: Option<String>,
    pub counters: TrafficCounters,
    pub rates: TransferRates,
    pub source: CounterSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelfTestLevel {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfTestCheck {
    pub id: String,
    pub level: SelfTestLevel,
    pub title: String,
    pub detail: String,
    pub diagnostic_code: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSelfTest {
    pub generated_at_ms: i64,
    pub checks: Vec<SelfTestCheck>,
    #[serde(default)]
    pub coverage: Vec<CaptureCapability>,
    #[serde(default)]
    pub redacted_report: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureCapability {
    pub protocol: NetworkProtocol,
    pub family: AddressFamily,
    pub endpoint: SelfTestLevel,
    pub process: SelfTestLevel,
    pub interface: SelfTestLevel,
    pub close_event: SelfTestLevel,
    pub limitation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedGeoEnrichment {
    pub connection_id: String,
    pub observed_geo: Option<GeoLocation>,
    pub observed_asn: Option<AsnInfo>,
    pub current_geo: Option<GeoLocation>,
    pub current_asn: Option<AsnInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReenrichmentJobState {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReenrichmentJobStatus {
    pub id: String,
    pub session_id: String,
    pub state: ReenrichmentJobState,
    pub processed: usize,
    pub total: usize,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonRow {
    pub key: String,
    pub left_count: usize,
    pub right_count: usize,
    pub delta: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionComparison {
    pub left_session_id: String,
    pub right_session_id: String,
    pub processes: Vec<ComparisonRow>,
    pub destinations: Vec<ComparisonRow>,
    pub countries: Vec<ComparisonRow>,
    pub ports: Vec<ComparisonRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportBundleRequest {
    pub path: String,
    pub include_settings: bool,
    pub include_logs: bool,
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
    #[serde(default)]
    pub redaction: ExportRedaction,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportRedaction {
    pub local_ip: bool,
    pub executable_path: bool,
    pub user_names: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalDataCategory {
    Sessions,
    GeoIp,
    DnsCache,
    Logs,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalDataRemovalResult {
    pub category: LocalDataCategory,
    pub removed_items: usize,
    pub requires_capture_restart: bool,
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

#[cfg(test)]
mod address_tests {
    use super::*;

    #[test]
    fn special_address_table_covers_ipv4_and_ipv6_boundaries() {
        let cases = [
            ("0.0.0.0", AddressScope::Unspecified),
            ("0.0.0.1", AddressScope::Reserved),
            ("9.255.255.255", AddressScope::Global),
            ("10.0.0.0", AddressScope::Private),
            ("10.255.255.255", AddressScope::Private),
            ("11.0.0.0", AddressScope::Global),
            ("100.63.255.255", AddressScope::Global),
            ("100.64.0.0", AddressScope::Cgnat),
            ("100.127.255.255", AddressScope::Cgnat),
            ("100.128.0.0", AddressScope::Global),
            ("127.255.255.255", AddressScope::Loopback),
            ("169.254.0.0", AddressScope::LinkLocal),
            ("172.15.255.255", AddressScope::Global),
            ("172.16.0.0", AddressScope::Private),
            ("172.31.255.255", AddressScope::Private),
            ("172.32.0.0", AddressScope::Global),
            ("192.0.0.9", AddressScope::Global),
            ("192.0.0.8", AddressScope::Reserved),
            ("192.0.2.0", AddressScope::Documentation),
            ("198.18.0.0", AddressScope::Benchmark),
            ("198.19.255.255", AddressScope::Benchmark),
            ("198.20.0.0", AddressScope::Global),
            ("198.51.100.255", AddressScope::Documentation),
            ("203.0.113.1", AddressScope::Documentation),
            ("223.255.255.255", AddressScope::Global),
            ("224.0.0.0", AddressScope::Multicast),
            ("239.255.255.255", AddressScope::Multicast),
            ("240.0.0.0", AddressScope::Reserved),
            ("255.255.255.255", AddressScope::Broadcast),
            ("::", AddressScope::Unspecified),
            ("::1", AddressScope::Loopback),
            ("fc00::", AddressScope::Private),
            (
                "fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
                AddressScope::Private,
            ),
            ("fe80::1", AddressScope::LinkLocal),
            (
                "febf:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
                AddressScope::LinkLocal,
            ),
            ("fec0::1", AddressScope::Reserved),
            ("ff02::1", AddressScope::Multicast),
            ("2001:2::1", AddressScope::Benchmark),
            ("2001:db8::1", AddressScope::Documentation),
            ("2001:4860:4860::8888", AddressScope::Global),
            ("3fff::1", AddressScope::Documentation),
        ];
        for (address, expected) in cases {
            let address: IpAddr = address.parse().expect("versioned address table");
            assert_eq!(classify_address(address), expected, "{address}");
        }
    }

    #[test]
    fn mapped_ipv4_is_canonical_but_keeps_observed_family() {
        let endpoint = Endpoint::new("::ffff:192.0.2.1".parse().expect("mapped IPv4"), 443);
        assert_eq!(
            endpoint.address,
            "192.0.2.1".parse::<IpAddr>().expect("IPv4")
        );
        assert_eq!(endpoint.observed_family, AddressFamily::Ipv6);
        assert_eq!(endpoint.scope, AddressScope::Documentation);
    }

    #[test]
    fn zone_is_separate_and_legacy_json_reconstructs_scope() {
        let endpoint = Endpoint::parse("fe80::1%Ethernet", 5353).expect("zoned endpoint");
        assert_eq!(endpoint.zone.as_deref(), Some("Ethernet"));
        assert_eq!(endpoint.address, "fe80::1".parse::<IpAddr>().expect("IPv6"));
        assert_eq!(endpoint.scope, AddressScope::LinkLocal);
        assert_eq!(endpoint.label(), "[fe80::1%Ethernet]:5353");

        let legacy: Endpoint =
            serde_json::from_str(r#"{"address":"1.1.1.1","port":53}"#).expect("legacy endpoint");
        assert_eq!(legacy.scope, AddressScope::Global);
        assert_eq!(legacy.observed_family, AddressFamily::Ipv4);
    }

    #[test]
    fn canonical_addresses_roundtrip_for_representative_inputs() {
        for value in [
            "1.1.1.1",
            "10.0.0.1",
            "2001:4860:4860::8888",
            "2001:db8:0:0:0:0:0:1",
        ] {
            let parsed: IpAddr = value.parse().expect("input address");
            let canonical = canonicalize_address(parsed);
            assert_eq!(
                canonical
                    .to_string()
                    .parse::<IpAddr>()
                    .expect("formatted address"),
                canonical
            );
        }
    }
}
