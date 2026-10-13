use crate::{CaptureError, process::ProcessResolver};
use aya::{
    Ebpf,
    maps::{Array, RingBuf},
    programs::{CgroupAttachMode, CgroupSkb, CgroupSkbAttachType, TracePoint},
};
use sabicheiro_core::{
    CapabilityAvailability, CaptureCapabilityKind, ConnectionEvidence, ConnectionKey,
    ConnectionState, CorrelationQuality, CounterSource, Direction, Endpoint, NetworkEventKind,
    NetworkProtocol, NormalizedNetworkEvent, ObservationSource, ProcessInfo, ProviderCapability,
    ProviderDescriptor, ProviderMode, TrafficCounters,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::{Receiver, TrySendError, sync_channel},
    },
    thread,
    time::{Duration, Instant},
};

const MAX_PENDING: usize = 8192;
const MAX_FLOWS: usize = 65536;
const MAX_FRAME: u64 = 65536;
const HELPER_PATH: &str = "/usr/libexec/axvaris-capture-helper";

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Frame {
    Ready { descriptor: ProviderDescriptor },
    Event { event: Box<NormalizedNetworkEvent> },
    Health { lost: u64 },
}

pub struct LinuxEventCapture {
    child: Child,
    receiver: Receiver<NormalizedNetworkEvent>,
    dropped: Arc<AtomicU64>,
    alive: Arc<AtomicBool>,
    descriptor: ProviderDescriptor,
    pending: Arc<AtomicUsize>,
}

impl LinuxEventCapture {
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }
    pub fn start() -> Result<Self, CaptureError> {
        for directory in ["/usr", "/usr/libexec"] {
            let metadata = fs::metadata(directory)?;
            if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
                return Err(CaptureError::PermissionDenied(
                    "eBPF helper directories must be root-owned without group/world write permission".into(),
                ));
            }
        }
        let metadata = fs::metadata(HELPER_PATH)?;
        if !metadata.is_file() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err(CaptureError::PermissionDenied(
                "eBPF helper must be a root-owned file without group/world write permission".into(),
            ));
        }
        let mut child = Command::new(HELPER_PATH)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            return Err(CaptureError::OperatingSystem(
                "helper stdout unavailable".into(),
            ));
        };
        let (ready_tx, ready_rx) = sync_channel(1);
        let (sender, receiver) = sync_channel(MAX_PENDING);
        let dropped = Arc::new(AtomicU64::new(0));
        let alive = Arc::new(AtomicBool::new(true));
        let pending = Arc::new(AtomicUsize::new(0));
        let reader_pending = Arc::clone(&pending);
        let reader_drops = Arc::clone(&dropped);
        let reader_alive = Arc::clone(&alive);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut kernel_lost = 0;
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                let Ok(size) = reader
                    .by_ref()
                    .take(MAX_FRAME + 1)
                    .read_until(b'\n', &mut buffer)
                else {
                    break;
                };
                if size == 0 || size as u64 > MAX_FRAME || buffer.last() != Some(&b'\n') {
                    break;
                }
                let Ok(frame) = serde_json::from_slice::<Frame>(&buffer) else {
                    break;
                };
                match frame {
                    Frame::Ready { descriptor } => {
                        if ready_tx.try_send(descriptor).is_err() {
                            break;
                        }
                    }
                    Frame::Event { event } => {
                        reader_pending.fetch_add(1, Ordering::Relaxed);
                        match sender.try_send(*event) {
                            Ok(()) => {}
                            Err(TrySendError::Full(_)) => {
                                reader_pending.fetch_sub(1, Ordering::Relaxed);
                                reader_drops.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                reader_pending.fetch_sub(1, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                    Frame::Health { lost } => {
                        reader_drops.fetch_add(lost.saturating_sub(kernel_lost), Ordering::Relaxed);
                        kernel_lost = kernel_lost.max(lost);
                    }
                }
            }
            reader_alive.store(false, Ordering::Relaxed);
        });
        match ready_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(descriptor) => Ok(Self {
                child,
                receiver,
                dropped,
                alive,
                descriptor,
                pending,
            }),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(CaptureError::PermissionDenied(
                    "eBPF helper could not attach; check kernel support, tracefs and helper capabilities".into(),
                ))
            }
        }
    }

    pub fn drain(&mut self) -> (Vec<NormalizedNetworkEvent>, u64) {
        let events: Vec<_> = self.receiver.try_iter().take(MAX_PENDING).collect();
        self.pending.fetch_sub(events.len(), Ordering::Relaxed);
        (events, self.dropped.load(Ordering::Relaxed))
    }
    pub fn queue(&self) -> (usize, usize) {
        (self.pending.load(Ordering::Relaxed), MAX_PENDING)
    }

    pub fn descriptor(&self, generation: u64) -> ProviderDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor.generation = generation.max(1);
        if !self.alive.load(Ordering::Relaxed) {
            for capability in &mut descriptor.capabilities {
                capability.availability = CapabilityAvailability::Unavailable;
                capability.limitation =
                    Some("helper stopped; INET_DIAG polling remains active".into());
            }
        }
        descriptor
    }
}

impl Drop for LinuxEventCapture {
    fn drop(&mut self) {
        self.child.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn linux_event_descriptor(generation: u64) -> ProviderDescriptor {
    let capabilities = [
        (
            CaptureCapabilityKind::LifecycleEvents,
            "TCP state changes; UDP closes use a 30-second inactivity timeout",
        ),
        (
            CaptureCapabilityKind::ProcessIdentity,
            "active TCP opens carry PID; passive TCP and UDP packet owners remain unknown",
        ),
        (
            CaptureCapabilityKind::FlowBytes,
            "UDP payload bytes only; TCP byte counters are unavailable",
        ),
        (
            CaptureCapabilityKind::FlowPackets,
            "UDP datagrams only; fragmented IPv4 and IPv6 extension headers are not decoded",
        ),
        (
            CaptureCapabilityKind::Interface,
            "UDP interface index only; TCP interface comes from snapshots",
        ),
        (
            CaptureCapabilityKind::NetworkNamespace,
            "TCP tracepoint is system-wide; UDP covers sockets under the attached cgroup",
        ),
    ]
    .map(|(kind, limitation)| ProviderCapability {
        kind,
        source: ObservationSource::LinuxEbpf,
        availability: CapabilityAvailability::Degraded,
        limitation: Some(limitation.into()),
    })
    .to_vec();
    ProviderDescriptor {
        protocol_version: 1,
        provider_id: "linux.ebpf".into(),
        display_name: "Linux Aya eBPF".into(),
        mode: ProviderMode::EventStream,
        generation,
        capabilities,
    }
}

fn emit(writer: &mut impl Write, frame: &Frame) -> Result<(), CaptureError> {
    serde_json::to_writer(&mut *writer, frame)
        .map_err(|error| CaptureError::InvalidData(error.to_string()))?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn validate_tracepoint() -> Result<(), CaptureError> {
    let format = fs::read_to_string("/sys/kernel/tracing/events/sock/inet_sock_set_state/format")
        .or_else(|_| {
        fs::read_to_string("/sys/kernel/debug/tracing/events/sock/inet_sock_set_state/format")
    })?;
    for (field, offset) in [
        ("skaddr", 8),
        ("newstate", 20),
        ("sport", 24),
        ("dport", 26),
        ("family", 28),
        ("protocol", 30),
        ("saddr", 32),
        ("daddr", 36),
        ("saddr_v6", 40),
        ("daddr_v6", 56),
    ] {
        if !format.lines().any(|line| {
            line.split(';')
                .next()
                .is_some_and(|value| value.split_whitespace().last() == Some(field))
                && line.contains(&format!("offset:{offset};"))
        }) {
            // Array names are rendered with their dimensions in tracefs.
            if !format.lines().any(|line| {
                line.contains(&format!("{field}[")) && line.contains(&format!("offset:{offset};"))
            }) {
                return Err(CaptureError::InvalidData(format!(
                    "unsupported TCP tracepoint layout for {field}"
                )));
            }
        }
    }
    Ok(())
}

pub fn run_linux_helper() -> Result<(), CaptureError> {
    validate_tracepoint()?;
    let mut ebpf = Ebpf::load(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/axvaris-network.bpf.o"
    )))
    .map_err(|error| CaptureError::OperatingSystem(error.to_string()))?;
    let pid_namespace = fs::metadata("/proc/self/ns/pid")?;
    let mut namespace = Array::<_, [u64; 2]>::try_from(
        ebpf.map_mut("PID_NAMESPACE")
            .ok_or_else(|| CaptureError::InvalidData("PID namespace map missing".into()))?,
    )
    .map_err(|error| CaptureError::OperatingSystem(error.to_string()))?;
    namespace
        .set(0, [pid_namespace.dev(), pid_namespace.ino()], 0)
        .map_err(|error| CaptureError::OperatingSystem(error.to_string()))?;
    let tcp: &mut TracePoint = ebpf
        .program_mut("tcp_state")
        .ok_or_else(|| CaptureError::InvalidData("TCP program missing".into()))?
        .try_into()
        .map_err(|error: aya::programs::ProgramError| {
            CaptureError::OperatingSystem(error.to_string())
        })?;
    tcp.load()
        .and_then(|_| tcp.attach("sock", "inet_sock_set_state").map(|_| ()))
        .map_err(|error| CaptureError::PermissionDenied(format!("TCP tracepoint: {error:?}")))?;
    let cgroup = File::open("/sys/fs/cgroup")?;
    for (name, attach) in [
        ("udp_egress", CgroupSkbAttachType::Egress),
        ("udp_ingress", CgroupSkbAttachType::Ingress),
    ] {
        let program: &mut CgroupSkb = ebpf
            .program_mut(name)
            .ok_or_else(|| CaptureError::InvalidData("UDP program missing".into()))?
            .try_into()
            .map_err(|error: aya::programs::ProgramError| {
                CaptureError::OperatingSystem(error.to_string())
            })?;
        program
            .load()
            .and_then(|_| {
                program
                    // Ring buffers require 5.8+, where cgroup BPF links are additive
                    // and reject the older BPF_PROG_ATTACH flags.
                    .attach(&cgroup, attach, CgroupAttachMode::Single)
                    .map(|_| ())
            })
            .map_err(|error| CaptureError::PermissionDenied(format!("{name}: {error:?}")))?;
    }
    let mut ring = RingBuf::try_from(
        ebpf.take_map("EVENTS")
            .ok_or_else(|| CaptureError::InvalidData("event ring missing".into()))?,
    )
    .map_err(|error| CaptureError::OperatingSystem(error.to_string()))?;
    let losses = Array::<_, u64>::try_from(
        ebpf.take_map("LOST")
            .ok_or_else(|| CaptureError::InvalidData("loss map missing".into()))?,
    )
    .map_err(|error| CaptureError::OperatingSystem(error.to_string()))?;
    drop_capabilities()?;
    let running = Arc::new(AtomicBool::new(true));
    let stdin_running = Arc::clone(&running);
    thread::spawn(move || {
        let mut byte = [0];
        let _ = std::io::stdin().read(&mut byte);
        stdin_running.store(false, Ordering::Relaxed);
    });
    let mut writer = std::io::BufWriter::new(std::io::stdout().lock());
    emit(
        &mut writer,
        &Frame::Ready {
            descriptor: linux_event_descriptor(1),
        },
    )?;
    let mut normalizer = Normalizer::new();
    let mut health = Instant::now();
    while running.load(Ordering::Relaxed) {
        while let Some(record) = ring.next() {
            if let Some(event) = normalizer.decode(&record)? {
                emit(
                    &mut writer,
                    &Frame::Event {
                        event: Box::new(event),
                    },
                )?;
            }
        }
        if health.elapsed() >= Duration::from_secs(1) {
            for event in normalizer.expire() {
                emit(
                    &mut writer,
                    &Frame::Event {
                        event: Box::new(event),
                    },
                )?;
            }
            let kernel = losses
                .get(&0, 0)
                .map_err(|error| CaptureError::OperatingSystem(error.to_string()))?;
            emit(
                &mut writer,
                &Frame::Health {
                    lost: kernel.saturating_add(normalizer.lost),
                },
            )?;
            health = Instant::now();
        }
        let mut descriptor = libc::pollfd {
            fd: ring.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut descriptor, 1, 100) } < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error.into());
            }
        }
    }
    Ok(())
}

fn drop_capabilities() -> Result<(), CaptureError> {
    #[repr(C)]
    struct Header {
        version: u32,
        pid: i32,
    }
    #[repr(C)]
    struct Data {
        effective: u32,
        permitted: u32,
        inheritable: u32,
    }
    let header = Header {
        version: 0x2008_0522,
        pid: 0,
    };
    let data = [
        Data {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        },
        Data {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        },
    ];
    if unsafe { libc::syscall(libc::SYS_capset, &header, data.as_ptr()) } != 0
        || unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

fn monotonic_ns() -> u64 {
    let mut clock = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut clock);
    }
    (clock.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(clock.tv_nsec as u64)
}

struct Normalizer {
    processes: ProcessResolver,
    owners: HashMap<(u32, u64), ProcessInfo>,
    udp: HashMap<ConnectionKey, (u64, NormalizedNetworkEvent)>,
    utc_offset_ms: i64,
    lost: u64,
}

impl Normalizer {
    fn new() -> Self {
        Self {
            processes: ProcessResolver::new(),
            owners: HashMap::new(),
            udp: HashMap::new(),
            utc_offset_ms: sabicheiro_core::now_ms()
                .saturating_sub((monotonic_ns() / 1_000_000) as i64),
            lost: 0,
        }
    }

    fn decode(&mut self, bytes: &[u8]) -> Result<Option<NormalizedNetworkEvent>, CaptureError> {
        if bytes.len() != 88 {
            return Err(CaptureError::InvalidData(
                "invalid kernel event size".into(),
            ));
        }
        let u64_at = |offset| {
            u64::from_ne_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .expect("checked frame size"),
            )
        };
        let u32_at = |offset| {
            u32::from_ne_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("checked frame size"),
            )
        };
        let u16_at = |offset| {
            u16::from_ne_bytes(
                bytes[offset..offset + 2]
                    .try_into()
                    .expect("checked frame size"),
            )
        };
        let observed = u64_at(0);
        let owner_ns = u64_at(8);
        let timestamp_ms = self
            .utc_offset_ms
            .saturating_add((observed / 1_000_000) as i64);
        let address = |offset| -> Result<IpAddr, CaptureError> {
            match bytes[32] {
                2 => Ok(Ipv4Addr::from(
                    <[u8; 4]>::try_from(&bytes[offset..offset + 4]).expect("checked frame size"),
                )
                .into()),
                10 => Ok(Ipv6Addr::from(
                    <[u8; 16]>::try_from(&bytes[offset..offset + 16]).expect("checked frame size"),
                )
                .into()),
                _ => Err(CaptureError::InvalidData(
                    "invalid kernel address family".into(),
                )),
            }
        };
        let local = Endpoint::new(address(36)?, u16_at(28));
        let remote = Endpoint::new(address(52)?, u16_at(30));
        // SYN_SENT can precede ephemeral-port assignment. Keep its kernel owner
        // metadata, but emit a connection only after its tuple is complete.
        if local.port == 0 || remote.address.is_unspecified() || remote.port == 0 {
            return Ok(None);
        }
        let protocol = match bytes[33] {
            6 => NetworkProtocol::Tcp,
            17 => NetworkProtocol::Udp,
            _ => return Err(CaptureError::InvalidData("invalid kernel protocol".into())),
        };
        let pid = (protocol == NetworkProtocol::Tcp && u32_at(16) > 0 && owner_ns > 0)
            .then(|| u32_at(16));
        let mut process = ProcessInfo::default();
        if let Some(pid) = pid {
            let owner = (pid, owner_ns);
            if let Some(cached) = self.owners.get(&owner) {
                process = cached.clone();
            } else {
                self.processes.refresh([pid]);
                process = self.processes.resolve_at(
                    Some(pid),
                    self.utc_offset_ms
                        .saturating_add((owner_ns / 1_000_000) as i64),
                );
                if process.started_at_ms.is_none() {
                    process.name = String::from_utf8_lossy(&bytes[68..84])
                        .trim_end_matches('\0')
                        .to_owned();
                }
                if self.owners.len() < MAX_FLOWS {
                    self.owners.insert(owner, process.clone());
                } else {
                    self.lost = self.lost.saturating_add(1);
                }
            }
            if matches!(bytes[34], 6 | 7) {
                self.owners.remove(&owner);
            }
        }
        let state = if protocol == NetworkProtocol::Udp || bytes[34] == 1 {
            ConnectionState::Established
        } else if matches!(bytes[34], 6 | 7) {
            ConnectionState::Closed
        } else if matches!(bytes[34], 4 | 5 | 8 | 9 | 11) {
            ConnectionState::Closing
        } else {
            ConnectionState::Connecting
        };
        let kind = if state == ConnectionState::Closed {
            NetworkEventKind::Closed
        } else if matches!(bytes[34], 1 | 2) {
            NetworkEventKind::Opened
        } else {
            NetworkEventKind::Updated
        };
        let mut key = ConnectionKey::new(pid, local, remote.clone(), protocol);
        key.process_instance = process.instance_id();
        if protocol == NetworkProtocol::Tcp {
            key.flow_generation = owner_ns;
        }
        let mut evidence = ConnectionEvidence::for_peer(
            &remote,
            state,
            ObservationSource::LinuxEbpf,
            timestamp_ms,
        );
        evidence.correlation_quality = if key.process_instance.is_some() {
            CorrelationQuality::Confirmed
        } else {
            CorrelationQuality::Unknown
        };
        let incoming = bytes[35] != 0;
        let mut event = NormalizedNetworkEvent {
            kind,
            key,
            evidence,
            timestamp_ms,
            process,
            direction: if protocol == NetworkProtocol::Tcp && pid.is_none() {
                Direction::Unknown
            } else if incoming {
                Direction::Inbound
            } else {
                Direction::Outbound
            },
            state,
            interface: (u32_at(24) > 0).then(|| format!("ifindex:{}", u32_at(24))),
            hostname: None,
            domain: None,
            geo: None,
            asn: None,
            traffic_delta: TrafficCounters::default(),
            counter_source: CounterSource::Unavailable,
        };
        if protocol == NetworkProtocol::Udp {
            let tuple = event.key.clone();
            event.counter_source = CounterSource::EbpfFlow;
            if incoming {
                event.traffic_delta.bytes_received = u64::from(u32_at(20));
                event.traffic_delta.packets_received = 1;
            } else {
                event.traffic_delta.bytes_sent = u64::from(u32_at(20));
                event.traffic_delta.packets_sent = 1;
            }
            if let Some((_, previous)) = self.udp.get(&event.key) {
                event.direction = previous.direction;
                event.key.flow_generation = previous.key.flow_generation;
            } else {
                if self.udp.len() >= MAX_FLOWS {
                    self.lost = self.lost.saturating_add(1);
                    return Ok(None);
                }
                event.kind = NetworkEventKind::Opened;
                event.key.flow_generation = observed.max(1);
            }
            self.udp.insert(tuple, (observed, event.clone()));
        }
        Ok(Some(event))
    }

    fn expire(&mut self) -> Vec<NormalizedNetworkEvent> {
        let now = monotonic_ns();
        let mut closed = Vec::new();
        self.udp.retain(|_, (seen, event)| {
            if now.saturating_sub(*seen) < 30_000_000_000 {
                return true;
            }
            let mut event = event.clone();
            event.kind = NetworkEventKind::Closed;
            event.state = ConnectionState::Closed;
            event.timestamp_ms = self.utc_offset_ms.saturating_add((now / 1_000_000) as i64);
            event.traffic_delta = TrafficCounters::default();
            closed.push(event);
            false
        });
        closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(protocol: u8, family: u8, state: u8) -> [u8; 88] {
        let mut bytes = [0; 88];
        bytes[..8].copy_from_slice(&monotonic_ns().to_ne_bytes());
        bytes[28..30].copy_from_slice(&42001_u16.to_ne_bytes());
        bytes[30..32].copy_from_slice(&443_u16.to_ne_bytes());
        bytes[32] = family;
        bytes[33] = protocol;
        bytes[34] = state;
        if family == 2 {
            bytes[36..40].copy_from_slice(&[127, 0, 0, 1]);
            bytes[52..56].copy_from_slice(&[127, 0, 0, 2]);
        } else {
            bytes[51] = 1;
            bytes[67] = 2;
        }
        bytes
    }

    #[test]
    fn udp_counters_and_peer_survive_updates_without_inventing_an_owner() {
        let mut normalizer = Normalizer::new();
        let mut bytes = record(17, 10, 0);
        bytes[16..20].copy_from_slice(&std::process::id().to_ne_bytes());
        bytes[20..24].copy_from_slice(&8_u32.to_ne_bytes());
        let first = normalizer.decode(&bytes).unwrap().unwrap();
        assert_eq!(first.kind, NetworkEventKind::Opened);
        assert_eq!(first.key.local.address, "::1".parse::<IpAddr>().unwrap());
        assert!(first.key.pid.is_none());
        assert!(first.process.instance_id().is_none());
        assert_eq!(
            first.evidence.correlation_quality,
            CorrelationQuality::Unknown
        );
        assert_eq!(first.traffic_delta.bytes_sent, 8);
        assert_eq!(first.traffic_delta.packets_sent, 1);
        bytes[35] = 1;
        let update = normalizer.decode(&bytes).unwrap().unwrap();
        assert_eq!(update.key, first.key);
        assert_eq!(update.kind, NetworkEventKind::Updated);
        assert_eq!(update.direction, Direction::Outbound);
        assert_eq!(update.traffic_delta.bytes_received, 8);
        assert_eq!(normalizer.udp.len(), 1);
    }

    #[test]
    fn malformed_and_incomplete_records_do_not_create_connections() {
        let mut normalizer = Normalizer::new();
        assert!(normalizer.decode(&[0; 87]).is_err());
        let mut bytes = record(6, 2, 2);
        bytes[28..30].fill(0);
        assert!(normalizer.decode(&bytes).unwrap().is_none());
        let bytes = record(6, 99, 1);
        assert!(normalizer.decode(&bytes).is_err());
    }

    #[test]
    fn tcp_close_retains_the_process_that_terminated_after_open() {
        let mut child = Command::new("sleep").arg("0.15").spawn().unwrap();
        let mut normalizer = Normalizer::new();
        let mut bytes = record(6, 2, 1);
        bytes[8..16].copy_from_slice(&monotonic_ns().to_ne_bytes());
        bytes[16..20].copy_from_slice(&child.id().to_ne_bytes());
        let open = normalizer.decode(&bytes).unwrap().unwrap();
        assert!(open.process.instance_id().is_some());
        child.wait().unwrap();
        bytes[34] = 7;
        let closed = normalizer.decode(&bytes).unwrap().unwrap();
        assert_eq!(closed.process, open.process);
        assert_eq!(closed.key, open.key);
        assert_eq!(closed.kind, NetworkEventKind::Closed);
        assert!(normalizer.owners.is_empty());
    }

    #[test]
    fn udp_inactivity_closes_once_without_repeating_traffic() {
        let mut normalizer = Normalizer::new();
        let mut bytes = record(17, 2, 0);
        bytes[..8].copy_from_slice(&monotonic_ns().saturating_sub(31_000_000_000).to_ne_bytes());
        let open = normalizer.decode(&bytes).unwrap().unwrap();
        let closed = normalizer.expire();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].key, open.key);
        assert_eq!(closed[0].traffic_delta, TrafficCounters::default());
        assert!(normalizer.expire().is_empty());
    }

    #[test]
    #[ignore = "requires the installed capability-enabled helper and mounted tracefs"]
    fn installed_helper_drops_privileges_and_reports_termination() {
        let mut capture = LinuxEventCapture::start().unwrap();
        let status = fs::read_to_string(format!("/proc/{}/status", capture.child.id())).unwrap();
        for field in ["CapEff:", "CapPrm:", "CapInh:"] {
            assert!(
                status
                    .lines()
                    .any(|line| line.starts_with(field) && line.ends_with("0000000000000000"))
            );
        }
        assert!(status.lines().any(|line| line == "NoNewPrivs:\t1"));
        capture.child.kill().unwrap();
        capture.child.wait().unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while capture.is_alive() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(!capture.is_alive());
        assert!(
            capture
                .descriptor(1)
                .capabilities
                .iter()
                .all(|capability| capability.availability == CapabilityAvailability::Unavailable)
        );
    }
}
