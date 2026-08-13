use crate::{
    AsnInfo, BaselineBook, BaselineObservation, CaptureMode, CaptureStatus, CaptureSummary,
    Connection, ConnectionKey, ConnectionState, CounterSource, DashboardSnapshot, DashboardStats,
    DestinationSummary, Diagnostics, EndpointRole, GeoLocation, NetworkEventKind,
    NormalizedNetworkEvent, ProcessInfo, ProcessSummary, SessionSummary, TrafficCounters,
    TransferRates,
};
use std::collections::{HashMap, HashSet};

const DEFAULT_VISUAL_TTL_MS: i64 = 8_000;
const MAX_VISIBLE_CONNECTIONS: usize = 2_000;
const MAX_IPC_PROCESSES: usize = 1_000;
const MAX_IPC_DESTINATIONS: usize = 2_000;

pub struct ConnectionAggregator {
    connections: HashMap<ConnectionKey, Connection>,
    baselines: BaselineBook,
    next_id: u64,
    revision: u64,
    visual_ttl_ms: i64,
}

impl Default for ConnectionAggregator {
    fn default() -> Self {
        Self::new(DEFAULT_VISUAL_TTL_MS)
    }
}

impl ConnectionAggregator {
    pub fn new(visual_ttl_ms: i64) -> Self {
        Self {
            connections: HashMap::new(),
            baselines: BaselineBook::default(),
            next_id: 1,
            revision: 0,
            visual_ttl_ms: visual_ttl_ms.max(0),
        }
    }

    pub fn with_baselines(visual_ttl_ms: i64, baselines: BaselineBook) -> Self {
        Self {
            baselines,
            ..Self::new(visual_ttl_ms)
        }
    }

    pub fn from_baselines(baselines: BaselineBook) -> Self {
        Self::with_baselines(DEFAULT_VISUAL_TTL_MS, baselines)
    }

    pub const fn baselines(&self) -> &BaselineBook {
        &self.baselines
    }

    pub fn apply_batch<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = NormalizedNetworkEvent>,
    {
        for event in events {
            self.apply(event);
        }
    }

    pub fn apply(&mut self, event: NormalizedNetworkEvent) {
        let destination_key = event
            .domain
            .as_deref()
            .or(event.hostname.as_deref())
            .map(str::to_owned)
            .unwrap_or_else(|| event.key.remote.address.to_string());
        let process_key = baseline_process_key(&event.process);
        let existed = self.connections.contains_key(&event.key);
        let baseline_eligible = event.evidence.endpoint_role == EndpointRole::RemotePeer
            && event.state != ConnectionState::Listening
            && !event.key.remote.address.is_unspecified()
            && event.key.remote.port != 0;

        if !existed {
            let signals = if baseline_eligible {
                self.baselines.observe(BaselineObservation {
                    process_key: &process_key,
                    destination_key: &destination_key,
                    connection: &event.key,
                    geo: event.geo.as_ref(),
                    timestamp_ms: event.timestamp_ms,
                })
            } else {
                Vec::new()
            };
            let id = format!("connection-{}-{}", event.timestamp_ms, self.next_id);
            self.next_id = self.next_id.saturating_add(1);
            self.connections.insert(
                event.key.clone(),
                Connection {
                    id,
                    key: event.key.clone(),
                    evidence: event.evidence.clone(),
                    process: event.process.clone(),
                    protocol: event.key.protocol,
                    direction: event.direction,
                    state: event.state,
                    interface: event.interface.clone(),
                    hostname: event.hostname.clone(),
                    domain: event.domain.clone(),
                    geo: event.geo.clone(),
                    asn: event.asn.clone(),
                    traffic: TrafficCounters::default(),
                    counter_source: event.counter_source,
                    rates: TransferRates::default(),
                    first_seen_ms: event.timestamp_ms,
                    last_seen_ms: event.timestamp_ms,
                    duration_ms: 0,
                    active: event.kind != NetworkEventKind::Closed,
                    first_seen_destination: signals
                        .iter()
                        .any(|signal| signal.kind == crate::SignalKind::NewDestination),
                    repeat_count: 1,
                    unusual: signals,
                    visual_expires_ms: None,
                },
            );
        }

        if let Some(connection) = self.connections.get_mut(&event.key) {
            let elapsed_ms = event
                .timestamp_ms
                .saturating_sub(connection.last_seen_ms)
                .max(1);
            connection.traffic.add_assign(event.traffic_delta);
            if event.counter_source != CounterSource::Unavailable {
                connection.counter_source = event.counter_source;
            }
            connection.rates = TransferRates {
                down_bytes_per_second: event.traffic_delta.bytes_received as f64 * 1_000.0
                    / elapsed_ms as f64,
                up_bytes_per_second: event.traffic_delta.bytes_sent as f64 * 1_000.0
                    / elapsed_ms as f64,
                packets_per_second: (event.traffic_delta.packets_received
                    + event.traffic_delta.packets_sent) as f64
                    * 1_000.0
                    / elapsed_ms as f64,
            };
            connection.last_seen_ms = connection.last_seen_ms.max(event.timestamp_ms);
            connection.duration_ms = connection
                .last_seen_ms
                .saturating_sub(connection.first_seen_ms)
                as u64;
            connection.process = event.process;
            connection.evidence = event.evidence.clone();
            connection.direction = event.direction;
            connection.state = event.state;
            connection.interface = event.interface.or_else(|| connection.interface.clone());
            connection.hostname = event.hostname.or_else(|| connection.hostname.clone());
            connection.domain = event.domain.or_else(|| connection.domain.clone());
            connection.geo = event.geo.or_else(|| connection.geo.clone());
            connection.asn = event.asn.or_else(|| connection.asn.clone());
            let activity_signals = if baseline_eligible {
                self.baselines.observe_activity(
                    &process_key,
                    event
                        .traffic_delta
                        .bytes_sent
                        .saturating_add(event.traffic_delta.bytes_received),
                    elapsed_ms,
                    connection.duration_ms,
                )
            } else {
                Vec::new()
            };
            for signal in activity_signals {
                if !connection
                    .unusual
                    .iter()
                    .any(|existing| existing.kind == signal.kind)
                {
                    connection.unusual.push(signal);
                }
            }

            match event.kind {
                NetworkEventKind::Opened => {
                    if existed
                        && event.evidence.correlation_quality
                            != crate::CorrelationQuality::Correlated
                    {
                        connection.repeat_count = connection.repeat_count.saturating_add(1);
                    }
                    connection.active = true;
                    connection.visual_expires_ms = None;
                }
                NetworkEventKind::Updated => {
                    connection.active = true;
                    connection.visual_expires_ms = None;
                }
                NetworkEventKind::Closed => {
                    connection.active = false;
                    connection.visual_expires_ms =
                        Some(event.timestamp_ms.saturating_add(self.visual_ttl_ms));
                }
            }
        }

        self.revision = self.revision.saturating_add(1);
    }

    pub fn prune(&mut self, now_ms: i64) {
        self.connections.retain(|_, connection| {
            connection.active
                || connection
                    .visual_expires_ms
                    .is_some_and(|expires| expires > now_ms)
        });
        if self.connections.len() > MAX_VISIBLE_CONNECTIONS {
            let mut ended = self
                .connections
                .iter()
                .filter(|(_, connection)| !connection.active)
                .map(|(key, connection)| (key.clone(), connection.last_seen_ms))
                .collect::<Vec<_>>();
            ended.sort_by_key(|(_, last_seen)| *last_seen);
            let excess = self.connections.len() - MAX_VISIBLE_CONNECTIONS;
            for (key, _) in ended.into_iter().take(excess) {
                self.connections.remove(&key);
            }
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn enrich(
        &mut self,
        key: &ConnectionKey,
        hostname: Option<String>,
        domain: Option<String>,
        geo: Option<GeoLocation>,
        asn: Option<AsnInfo>,
    ) -> bool {
        let Some(connection) = self.connections.get_mut(key) else {
            return false;
        };
        connection.hostname = hostname.or_else(|| connection.hostname.clone());
        connection.domain = domain.or_else(|| connection.domain.clone());
        connection.geo = geo.or_else(|| connection.geo.clone());
        connection.asn = asn.or_else(|| connection.asn.clone());
        self.revision = self.revision.saturating_add(1);
        true
    }

    pub fn connections(&self) -> impl Iterator<Item = &Connection> {
        self.connections.values()
    }

    pub fn snapshot(
        &self,
        generated_at_ms: i64,
        capture: CaptureSummary,
        diagnostics: Diagnostics,
        session: Option<SessionSummary>,
    ) -> DashboardSnapshot {
        let mut connections = self.connections.values().cloned().collect::<Vec<_>>();
        connections.sort_by(|left, right| {
            right
                .active
                .cmp(&left.active)
                .then_with(|| right.last_seen_ms.cmp(&left.last_seen_ms))
        });

        let mut processes = summarize_processes(&connections);
        let mut destinations = summarize_destinations(&connections);
        let country_count = connections
            .iter()
            .filter_map(|connection| connection.geo.as_ref())
            .filter_map(|geo| geo.country_code.as_deref())
            .collect::<HashSet<_>>()
            .len();
        let stats = DashboardStats {
            active_connections: connections
                .iter()
                .filter(|connection| connection.active)
                .count(),
            down_bytes_per_second: connections
                .iter()
                .filter(|connection| connection.active)
                .map(|connection| connection.rates.down_bytes_per_second)
                .sum(),
            up_bytes_per_second: connections
                .iter()
                .filter(|connection| connection.active)
                .map(|connection| connection.rates.up_bytes_per_second)
                .sum(),
            packets_per_second: connections
                .iter()
                .filter(|connection| connection.active)
                .map(|connection| connection.rates.packets_per_second)
                .sum(),
            countries: country_count,
            processes: processes.len(),
            total_connections: connections.len(),
        };
        // Aggregate totals are computed from the complete in-memory set, while the
        // high-frequency IPC payload remains explicitly bounded. Retained history
        // remains queryable from SQLite and is not discarded by this presentation cap.
        connections.truncate(MAX_VISIBLE_CONNECTIONS);
        processes.truncate(MAX_IPC_PROCESSES);
        destinations.truncate(MAX_IPC_DESTINATIONS);

        DashboardSnapshot {
            generated_at_ms,
            revision: self.revision,
            connections,
            processes,
            destinations,
            stats,
            diagnostics,
            capture,
            session,
        }
    }

    pub fn empty_snapshot(&self, generated_at_ms: i64) -> DashboardSnapshot {
        self.snapshot(
            generated_at_ms,
            CaptureSummary {
                mode: CaptureMode::Off,
                status: CaptureStatus::Stopped,
                selected_interface: None,
                paused_view_events: 0,
                vpn_active: false,
            },
            Diagnostics::default(),
            None,
        )
    }
}

fn process_key(process: &ProcessInfo) -> String {
    process
        .pid
        .map(|pid| format!("{}:{pid}", process.name))
        .unwrap_or_else(|| process.name.clone())
}

fn baseline_process_key(process: &ProcessInfo) -> String {
    process
        .executable
        .as_deref()
        .unwrap_or(&process.name)
        .to_lowercase()
}

fn summarize_processes(connections: &[Connection]) -> Vec<ProcessSummary> {
    let mut summaries: HashMap<String, ProcessSummary> = HashMap::new();
    let mut countries: HashMap<String, HashSet<String>> = HashMap::new();
    for connection in connections {
        let key = process_key(&connection.process);
        let summary = summaries
            .entry(key.clone())
            .or_insert_with(|| ProcessSummary {
                key: key.clone(),
                process: connection.process.clone(),
                active_connections: 0,
                total_connections: 0,
                traffic: TrafficCounters::default(),
                countries: 0,
            });
        summary.total_connections += 1;
        summary.active_connections += usize::from(connection.active);
        summary.traffic.add_assign(connection.traffic);
        if let Some(geo) = &connection.geo {
            if let Some(country_code) = &geo.country_code {
                countries
                    .entry(key)
                    .or_default()
                    .insert(country_code.clone());
            }
        }
    }
    for (key, summary) in &mut summaries {
        summary.countries = countries.get(key).map_or(0, HashSet::len);
    }
    let mut result = summaries.into_values().collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right
            .active_connections
            .cmp(&left.active_connections)
            .then_with(|| {
                right
                    .traffic
                    .bytes_received
                    .cmp(&left.traffic.bytes_received)
            })
    });
    result
}

fn summarize_destinations(connections: &[Connection]) -> Vec<DestinationSummary> {
    let mut summaries: HashMap<String, DestinationSummary> = HashMap::new();
    let mut processes: HashMap<String, HashSet<String>> = HashMap::new();
    for connection in connections {
        let key = connection
            .domain
            .clone()
            .or_else(|| connection.hostname.clone())
            .unwrap_or_else(|| connection.key.remote.address.to_string());
        let summary = summaries
            .entry(key.clone())
            .or_insert_with(|| DestinationSummary {
                key: key.clone(),
                ip: connection.key.remote.address,
                domain: connection
                    .domain
                    .clone()
                    .or_else(|| connection.hostname.clone()),
                country_code: connection
                    .geo
                    .as_ref()
                    .and_then(|geo| geo.country_code.clone()),
                country_name: connection
                    .geo
                    .as_ref()
                    .and_then(|geo| geo.country_name.clone()),
                asn: connection.asn.clone(),
                active_connections: 0,
                total_connections: 0,
                process_names: Vec::new(),
                traffic: TrafficCounters::default(),
                ports: Vec::new(),
                first_seen_ms: connection.first_seen_ms,
                last_seen_ms: connection.last_seen_ms,
                recurrence: 0,
                geo_precision: connection
                    .geo
                    .as_ref()
                    .map_or_else(Default::default, |geo| geo.precision),
                placement: connection
                    .geo
                    .as_ref()
                    .map_or_else(Default::default, |geo| geo.placement),
            });
        summary.total_connections += 1;
        summary.active_connections += usize::from(connection.active);
        summary.traffic.add_assign(connection.traffic);
        summary.first_seen_ms = summary.first_seen_ms.min(connection.first_seen_ms);
        summary.last_seen_ms = summary.last_seen_ms.max(connection.last_seen_ms);
        summary.recurrence = summary
            .recurrence
            .saturating_add(connection.repeat_count.max(1));
        if !summary.ports.contains(&connection.key.remote.port) {
            summary.ports.push(connection.key.remote.port);
            summary.ports.sort_unstable();
        }
        processes
            .entry(key)
            .or_default()
            .insert(connection.process.name.clone());
    }
    for (key, summary) in &mut summaries {
        let mut names = processes
            .remove(key)
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        names.sort();
        summary.process_names = names;
    }
    let mut result = summaries.into_values().collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right
            .active_connections
            .cmp(&left.active_connections)
            .then_with(|| right.total_connections.cmp(&left.total_connections))
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConnectionState, Direction, Endpoint, NetworkProtocol, NormalizedNetworkEvent, ProcessInfo,
        TrafficCounters,
    };

    fn event(kind: NetworkEventKind, timestamp_ms: i64) -> NormalizedNetworkEvent {
        NormalizedNetworkEvent {
            kind,
            key: ConnectionKey {
                pid: Some(42),
                process_instance: Some("pid:42:start:100".into()),
                local: Endpoint::new("192.168.1.10".parse().expect("IP"), 50_000),
                remote: Endpoint::new("1.1.1.1".parse().expect("IP"), 443),
                protocol: NetworkProtocol::Tcp,
                interface_id: Some("test-interface".into()),
                capture_generation: 1,
                flow_generation: 1,
            },
            evidence: crate::ConnectionEvidence::for_peer(
                &Endpoint::new("1.1.1.1".parse().expect("IP"), 443),
                ConnectionState::Established,
                crate::ObservationSource::Demo,
                timestamp_ms,
            ),
            timestamp_ms,
            process: ProcessInfo {
                pid: Some(42),
                name: "Browser".into(),
                ..ProcessInfo::default()
            },
            direction: Direction::Outbound,
            state: if kind == NetworkEventKind::Closed {
                ConnectionState::Closed
            } else {
                ConnectionState::Established
            },
            interface: Some("Ethernet".into()),
            hostname: Some("one.one.one.one".into()),
            domain: Some("one.one.one.one".into()),
            geo: None,
            asn: None,
            traffic_delta: TrafficCounters {
                bytes_sent: 10,
                bytes_received: 20,
                packets_sent: 1,
                packets_received: 2,
            },
            counter_source: CounterSource::EtwFlow,
        }
    }

    #[test]
    fn aggregates_deltas_and_keeps_closed_visual_ttl() {
        let mut aggregator = ConnectionAggregator::new(1_000);
        aggregator.apply(event(NetworkEventKind::Opened, 1_000));
        aggregator.apply(event(NetworkEventKind::Updated, 2_000));
        aggregator.apply(event(NetworkEventKind::Closed, 3_000));

        let connection = aggregator.connections().next().expect("connection");
        assert_eq!(connection.traffic.bytes_received, 60);
        assert!(!connection.active);
        assert_eq!(connection.visual_expires_ms, Some(4_000));

        aggregator.prune(3_999);
        assert_eq!(aggregator.connection_count(), 1);
        aggregator.prune(4_000);
        assert_eq!(aggregator.connection_count(), 0);
    }

    #[test]
    fn summaries_are_derived_from_same_connections() {
        let mut aggregator = ConnectionAggregator::default();
        aggregator.apply(event(NetworkEventKind::Opened, 1_000));
        let snapshot = aggregator.empty_snapshot(1_000);
        assert_eq!(snapshot.stats.active_connections, 1);
        assert_eq!(snapshot.processes[0].process.name, "Browser");
        assert_eq!(
            snapshot.destinations[0].domain.as_deref(),
            Some("one.one.one.one")
        );
    }

    #[test]
    fn snapshot_keeps_complete_totals_but_bounds_high_frequency_ipc_rows() {
        let mut aggregator = ConnectionAggregator::default();
        for index in 0_u32..2_105 {
            let mut item = event(NetworkEventKind::Opened, 1_000 + i64::from(index));
            item.key.remote.address = std::net::Ipv4Addr::new(
                11,
                ((index >> 16) & 0xff) as u8,
                ((index >> 8) & 0xff) as u8,
                (index & 0xff) as u8,
            )
            .into();
            item.key.flow_generation = u64::from(index) + 1;
            item.domain = None;
            item.hostname = None;
            aggregator.apply(item);
        }

        let snapshot = aggregator.empty_snapshot(4_000);
        assert_eq!(snapshot.stats.total_connections, 2_105);
        assert_eq!(snapshot.stats.active_connections, 2_105);
        assert_eq!(snapshot.connections.len(), MAX_VISIBLE_CONNECTIONS);
        assert_eq!(snapshot.destinations.len(), MAX_IPC_DESTINATIONS);
    }

    #[test]
    fn listeners_do_not_pollute_remote_destination_baselines() {
        let mut aggregator = ConnectionAggregator::default();
        let mut listener = event(NetworkEventKind::Opened, 1_000);
        listener.key.remote = Endpoint::new("0.0.0.0".parse().expect("IP"), 0);
        listener.state = ConnectionState::Listening;
        listener.domain = None;
        listener.hostname = None;
        aggregator.apply(listener);

        let mut remote = event(NetworkEventKind::Opened, 2_000);
        remote.key.remote = Endpoint::new("8.8.8.8".parse().expect("IP"), 443);
        remote.domain = Some("dns.google".into());
        aggregator.apply(remote);

        let remote_connection = aggregator
            .connections()
            .find(|connection| connection.key.remote.address.to_string() == "8.8.8.8")
            .expect("remote connection");
        assert!(remote_connection.unusual.is_empty());
        assert!(
            aggregator
                .connections()
                .find(|connection| connection.state == ConnectionState::Listening)
                .is_some_and(|connection| connection.unusual.is_empty())
        );
    }
}
