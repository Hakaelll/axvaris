use sabicheiro_core::{
    Endpoint, EvidenceObservation, NetworkEventKind, NetworkProtocol, ObservationKind,
    ProviderDescriptor,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener, TcpStream, UdpSocket},
    thread,
    time::{Duration, Instant},
};

pub const FIXTURE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureFlowKind {
    TcpConnection,
    TcpListener,
    UdpConnected,
    UdpUnconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureFlow {
    pub id: String,
    pub kind: FixtureFlowKind,
    pub process_id: u32,
    pub process_instance: String,
    pub protocol: NetworkProtocol,
    pub local: Endpoint,
    pub remote: Option<Endpoint>,
    pub interface_id: Option<String>,
    pub opened_monotonic_ns: u64,
    pub activity_monotonic_ns: Vec<u64>,
    pub closed_monotonic_ns: Option<u64>,
    pub duration_ms: u64,
    pub expected_remote_destination: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FixturePerturbation {
    Burst { flows: usize },
    ProviderDrop { events: u64 },
    Reorder { window_events: usize },
    SleepResume { duration_ms: u64 },
    InterfaceChange { from: String, to: String },
    PidReuse { pid: u32 },
    PortReuse { port: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureManifest {
    pub schema_version: u16,
    pub fixture_id: String,
    pub generated_at_ms: i64,
    pub operating_system: String,
    pub architecture: String,
    pub network_scope: String,
    pub flows: Vec<FixtureFlow>,
    pub perturbations: Vec<FixturePerturbation>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DurationBucketReport {
    pub expected: usize,
    pub observed: usize,
    pub recall: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CaptureHarnessReport {
    pub fixture_id: String,
    pub provider_id: String,
    pub expected_remote_flows: usize,
    pub observed_remote_flows: usize,
    pub true_positive_flows: usize,
    pub false_positive_flows: usize,
    pub unknown_process_attributions: usize,
    pub incorrect_process_attributions: usize,
    pub precision: f64,
    pub process_attribution_precision: f64,
    pub recall_5_to_50_ms: f64,
    pub recall_50_ms_or_longer: f64,
    pub close_latency_p95_ms: Option<f64>,
    pub detected_provider_gaps: usize,
    pub unknown_fields: BTreeMap<String, usize>,
    pub duration_buckets: BTreeMap<String, DurationBucketReport>,
    pub meets_release_targets: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderHarnessRun {
    pub manifest: FixtureManifest,
    pub provider: ProviderDescriptor,
    pub poll_interval_ms: u64,
    pub observations: Vec<EvidenceObservation>,
    pub report: CaptureHarnessReport,
}

#[derive(Debug, Clone)]
struct MatchedFlow {
    observed: bool,
    close_latency_ms: Option<f64>,
    process_unknown: bool,
    process_incorrect: bool,
}

pub fn compare_observations(
    manifest: &FixtureManifest,
    provider_id: impl Into<String>,
    observations: &[EvidenceObservation],
) -> CaptureHarnessReport {
    let provider_id = provider_id.into();
    let expected = manifest
        .flows
        .iter()
        .filter(|flow| flow.expected_remote_destination)
        .collect::<Vec<_>>();
    let network = observations
        .iter()
        .filter_map(|observation| {
            observation
                .network
                .as_ref()
                .map(|event| (observation, event))
        })
        .collect::<Vec<_>>();
    let mut matched_observation_ids = BTreeSet::new();
    let mut matches = Vec::with_capacity(expected.len());
    let mut unknown_fields = BTreeMap::new();

    for flow in &expected {
        let candidates = network
            .iter()
            .filter(|(_, event)| tuple_matches(flow, event))
            .collect::<Vec<_>>();
        let first = candidates
            .iter()
            .filter(|(_, event)| event.kind != NetworkEventKind::Closed)
            .min_by_key(|(observation, _)| observation.clock.monotonic_ns);
        let closed = candidates
            .iter()
            .filter(|(_, event)| event.kind == NetworkEventKind::Closed)
            .min_by_key(|(observation, _)| observation.clock.monotonic_ns);
        for (observation, _) in &candidates {
            matched_observation_ids.insert(observation.id.clone());
        }
        let (process_unknown, process_incorrect) = first.map_or((false, false), |(_, event)| {
            let instance_unknown = event
                .key
                .process_instance
                .as_deref()
                .is_none_or(|instance| instance.ends_with(":start:unknown"));
            let unknown = event.key.pid.is_none() || instance_unknown;
            let incorrect = event.key.pid.is_some_and(|pid| pid != flow.process_id)
                || event.key.process_instance.as_ref().is_some_and(|instance| {
                    !instance.ends_with(":start:unknown") && instance != &flow.process_instance
                });
            (unknown, incorrect)
        });
        if first.is_some_and(|(_, event)| event.interface.is_none()) {
            *unknown_fields.entry("interface".into()).or_default() += 1;
        }
        let close_latency_ms = flow.closed_monotonic_ns.and_then(|expected_close| {
            closed.map(|(observation, _)| {
                observation
                    .clock
                    .monotonic_ns
                    .saturating_sub(expected_close) as f64
                    / 1_000_000.0
            })
        });
        matches.push(MatchedFlow {
            observed: first.is_some(),
            close_latency_ms,
            process_unknown,
            process_incorrect,
        });
    }

    let false_positive_flows = network
        .iter()
        .filter(|(observation, event)| {
            event.kind != NetworkEventKind::Closed
                && !matched_observation_ids.contains(&observation.id)
                && !event.key.remote.address.is_unspecified()
                && event.key.remote.port != 0
        })
        .map(|(_, event)| {
            format!(
                "{}|{}|{}|{}|{}",
                event.key.pid.unwrap_or_default(),
                event.key.protocol.as_str(),
                event.key.local.label(),
                event.key.remote.label(),
                event.key.flow_generation
            )
        })
        .collect::<HashSet<_>>()
        .len();
    let true_positive_flows = matches.iter().filter(|item| item.observed).count();
    let observed_remote_flows = true_positive_flows.saturating_add(false_positive_flows);
    let precision = ratio(true_positive_flows, observed_remote_flows);
    let attributed = matches
        .iter()
        .filter(|item| item.observed && !item.process_unknown)
        .count();
    let incorrect_process_attributions =
        matches.iter().filter(|item| item.process_incorrect).count();
    let unknown_process_attributions = matches
        .iter()
        .filter(|item| item.observed && item.process_unknown)
        .count();
    let correct_attributions = attributed.saturating_sub(incorrect_process_attributions);
    let process_attribution_precision = ratio(correct_attributions, attributed);
    let short_indices = expected
        .iter()
        .enumerate()
        .filter(|(_, flow)| (5..50).contains(&flow.duration_ms))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let long_indices = expected
        .iter()
        .enumerate()
        .filter(|(_, flow)| flow.duration_ms >= 50)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let recall_5_to_50_ms = recall(&matches, &short_indices);
    let recall_50_ms_or_longer = recall(&matches, &long_indices);
    let mut close_latencies = matches
        .iter()
        .filter_map(|item| item.close_latency_ms)
        .collect::<Vec<_>>();
    close_latencies.sort_by(f64::total_cmp);
    let close_latency_p95_ms = percentile(&close_latencies, 0.95);
    let detected_provider_gaps = observations
        .iter()
        .filter(|observation| observation.kind == ObservationKind::ProviderGap)
        .count();
    let duration_buckets = duration_buckets(&expected, &matches);
    let meets_release_targets = false_positive_flows == 0
        && incorrect_process_attributions == 0
        && precision >= 0.99
        && recall_5_to_50_ms >= 0.95
        && recall_50_ms_or_longer >= 0.99
        && close_latency_p95_ms.is_some_and(|latency| latency <= 1_000.0);

    CaptureHarnessReport {
        fixture_id: manifest.fixture_id.clone(),
        provider_id,
        expected_remote_flows: expected.len(),
        observed_remote_flows,
        true_positive_flows,
        false_positive_flows,
        unknown_process_attributions,
        incorrect_process_attributions,
        precision,
        process_attribution_precision,
        recall_5_to_50_ms,
        recall_50_ms_or_longer,
        close_latency_p95_ms,
        detected_provider_gaps,
        unknown_fields,
        duration_buckets,
        meets_release_targets,
    }
}

fn tuple_matches(flow: &FixtureFlow, event: &sabicheiro_core::NormalizedNetworkEvent) -> bool {
    let Some(remote) = flow.remote.as_ref() else {
        return false;
    };
    event.key.protocol == flow.protocol
        && event.key.local == flow.local
        && &event.key.remote == remote
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn recall(matches: &[MatchedFlow], indices: &[usize]) -> f64 {
    ratio(
        indices
            .iter()
            .filter(|index| matches[**index].observed)
            .count(),
        indices.len(),
    )
}

fn percentile(sorted: &[f64], percentile: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted.get(index).copied()
}

fn duration_buckets(
    expected: &[&FixtureFlow],
    matches: &[MatchedFlow],
) -> BTreeMap<String, DurationBucketReport> {
    let ranges = [
        ("5-19ms", 5, 20),
        ("20-49ms", 20, 50),
        ("50-99ms", 50, 100),
        ("100-999ms", 100, 1_000),
        ("1s+", 1_000, u64::MAX),
    ];
    ranges
        .into_iter()
        .map(|(label, minimum, maximum)| {
            let indices = expected
                .iter()
                .enumerate()
                .filter(|(_, flow)| flow.duration_ms >= minimum && flow.duration_ms < maximum)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let observed = indices
                .iter()
                .filter(|index| matches[**index].observed)
                .count();
            (
                label.into(),
                DurationBucketReport {
                    expected: indices.len(),
                    observed,
                    recall: ratio(observed, indices.len()),
                },
            )
        })
        .collect()
}

pub fn reference_manifest() -> FixtureManifest {
    let pid = 4_242;
    let process_instance = "fixture-process:4242:generation:1".to_string();
    let mut port = 41_000_u16;
    let mut flows = Vec::new();
    for family in ["ipv4", "ipv6"] {
        for protocol in [NetworkProtocol::Tcp, NetworkProtocol::Udp] {
            for duration_ms in [5_u64, 20, 50, 100, 1_000] {
                port = port.saturating_add(1);
                let (local_ip, remote_ip): (IpAddr, IpAddr) = if family == "ipv4" {
                    (Ipv4Addr::LOCALHOST.into(), Ipv4Addr::LOCALHOST.into())
                } else {
                    (Ipv6Addr::LOCALHOST.into(), Ipv6Addr::LOCALHOST.into())
                };
                let opened = (flows.len() as u64 + 1).saturating_mul(2_000_000_000);
                let flow = FixtureFlow {
                    id: format!("{family}-{}-{duration_ms}", protocol.as_str()),
                    kind: if protocol == NetworkProtocol::Tcp {
                        FixtureFlowKind::TcpConnection
                    } else {
                        FixtureFlowKind::UdpConnected
                    },
                    process_id: pid,
                    process_instance: process_instance.clone(),
                    protocol,
                    local: Endpoint::new(local_ip, port),
                    remote: Some(Endpoint::new(remote_ip, port.saturating_add(1_000))),
                    interface_id: Some("loopback".into()),
                    opened_monotonic_ns: opened,
                    activity_monotonic_ns: vec![opened.saturating_add(1_000_000)],
                    closed_monotonic_ns: Some(
                        opened.saturating_add(duration_ms.saturating_mul(1_000_000)),
                    ),
                    duration_ms,
                    expected_remote_destination: true,
                };
                if protocol == NetworkProtocol::Tcp {
                    let mut accepted = flow.clone();
                    accepted.id.push_str("-accept");
                    accepted.local = flow.remote.clone().expect("reference remote");
                    accepted.remote = Some(flow.local.clone());
                    flows.push(accepted);
                }
                flows.push(flow);
            }
        }
    }
    for (id, kind, protocol, local, remote) in [
        (
            "tcp-wildcard-listener",
            FixtureFlowKind::TcpListener,
            NetworkProtocol::Tcp,
            Endpoint::new(Ipv4Addr::UNSPECIFIED.into(), 45_000),
            None,
        ),
        (
            "udp-bound-no-peer",
            FixtureFlowKind::UdpUnconnected,
            NetworkProtocol::Udp,
            Endpoint::new(Ipv6Addr::UNSPECIFIED.into(), 45_001),
            None,
        ),
    ] {
        flows.push(FixtureFlow {
            id: id.into(),
            kind,
            process_id: pid,
            process_instance: process_instance.clone(),
            protocol,
            local,
            remote,
            interface_id: None,
            opened_monotonic_ns: 1,
            activity_monotonic_ns: Vec::new(),
            closed_monotonic_ns: Some(2),
            duration_ms: 1,
            expected_remote_destination: false,
        });
    }
    FixtureManifest {
        schema_version: FIXTURE_SCHEMA_VERSION,
        fixture_id: "sabicheiro-reference-v1".into(),
        generated_at_ms: 0,
        operating_system: "deterministic".into(),
        architecture: "portable".into(),
        network_scope: "loopback-only".into(),
        flows,
        perturbations: vec![
            FixturePerturbation::Burst { flows: 1_000 },
            FixturePerturbation::ProviderDrop { events: 7 },
            FixturePerturbation::Reorder { window_events: 16 },
            FixturePerturbation::SleepResume { duration_ms: 2_000 },
            FixturePerturbation::InterfaceChange {
                from: "loopback-a".into(),
                to: "loopback-b".into(),
            },
            FixturePerturbation::PidReuse { pid },
            FixturePerturbation::PortReuse { port: 41_001 },
        ],
        limitations: Vec::new(),
    }
}

/// Executes real local sockets and records only truth. A capture provider can run in
/// parallel and its journal can then be compared with [`compare_observations`].
pub fn run_loopback_truth_suite() -> std::io::Result<FixtureManifest> {
    let origin = Instant::now();
    let generated_at_ms = sabicheiro_core::now_ms();
    let pid = std::process::id();
    let process_instance = current_process_instance(pid);
    run_loopback_truth_suite_with_origin(origin, generated_at_ms, pid, process_instance)
}

pub fn run_system_provider_fixture(poll_interval: Duration) -> Result<ProviderHarnessRun, String> {
    let poll_interval = poll_interval.clamp(Duration::from_millis(1), Duration::from_secs(1));
    let mut provider = crate::system_provider().map_err(|error| error.to_string())?;
    let descriptor = provider.descriptor(1);
    let generated_at_ms = sabicheiro_core::now_ms();
    let origin = Instant::now();
    let pid = std::process::id();
    let process_instance = current_process_instance(pid);
    let fixture_origin = origin;
    let fixture = thread::Builder::new()
        .name("sabicheiro-fixture-truth".into())
        .spawn(move || {
            run_loopback_truth_suite_with_origin(
                fixture_origin,
                generated_at_ms,
                pid,
                process_instance,
            )
        })
        .map_err(|error| error.to_string())?;
    let mut differ = crate::SnapshotDiffer::new(descriptor.generation);
    let mut sequencer = crate::EvidenceSequencer::new(descriptor.clone(), generated_at_ms);
    let mut observations = Vec::new();
    while !fixture.is_finished() {
        capture_snapshot(
            provider.as_mut(),
            pid,
            &origin,
            generated_at_ms,
            &mut differ,
            &mut sequencer,
            &mut observations,
        )?;
        thread::sleep(poll_interval);
    }
    let manifest = fixture
        .join()
        .map_err(|_| "fixture truth worker panicked".to_string())?
        .map_err(|error| error.to_string())?;
    // Give native tables one bounded interval to expose final closes, then reconcile.
    thread::sleep(poll_interval);
    capture_snapshot(
        provider.as_mut(),
        pid,
        &origin,
        generated_at_ms,
        &mut differ,
        &mut sequencer,
        &mut observations,
    )?;
    let monotonic_ns = elapsed_ns(&origin);
    let utc_ms = generated_at_ms.saturating_add((monotonic_ns / 1_000_000) as i64);
    for event in differ.clear(utc_ms) {
        observations.push(sequencer.wrap_network_at(monotonic_ns, utc_ms, event));
    }
    let report = compare_observations(&manifest, &descriptor.provider_id, &observations);
    Ok(ProviderHarnessRun {
        manifest,
        provider: descriptor,
        poll_interval_ms: poll_interval.as_millis() as u64,
        observations,
        report,
    })
}

#[cfg(all(target_os = "linux", feature = "linux-ebpf"))]
pub fn run_linux_event_fixture(poll_interval: Duration) -> Result<ProviderHarnessRun, String> {
    let interval = poll_interval.clamp(Duration::from_millis(1), Duration::from_secs(1));
    let mut capture = crate::LinuxEventCapture::start().map_err(|error| error.to_string())?;
    let descriptor = capture.descriptor(1);
    let origin = Instant::now();
    let utc_ms = sabicheiro_core::now_ms();
    let pid = std::process::id();
    let instance = current_process_instance(pid);
    let fixture =
        thread::spawn(move || run_loopback_truth_suite_with_origin(origin, utc_ms, pid, instance));
    let mut sequencer = crate::EvidenceSequencer::new(descriptor.clone(), utc_ms);
    let mut observations = Vec::new();
    let mut last_dropped = 0;
    let mut collect = |capture: &mut crate::LinuxEventCapture| {
        let (events, dropped) = capture.drain();
        for event in events.into_iter().filter(|event| {
            event.key.pid.is_none_or(|owner| owner == pid)
                && event.key.local.address.is_loopback()
                && event.key.remote.address.is_loopback()
                // The truth suite defines originating UDP sockets, not the
                // receiving socket's inferred inbound conversation.
                && !(event.key.protocol == NetworkProtocol::Udp && event.direction == sabicheiro_core::Direction::Inbound)
        }) {
            let at = event.timestamp_ms;
            observations.push(sequencer.wrap_network_at(elapsed_ns(&origin), at, event));
        }
        if let Some(gap) = sequencer.record_gap(
            dropped.saturating_sub(last_dropped),
            elapsed_ns(&origin),
            sabicheiro_core::now_ms(),
            [sabicheiro_core::CaptureCapabilityKind::LifecycleEvents],
            "Linux event fixture dropped records",
        ) {
            observations.push(gap);
        }
        last_dropped = dropped;
    };
    while !fixture.is_finished() {
        collect(&mut capture);
        thread::sleep(interval);
    }
    let manifest = fixture
        .join()
        .map_err(|_| "fixture thread panicked".to_string())?
        .map_err(|error| error.to_string())?;
    thread::sleep(Duration::from_millis(100));
    collect(&mut capture);
    let report = compare_observations(&manifest, &descriptor.provider_id, &observations);
    Ok(ProviderHarnessRun {
        manifest,
        provider: descriptor,
        poll_interval_ms: interval.as_millis() as u64,
        observations,
        report,
    })
}

#[cfg(not(all(target_os = "linux", feature = "linux-ebpf")))]
pub fn run_linux_event_fixture(_poll_interval: Duration) -> Result<ProviderHarnessRun, String> {
    Err("Linux eBPF fixture requires Linux with the linux-ebpf feature".into())
}

#[cfg(target_os = "windows")]
pub fn run_windows_event_fixture(poll_interval: Duration) -> Result<ProviderHarnessRun, String> {
    use sabicheiro_core::{
        CapabilityAvailability, CaptureCapabilityKind, ObservationSource, ProviderCapability,
        ProviderMode,
    };

    let poll_interval = poll_interval.clamp(Duration::from_millis(1), Duration::from_secs(1));
    let mut snapshot_provider = crate::system_provider().map_err(|error| error.to_string())?;
    let mut event_provider =
        crate::WindowsEventCapture::start().map_err(|error| error.to_string())?;
    let descriptor = ProviderDescriptor {
        protocol_version: 1,
        provider_id: "windows.etw".into(),
        display_name: "Windows Kernel Network + DNS Client ETW".into(),
        mode: ProviderMode::Hybrid,
        generation: 1,
        capabilities: vec![
            ProviderCapability {
                kind: CaptureCapabilityKind::LifecycleEvents,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::WindowsEtw,
                limitation: None,
            },
            ProviderCapability {
                kind: CaptureCapabilityKind::FlowBytes,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::WindowsEtw,
                limitation: None,
            },
            ProviderCapability {
                kind: CaptureCapabilityKind::FlowPackets,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::WindowsEtw,
                limitation: None,
            },
            ProviderCapability {
                kind: CaptureCapabilityKind::Dns,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::WindowsEtw,
                limitation: Some("process identity depends on the Windows event schema".into()),
            },
            ProviderCapability {
                kind: CaptureCapabilityKind::ProcessIdentity,
                availability: CapabilityAvailability::Degraded,
                source: ObservationSource::WindowsEtw,
                limitation: Some(
                    "snapshot correlation is required for a stable process instance".into(),
                ),
            },
            ProviderCapability {
                kind: CaptureCapabilityKind::Interface,
                availability: CapabilityAvailability::Degraded,
                source: ObservationSource::WindowsIpHelper,
                limitation: Some("interface is correlated from the socket snapshot".into()),
            },
        ],
    };
    let generated_at_ms = sabicheiro_core::now_ms();
    let origin = Instant::now();
    let pid = std::process::id();
    let process_instance = current_process_instance(pid);
    let fixture_origin = origin;
    let fixture = thread::Builder::new()
        .name("sabicheiro-fixture-etw-truth".into())
        .spawn(move || {
            run_loopback_truth_suite_with_origin(
                fixture_origin,
                generated_at_ms,
                pid,
                process_instance,
            )
        })
        .map_err(|error| error.to_string())?;
    let mut differ = crate::SnapshotDiffer::new(1);
    let mut correlator = crate::FlowCorrelator::default();
    let mut sequencer = crate::EvidenceSequencer::new(descriptor.clone(), generated_at_ms);
    let mut observations = Vec::new();
    let mut previous_dropped = 0_u64;
    while !fixture.is_finished() {
        capture_windows_events(
            snapshot_provider.as_mut(),
            &mut event_provider,
            pid,
            &origin,
            generated_at_ms,
            &mut differ,
            &mut correlator,
            &mut sequencer,
            &mut observations,
            &mut previous_dropped,
        )?;
        thread::sleep(poll_interval);
    }
    let manifest = fixture
        .join()
        .map_err(|_| "ETW fixture truth worker panicked".to_string())?
        .map_err(|error| error.to_string())?;
    for _ in 0..3 {
        thread::sleep(poll_interval);
        capture_windows_events(
            snapshot_provider.as_mut(),
            &mut event_provider,
            pid,
            &origin,
            generated_at_ms,
            &mut differ,
            &mut correlator,
            &mut sequencer,
            &mut observations,
            &mut previous_dropped,
        )?;
    }
    let report = compare_observations(&manifest, &descriptor.provider_id, &observations);
    Ok(ProviderHarnessRun {
        manifest,
        provider: descriptor,
        poll_interval_ms: poll_interval.as_millis() as u64,
        observations,
        report,
    })
}

#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn capture_windows_events(
    snapshot_provider: &mut dyn crate::NetworkCaptureProvider,
    event_provider: &mut crate::WindowsEventCapture,
    pid: u32,
    origin: &Instant,
    utc_origin_ms: i64,
    differ: &mut crate::SnapshotDiffer,
    correlator: &mut crate::FlowCorrelator,
    sequencer: &mut crate::EvidenceSequencer,
    observations: &mut Vec<EvidenceObservation>,
    previous_dropped: &mut u64,
) -> Result<(), String> {
    let monotonic_ns = elapsed_ns(origin);
    let utc_ms = utc_origin_ms.saturating_add((monotonic_ns / 1_000_000) as i64);
    let sockets = snapshot_provider
        .snapshot()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|socket| {
            socket.identity.pid == Some(pid)
                && (socket.identity.local.address.is_unspecified()
                    || socket.identity.local.address.is_loopback())
                && socket.identity.remote.as_ref().is_none_or(|remote| {
                    remote.address.is_unspecified() || remote.address.is_loopback()
                })
        })
        .collect::<Vec<_>>();
    let snapshots = differ.diff(sockets, utc_ms);
    correlator.observe_snapshots(snapshots.iter(), utc_ms);
    let (mut events, _) = event_provider.drain(utc_ms);
    for event in &mut events {
        correlator.correlate(event);
    }
    observations.extend(
        events
            .into_iter()
            .filter(|event| {
                event.key.pid == Some(pid)
                    && event.key.local.address.is_loopback()
                    && event.key.remote.address.is_loopback()
            })
            .map(|event| sequencer.wrap_network_at(monotonic_ns, utc_ms, event)),
    );
    let dropped = event_provider.dropped();
    let newly_dropped = dropped.saturating_sub(*previous_dropped);
    if let Some(gap) = sequencer.record_gap(
        newly_dropped,
        monotonic_ns,
        utc_ms,
        [
            sabicheiro_core::CaptureCapabilityKind::LifecycleEvents,
            sabicheiro_core::CaptureCapabilityKind::FlowBytes,
            sabicheiro_core::CaptureCapabilityKind::FlowPackets,
            sabicheiro_core::CaptureCapabilityKind::Dns,
        ],
        "ETW callback queue overflow",
    ) {
        observations.push(gap);
    }
    *previous_dropped = dropped;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn run_windows_event_fixture(_poll_interval: Duration) -> Result<ProviderHarnessRun, String> {
    Err("Windows ETW fixture is only available on Windows".into())
}

fn capture_snapshot(
    provider: &mut dyn crate::NetworkCaptureProvider,
    pid: u32,
    origin: &Instant,
    utc_origin_ms: i64,
    differ: &mut crate::SnapshotDiffer,
    sequencer: &mut crate::EvidenceSequencer,
    observations: &mut Vec<EvidenceObservation>,
) -> Result<(), String> {
    let sockets = provider
        .snapshot()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|socket| {
            socket.identity.pid == Some(pid)
                && (socket.identity.local.address.is_unspecified()
                    || socket.identity.local.address.is_loopback())
                && socket.identity.remote.as_ref().is_none_or(|remote| {
                    remote.address.is_unspecified() || remote.address.is_loopback()
                })
        })
        .collect::<Vec<_>>();
    let monotonic_ns = elapsed_ns(origin);
    let utc_ms = utc_origin_ms.saturating_add((monotonic_ns / 1_000_000) as i64);
    for event in differ.diff(sockets, utc_ms) {
        observations.push(sequencer.wrap_network_at(monotonic_ns, utc_ms, event));
    }
    Ok(())
}

fn current_process_instance(pid: u32) -> String {
    let mut resolver = crate::process::ProcessResolver::new();
    resolver.refresh([pid]);
    resolver
        .resolve(Some(pid))
        .instance_id()
        .unwrap_or_else(|| format!("pid:{pid}:start:unknown"))
}

fn run_loopback_truth_suite_with_origin(
    origin: Instant,
    generated_at_ms: i64,
    pid: u32,
    process_instance: String,
) -> std::io::Result<FixtureManifest> {
    let mut flows = Vec::new();
    let mut limitations = Vec::new();
    for duration_ms in [5_u64, 20, 50, 100, 1_000] {
        flows.extend(run_tcp_flows(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            duration_ms,
            pid,
            &process_instance,
            &origin,
        )?);
        flows.push(run_udp_flow(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            duration_ms,
            pid,
            &process_instance,
            &origin,
        )?);
    }
    match run_tcp_flows(
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        50,
        pid,
        &process_instance,
        &origin,
    ) {
        Ok(tcp_flows) => flows.extend(tcp_flows),
        Err(error) => limitations.push(format!("IPv6 TCP loopback unavailable: {error}")),
    }
    match run_udp_flow(
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        50,
        pid,
        &process_instance,
        &origin,
    ) {
        Ok(flow) => flows.push(flow),
        Err(error) => limitations.push(format!("IPv6 UDP loopback unavailable: {error}")),
    }
    let wildcard = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let listener_address = wildcard.local_addr()?;
    let opened = elapsed_ns(&origin);
    flows.push(FixtureFlow {
        id: "tcp-wildcard-listener".into(),
        kind: FixtureFlowKind::TcpListener,
        process_id: pid,
        process_instance: process_instance.clone(),
        protocol: NetworkProtocol::Tcp,
        local: Endpoint::new(listener_address.ip(), listener_address.port()),
        remote: None,
        interface_id: None,
        opened_monotonic_ns: opened,
        activity_monotonic_ns: Vec::new(),
        closed_monotonic_ns: Some(elapsed_ns(&origin)),
        duration_ms: 0,
        expected_remote_destination: false,
    });
    drop(wildcard);
    let unconnected = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let bound_address = unconnected.local_addr()?;
    let opened = elapsed_ns(&origin);
    flows.push(FixtureFlow {
        id: "udp-bound-no-peer".into(),
        kind: FixtureFlowKind::UdpUnconnected,
        process_id: pid,
        process_instance,
        protocol: NetworkProtocol::Udp,
        local: Endpoint::new(bound_address.ip(), bound_address.port()),
        remote: None,
        interface_id: None,
        opened_monotonic_ns: opened,
        activity_monotonic_ns: Vec::new(),
        closed_monotonic_ns: Some(elapsed_ns(&origin)),
        duration_ms: 0,
        expected_remote_destination: false,
    });
    drop(unconnected);

    Ok(FixtureManifest {
        schema_version: FIXTURE_SCHEMA_VERSION,
        fixture_id: format!("local-{generated_at_ms}"),
        generated_at_ms,
        operating_system: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        network_scope: "loopback-only".into(),
        flows,
        perturbations: vec![
            FixturePerturbation::Burst { flows: 1_000 },
            FixturePerturbation::ProviderDrop { events: 7 },
            FixturePerturbation::Reorder { window_events: 16 },
            FixturePerturbation::SleepResume { duration_ms: 2_000 },
            FixturePerturbation::InterfaceChange {
                from: "fixture-route-a".into(),
                to: "fixture-route-b".into(),
            },
            FixturePerturbation::PidReuse { pid },
            FixturePerturbation::PortReuse { port: 0 },
        ],
        limitations,
    })
}

fn run_tcp_flows(
    ip: IpAddr,
    duration_ms: u64,
    pid: u32,
    process_instance: &str,
    origin: &Instant,
) -> std::io::Result<Vec<FixtureFlow>> {
    let listener = TcpListener::bind((ip, 0))?;
    let remote_address = listener.local_addr()?;
    let server = thread::spawn(
        move || -> std::io::Result<(std::net::SocketAddr, std::net::SocketAddr)> {
            let (mut stream, _) = listener.accept()?;
            let local = stream.local_addr()?;
            let peer = stream.peer_addr()?;
            let mut byte = [0_u8; 1];
            stream.read_exact(&mut byte)?;
            stream.write_all(&byte)?;
            Ok((local, peer))
        },
    );
    let opened = elapsed_ns(origin);
    let mut client = TcpStream::connect(remote_address)?;
    let local_address = client.local_addr()?;
    client.write_all(&[0x5a])?;
    let activity = elapsed_ns(origin);
    thread::sleep(Duration::from_millis(duration_ms));
    let mut response = [0_u8; 1];
    client.read_exact(&mut response)?;
    drop(client);
    let (accepted_local, accepted_remote) = server
        .join()
        .map_err(|_| std::io::Error::other("TCP fixture worker panicked"))??;
    let closed = elapsed_ns(origin);
    let connected = FixtureFlow {
        id: format!("{}-tcp-{duration_ms}-{opened}", family_name(ip)),
        kind: FixtureFlowKind::TcpConnection,
        process_id: pid,
        process_instance: process_instance.into(),
        protocol: NetworkProtocol::Tcp,
        local: Endpoint::new(local_address.ip(), local_address.port()),
        remote: Some(Endpoint::new(remote_address.ip(), remote_address.port())),
        interface_id: Some("loopback".into()),
        opened_monotonic_ns: opened,
        activity_monotonic_ns: vec![activity],
        closed_monotonic_ns: Some(closed),
        duration_ms,
        expected_remote_destination: true,
    };
    let accepted = FixtureFlow {
        id: format!("{}-tcp-accept-{duration_ms}-{opened}", family_name(ip)),
        kind: FixtureFlowKind::TcpConnection,
        process_id: pid,
        process_instance: process_instance.into(),
        protocol: NetworkProtocol::Tcp,
        local: Endpoint::new(accepted_local.ip(), accepted_local.port()),
        remote: Some(Endpoint::new(accepted_remote.ip(), accepted_remote.port())),
        interface_id: Some("loopback".into()),
        opened_monotonic_ns: opened,
        activity_monotonic_ns: vec![activity],
        closed_monotonic_ns: Some(closed),
        duration_ms,
        expected_remote_destination: true,
    };
    Ok(vec![connected, accepted])
}

fn run_udp_flow(
    ip: IpAddr,
    duration_ms: u64,
    pid: u32,
    process_instance: &str,
    origin: &Instant,
) -> std::io::Result<FixtureFlow> {
    let server = UdpSocket::bind((ip, 0))?;
    server.set_read_timeout(Some(Duration::from_secs(2)))?;
    let remote_address = server.local_addr()?;
    let client = UdpSocket::bind((ip, 0))?;
    client.connect(remote_address)?;
    let local_address = client.local_addr()?;
    let opened = elapsed_ns(origin);
    client.send(&[0xa5])?;
    let activity = elapsed_ns(origin);
    let mut byte = [0_u8; 1];
    let (count, sender) = server.recv_from(&mut byte)?;
    server.send_to(&byte[..count], sender)?;
    thread::sleep(Duration::from_millis(duration_ms));
    let _ = client.recv(&mut byte)?;
    drop(client);
    drop(server);
    let closed = elapsed_ns(origin);
    Ok(FixtureFlow {
        id: format!("{}-udp-{duration_ms}-{opened}", family_name(ip)),
        kind: FixtureFlowKind::UdpConnected,
        process_id: pid,
        process_instance: process_instance.into(),
        protocol: NetworkProtocol::Udp,
        local: Endpoint::new(local_address.ip(), local_address.port()),
        remote: Some(Endpoint::new(remote_address.ip(), remote_address.port())),
        interface_id: Some("loopback".into()),
        opened_monotonic_ns: opened,
        activity_monotonic_ns: vec![activity],
        closed_monotonic_ns: Some(closed),
        duration_ms,
        expected_remote_destination: true,
    })
}

fn elapsed_ns(origin: &Instant) -> u64 {
    origin.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
}

fn family_name(ip: IpAddr) -> &'static str {
    if ip.is_ipv4() { "ipv4" } else { "ipv6" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EvidenceSequencer;
    use sabicheiro_core::{
        CapabilityAvailability, ConnectionEvidence, ConnectionKey, ConnectionState, CounterSource,
        Direction, NetworkProtocol, NormalizedNetworkEvent, ObservationSource, ProviderCapability,
        ProviderDescriptor, ProviderMode, TrafficCounters,
    };

    fn observations(manifest: &FixtureManifest) -> Vec<EvidenceObservation> {
        let descriptor = ProviderDescriptor {
            protocol_version: 1,
            provider_id: "fixture".into(),
            display_name: "Fixture".into(),
            mode: ProviderMode::Fixture,
            generation: 1,
            capabilities: vec![ProviderCapability {
                kind: sabicheiro_core::CaptureCapabilityKind::LifecycleEvents,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::Demo,
                limitation: None,
            }],
        };
        let mut sequencer = EvidenceSequencer::new(descriptor, 0);
        let mut observations = Vec::new();
        for flow in manifest
            .flows
            .iter()
            .filter(|flow| flow.expected_remote_destination)
        {
            let remote = flow.remote.clone().expect("remote peer");
            let key = ConnectionKey {
                pid: Some(flow.process_id),
                process_instance: Some(flow.process_instance.clone()),
                local: flow.local.clone(),
                remote: remote.clone(),
                protocol: flow.protocol,
                interface_id: flow.interface_id.clone(),
                capture_generation: 1,
                flow_generation: observations.len() as u64 + 1,
            };
            let make_event = |kind, timestamp_ms| NormalizedNetworkEvent {
                kind,
                key: key.clone(),
                evidence: ConnectionEvidence::for_peer(
                    &remote,
                    ConnectionState::Established,
                    ObservationSource::Demo,
                    timestamp_ms,
                ),
                timestamp_ms,
                process: Default::default(),
                direction: Direction::Outbound,
                state: if kind == NetworkEventKind::Closed {
                    ConnectionState::Closed
                } else {
                    ConnectionState::Established
                },
                interface: flow.interface_id.clone(),
                hostname: None,
                domain: None,
                geo: None,
                asn: None,
                traffic_delta: TrafficCounters::default(),
                counter_source: CounterSource::Unavailable,
            };
            observations.push(sequencer.wrap_network_at(
                flow.opened_monotonic_ns,
                flow.opened_monotonic_ns as i64 / 1_000_000,
                make_event(
                    NetworkEventKind::Opened,
                    flow.opened_monotonic_ns as i64 / 1_000_000,
                ),
            ));
            let closed = flow.closed_monotonic_ns.expect("closed fixture");
            observations.push(sequencer.wrap_network_at(
                closed.saturating_add(10_000_000),
                closed as i64 / 1_000_000 + 10,
                make_event(NetworkEventKind::Closed, closed as i64 / 1_000_000 + 10),
            ));
        }
        observations
    }

    #[test]
    fn reference_fixture_covers_required_protocols_durations_and_negative_cases() {
        let manifest = reference_manifest();
        for duration in [5, 20, 50, 100, 1_000] {
            assert!(
                manifest
                    .flows
                    .iter()
                    .any(|flow| flow.duration_ms == duration)
            );
        }
        assert!(
            manifest.flows.iter().any(|flow| {
                flow.protocol == NetworkProtocol::Tcp && flow.local.address.is_ipv6()
            })
        );
        assert!(
            manifest.flows.iter().any(|flow| {
                flow.protocol == NetworkProtocol::Udp && flow.local.address.is_ipv4()
            })
        );
        assert!(manifest.flows.iter().any(|flow| {
            flow.kind == FixtureFlowKind::TcpListener && !flow.expected_remote_destination
        }));
        assert!(manifest.flows.iter().any(|flow| {
            flow.kind == FixtureFlowKind::UdpUnconnected && !flow.expected_remote_destination
        }));
    }

    #[test]
    fn perfect_event_stream_meets_release_metrics() {
        let manifest = reference_manifest();
        let report = compare_observations(&manifest, "fixture", &observations(&manifest));
        assert_eq!(report.false_positive_flows, 0);
        assert_eq!(report.incorrect_process_attributions, 0);
        assert_eq!(report.recall_5_to_50_ms, 1.0);
        assert_eq!(report.recall_50_ms_or_longer, 1.0);
        assert_eq!(report.close_latency_p95_ms, Some(10.0));
        assert!(report.meets_release_targets);
    }

    #[test]
    fn snapshots_that_miss_short_flows_report_the_real_recall() {
        let manifest = reference_manifest();
        let all = observations(&manifest);
        let visible = all
            .into_iter()
            .filter(|observation| {
                observation.network.as_ref().is_some_and(|event| {
                    manifest
                        .flows
                        .iter()
                        .any(|flow| flow.duration_ms >= 50 && tuple_matches(flow, event))
                })
            })
            .collect::<Vec<_>>();
        let report = compare_observations(&manifest, "snapshot", &visible);
        assert_eq!(report.recall_5_to_50_ms, 0.0);
        assert_eq!(report.recall_50_ms_or_longer, 1.0);
        assert!(!report.meets_release_targets);
    }
}
