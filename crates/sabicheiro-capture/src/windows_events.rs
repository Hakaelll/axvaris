use crate::{CaptureError, process::ProcessResolver};
use ferrisetw::{
    EventRecord, parser::Parser, provider::Provider, schema::Schema, schema_locator::SchemaLocator,
    trace::UserTrace,
};
use sabicheiro_core::{
    ConnectionEvidence, ConnectionKey, ConnectionState, CorrelationQuality, CounterSource,
    Direction, Endpoint, NetworkEventKind, NetworkProtocol, NormalizedNetworkEvent,
    ObservationSource, TrafficCounters,
};
use std::{
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
    },
};

const ETW_QUEUE_CAPACITY: usize = 8_192;
const DNS_TTL_MS: i64 = 3_600_000;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DnsObservation {
    pub address: IpAddr,
    pub hostname: String,
    pub canonical_name: Option<String>,
    pub ttl_seconds: u32,
    pub resolver: Option<String>,
    pub process_instance: Option<String>,
    pub observed_at_ms: i64,
    pub expires_at_ms: i64,
}

#[derive(Debug)]
enum RawEtwRecord {
    Network(RawNetworkEvent),
    Dns(RawDnsEvent),
}

#[derive(Debug)]
struct RawDnsEvent {
    observed_at_ms: i64,
    hostname: String,
    canonical_name: Option<String>,
    addresses: Vec<IpAddr>,
    ttl_seconds: u32,
    resolver: Option<String>,
    pid: Option<u32>,
}

#[derive(Debug)]
struct RawNetworkEvent {
    observed_at_ms: i64,
    kind: NetworkEventKind,
    pid: Option<u32>,
    protocol: NetworkProtocol,
    direction: Direction,
    state: ConnectionState,
    local: Endpoint,
    remote: Endpoint,
    traffic: TrafficCounters,
}

pub struct WindowsEventCapture {
    _trace: UserTrace,
    receiver: Receiver<RawEtwRecord>,
    dropped: Arc<AtomicU64>,
    pending: Arc<AtomicUsize>,
    processes: ProcessResolver,
}

impl WindowsEventCapture {
    pub fn start() -> Result<Self, CaptureError> {
        let (sender, receiver) = sync_channel(ETW_QUEUE_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let pending = Arc::new(AtomicUsize::new(0));

        let network_sender = sender.clone();
        let network_dropped = Arc::clone(&dropped);
        let network_pending = Arc::clone(&pending);
        let network_provider = Provider::by_guid("7dd42a49-5329-4832-8dfd-43d979153a88")
            .add_callback(move |record, schemas| {
                network_callback(
                    record,
                    schemas,
                    &network_sender,
                    &network_dropped,
                    &network_pending,
                );
            })
            .build();

        let dns_dropped = Arc::clone(&dropped);
        let dns_pending = Arc::clone(&pending);
        let dns_provider = Provider::by_guid("1c95126e-7eea-49a9-a3fe-a378b03ddb4d")
            .add_callback(move |record, schemas| {
                dns_callback(record, schemas, &sender, &dns_dropped, &dns_pending);
            })
            .build();

        let trace = UserTrace::new()
            .named("Sabicheiro-Network-Events".into())
            .enable(network_provider)
            .enable(dns_provider)
            .start_and_process()
            .map_err(|error| {
                CaptureError::PermissionDenied(format!(
                    "Windows ETW network events could not start ({error:?}); IP Helper polling remains active"
                ))
            })?;

        Ok(Self {
            _trace: trace,
            receiver,
            dropped,
            pending,
            processes: ProcessResolver::new(),
        })
    }

    pub fn drain(
        &mut self,
        _timestamp_ms: i64,
    ) -> (Vec<NormalizedNetworkEvent>, Vec<DnsObservation>) {
        let mut network = Vec::new();
        let mut raw_dns = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(RawEtwRecord::Network(event)) => network.push(event),
                Ok(RawEtwRecord::Dns(event)) => raw_dns.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
            self.pending.fetch_sub(1, Ordering::Relaxed);
        }
        self.processes.refresh(
            network
                .iter()
                .filter_map(|event| event.pid)
                .chain(raw_dns.iter().filter_map(|event| event.pid)),
        );
        let dns = raw_dns
            .into_iter()
            .flat_map(|event| {
                let timestamp_ms = event.observed_at_ms;
                let process_instance = self
                    .processes
                    .resolve_at(event.pid, timestamp_ms)
                    .instance_id();
                let ttl_ms = i64::from(event.ttl_seconds).saturating_mul(1_000);
                event
                    .addresses
                    .into_iter()
                    .map(move |address| DnsObservation {
                        address,
                        hostname: event.hostname.clone(),
                        canonical_name: event.canonical_name.clone(),
                        ttl_seconds: event.ttl_seconds,
                        resolver: event.resolver.clone(),
                        process_instance: process_instance.clone(),
                        observed_at_ms: timestamp_ms,
                        expires_at_ms: timestamp_ms.saturating_add(ttl_ms),
                    })
            })
            .collect();
        let events = network
            .into_iter()
            .map(|event| {
                let timestamp_ms = event.observed_at_ms;
                let process = self.processes.resolve_at(event.pid, timestamp_ms);
                let mut evidence = ConnectionEvidence::for_peer(
                    &event.remote,
                    event.state,
                    ObservationSource::WindowsEtw,
                    timestamp_ms,
                );
                evidence.correlation_quality = CorrelationQuality::Approximate;
                NormalizedNetworkEvent {
                    kind: event.kind,
                    key: ConnectionKey {
                        pid: event.pid,
                        process_instance: process.instance_id(),
                        local: event.local,
                        remote: event.remote,
                        protocol: event.protocol,
                        interface_id: None,
                        capture_generation: 0,
                        flow_generation: 0,
                    },
                    evidence,
                    timestamp_ms,
                    process,
                    direction: event.direction,
                    state: event.state,
                    interface: None,
                    hostname: None,
                    domain: None,
                    geo: None,
                    asn: None,
                    traffic_delta: event.traffic,
                    counter_source: CounterSource::EtwFlow,
                }
            })
            .collect();
        (events, dns)
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub const fn capacity(&self) -> usize {
        ETW_QUEUE_CAPACITY
    }

    pub fn pending(&self) -> usize {
        self.pending.load(Ordering::Relaxed)
    }
}

fn network_callback(
    record: &EventRecord,
    schemas: &SchemaLocator,
    sender: &SyncSender<RawEtwRecord>,
    dropped: &AtomicU64,
    pending: &AtomicUsize,
) {
    let Ok(schema) = schemas.event_schema(record) else {
        return;
    };
    let Some(event) = parse_network_event(record, &schema) else {
        return;
    };
    send_bounded(sender, RawEtwRecord::Network(event), dropped, pending);
}

fn parse_network_event(record: &EventRecord, schema: &Schema) -> Option<RawNetworkEvent> {
    let parser = Parser::create(record, schema);
    let opcode = record.opcode();
    let event_id = record.event_id();
    let operation = if opcode == 0 { event_id as u8 } else { opcode };
    let task = schema.task_name().to_ascii_lowercase();
    let opcode_name = schema.opcode_name().to_ascii_lowercase();
    let protocol = if task.contains("udp") || opcode_name.contains("udp") {
        NetworkProtocol::Udp
    } else {
        NetworkProtocol::Tcp
    };
    let (kind, direction, state) = match operation {
        10 | 26 => (
            NetworkEventKind::Updated,
            Direction::Outbound,
            ConnectionState::Established,
        ),
        11 | 27 => (
            NetworkEventKind::Updated,
            Direction::Inbound,
            ConnectionState::Established,
        ),
        12 | 16 | 28 | 32 => (
            NetworkEventKind::Opened,
            Direction::Outbound,
            ConnectionState::Established,
        ),
        15 | 31 => (
            NetworkEventKind::Opened,
            Direction::Inbound,
            ConnectionState::Established,
        ),
        13 | 17 | 29 | 33 => (
            NetworkEventKind::Closed,
            Direction::Unknown,
            ConnectionState::Closed,
        ),
        14 | 30 => (
            NetworkEventKind::Updated,
            Direction::Outbound,
            ConnectionState::Established,
        ),
        // Copy events duplicate send bytes and are intentionally ignored.
        18 | 34 => return None,
        _ => return None,
    };
    let source_address: IpAddr = parser.try_parse("saddr").ok()?;
    let source_port: u16 = parser.try_parse("sport").ok()?;
    let destination_address: IpAddr = parser.try_parse("daddr").ok()?;
    let destination_port: u16 = parser.try_parse("dport").ok()?;
    if source_port == 0 && destination_port == 0 {
        return None;
    }
    let size: u64 = parser.try_parse::<u32>("size").unwrap_or(0).into();
    let (local, remote) = if direction == Direction::Inbound {
        (
            Endpoint::new(destination_address, destination_port),
            Endpoint::new(source_address, source_port),
        )
    } else {
        (
            Endpoint::new(source_address, source_port),
            Endpoint::new(destination_address, destination_port),
        )
    };
    let mut traffic = TrafficCounters::default();
    if direction == Direction::Inbound && size > 0 {
        traffic.bytes_received = size;
        traffic.packets_received = 1;
    } else if size > 0 {
        traffic.bytes_sent = size;
        traffic.packets_sent = 1;
    }
    let payload_pid = parser.try_parse::<u32>("PID").ok();
    let header_pid = (record.process_id() != 0).then_some(record.process_id());
    Some(RawNetworkEvent {
        observed_at_ms: event_timestamp_ms(record),
        kind,
        pid: payload_pid.filter(|pid| *pid != 0).or(header_pid),
        protocol,
        direction,
        state,
        local,
        remote,
        traffic,
    })
}

fn dns_callback(
    record: &EventRecord,
    schemas: &SchemaLocator,
    sender: &SyncSender<RawEtwRecord>,
    dropped: &AtomicU64,
    pending: &AtomicUsize,
) {
    let Ok(schema) = schemas.event_schema(record) else {
        return;
    };
    let parser = Parser::create(record, &schema);
    let status = parser
        .try_parse::<u32>("QueryStatus")
        .or_else(|_| parser.try_parse("Status"))
        .unwrap_or(0);
    if status != 0 {
        return;
    }
    let Some(hostname) = parser
        .try_parse::<String>("QueryName")
        .ok()
        .map(|value| value.trim_end_matches('.').to_owned())
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let addresses = parser
        .try_parse::<String>("QueryResults")
        .ok()
        .map(|results| extract_addresses(&results))
        .unwrap_or_default();
    if addresses.is_empty() {
        return;
    }
    let canonical_name = parser
        .try_parse::<String>("CName")
        .or_else(|_| parser.try_parse("CanonicalName"))
        .ok()
        .map(|value| value.trim_end_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let ttl_seconds = parser
        .try_parse::<u32>("TTL")
        .or_else(|_| parser.try_parse("Ttl"))
        .unwrap_or((DNS_TTL_MS / 1_000) as u32)
        .clamp(1, 86_400);
    let resolver = parser
        .try_parse::<String>("ServerAddress")
        .or_else(|_| parser.try_parse("DnsServer"))
        .ok()
        .filter(|value| !value.trim().is_empty());
    let payload_pid = parser.try_parse::<u32>("PID").ok();
    let header_pid = (record.process_id() != 0).then_some(record.process_id());
    send_bounded(
        sender,
        RawEtwRecord::Dns(RawDnsEvent {
            observed_at_ms: event_timestamp_ms(record),
            hostname,
            canonical_name,
            addresses,
            ttl_seconds,
            resolver,
            pid: payload_pid.filter(|pid| *pid != 0).or(header_pid),
        }),
        dropped,
        pending,
    );
}

fn event_timestamp_ms(record: &EventRecord) -> i64 {
    record
        .raw_timestamp()
        .saturating_div(10_000)
        .saturating_sub(11_644_473_600_000)
}

fn extract_addresses(results: &str) -> Vec<IpAddr> {
    let mut addresses = results
        .split(|character: char| {
            character.is_ascii_whitespace()
                || matches!(character, ';' | ',' | '[' | ']' | '(' | ')')
        })
        .filter_map(|token| {
            let token = token.trim_matches(|character: char| {
                !(character.is_ascii_hexdigit() || matches!(character, '.' | ':'))
            });
            token.parse().ok()
        })
        .collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    addresses
}

fn send_bounded(
    sender: &SyncSender<RawEtwRecord>,
    event: RawEtwRecord,
    dropped: &AtomicU64,
    pending: &AtomicUsize,
) {
    match sender.try_send(event) {
        Ok(()) => {
            pending.fetch_add(1, Ordering::Relaxed);
        }
        Err(TrySendError::Full(_)) => {
            dropped.fetch_add(1, Ordering::Relaxed);
        }
        Err(TrySendError::Disconnected(_)) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires a live Windows ETW session"]
    fn live_etw_observes_loopback_flow_counters() {
        use std::{
            io::{Read, Write},
            net::{Ipv4Addr, TcpListener, TcpStream},
            thread,
            time::{Duration, Instant},
        };

        let mut capture = WindowsEventCapture::start().expect("start ETW event capture");
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind loopback listener");
        let listener_address = listener.local_addr().expect("listener address");
        let mut client = TcpStream::connect(listener_address).expect("connect loopback client");
        let client_address = client.local_addr().expect("client address");
        let (mut server, _) = listener.accept().expect("accept loopback client");
        client
            .write_all(&[0x5a; 4_096])
            .expect("write outbound proof bytes");
        let mut inbound = [0_u8; 4_096];
        server
            .read_exact(&mut inbound)
            .expect("read outbound proof bytes");
        server
            .write_all(&[0xa5; 2_048])
            .expect("write inbound proof bytes");
        let mut outbound = [0_u8; 2_048];
        client
            .read_exact(&mut outbound)
            .expect("read inbound proof bytes");

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut observed = Vec::new();
        while Instant::now() < deadline {
            let (events, _) = capture.drain(sabicheiro_core::now_ms());
            observed.extend(events);
            if observed.iter().any(|event| {
                event.key.pid == Some(std::process::id())
                    && [event.key.local.port, event.key.remote.port]
                        .contains(&listener_address.port())
                    && [event.key.local.port, event.key.remote.port]
                        .contains(&client_address.port())
                    && (event.traffic_delta.bytes_sent > 0
                        || event.traffic_delta.bytes_received > 0)
            }) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "ETW started but did not report the loopback flow with byte counters; observed {} parsed events",
            observed.len()
        );
    }

    #[test]
    fn dns_result_parser_extracts_ipv4_and_ipv6_without_names() {
        assert_eq!(
            extract_addresses("type: 5 example.test; type: 1 203.0.113.10; 2001:db8::10"),
            vec![
                "203.0.113.10".parse::<IpAddr>().expect("IPv4"),
                "2001:db8::10".parse::<IpAddr>().expect("IPv6"),
            ]
        );
    }
}
