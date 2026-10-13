use sabicheiro_core::{
    ConnectionKey, CorrelationQuality, CounterSource, DnsAnswerEvidence, EvidenceObservation,
    NetworkEventKind, NormalizedNetworkEvent, ObservationKind, ObservationSource, ProviderGap,
    TrafficCounters,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

const DEFAULT_CORRELATION_WINDOW_NS: u64 = 2_500_000_000;
const MAX_RECENT_RECORDS: usize = 65_536;
const MAX_ACTIVE_TRACKS: usize = 65_536;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FusionStats {
    pub ingested_observations: u64,
    pub duplicate_observations: u64,
    pub deduplicated_counter_events: u64,
    pub ambiguous_events: u64,
    pub orphan_events: u64,
    pub provider_gaps: u64,
    pub rejected_late_observations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationCandidate {
    pub observation_id: String,
    pub candidate_keys: Vec<ConnectionKey>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FusionBatch {
    pub events: Vec<NormalizedNetworkEvent>,
    pub dns_answers: Vec<DnsAnswerEvidence>,
    pub gaps: Vec<ProviderGap>,
    pub candidates: Vec<CorrelationCandidate>,
    pub stats: FusionStats,
}

#[derive(Debug, Clone)]
struct FlowTrack {
    key: ConnectionKey,
    active: bool,
    last_monotonic_ns: u64,
    providers: BTreeMap<String, ConnectionKey>,
}

pub struct EvidenceFusion {
    seen_observations: HashSet<String>,
    observation_order: VecDeque<(String, u64)>,
    retired_before_ns: Option<u64>,
    counter_fingerprints: HashSet<String>,
    counter_order: VecDeque<String>,
    tracks: Vec<FlowTrack>,
    correlation_window_ns: u64,
    next_flow_generation: u64,
    stats: FusionStats,
}

impl EvidenceFusion {
    pub fn new(correlation_window_ns: u64) -> Self {
        Self {
            seen_observations: HashSet::new(),
            observation_order: VecDeque::new(),
            retired_before_ns: None,
            counter_fingerprints: HashSet::new(),
            counter_order: VecDeque::new(),
            tracks: Vec::new(),
            correlation_window_ns: correlation_window_ns.max(1),
            next_flow_generation: 1,
            stats: FusionStats::default(),
        }
    }

    pub fn ingest(
        &mut self,
        observations: impl IntoIterator<Item = EvidenceObservation>,
    ) -> FusionBatch {
        let mut observations = observations.into_iter().collect::<Vec<_>>();
        observations.sort_by(|left, right| {
            (
                left.clock.monotonic_ns,
                &left.cursor.provider_id,
                left.cursor.generation,
                left.cursor.sequence,
                &left.id,
            )
                .cmp(&(
                    right.clock.monotonic_ns,
                    &right.cursor.provider_id,
                    right.cursor.generation,
                    right.cursor.sequence,
                    &right.id,
                ))
        });
        let mut batch = FusionBatch::default();
        for mut observation in observations {
            if self
                .retired_before_ns
                .is_some_and(|floor| observation.clock.monotonic_ns <= floor)
            {
                self.stats.rejected_late_observations =
                    self.stats.rejected_late_observations.saturating_add(1);
                self.record_capacity_gap(
                    &observation,
                    "record is older than the bounded live deduplication window",
                    &mut batch,
                );
                continue;
            }
            if !self.seen_observations.insert(observation.id.clone()) {
                self.stats.duplicate_observations =
                    self.stats.duplicate_observations.saturating_add(1);
                continue;
            }
            self.observation_order
                .push_back((observation.id.clone(), observation.clock.monotonic_ns));
            if self.observation_order.len() > MAX_RECENT_RECORDS {
                if let Some((id, at)) = self.observation_order.pop_front() {
                    self.seen_observations.remove(&id);
                    self.retired_before_ns = Some(self.retired_before_ns.unwrap_or(0).max(at));
                }
            }
            self.stats.ingested_observations = self.stats.ingested_observations.saturating_add(1);
            if observation.kind == ObservationKind::ProviderGap {
                if let Some(gap) = observation.gap {
                    self.stats.provider_gaps = self.stats.provider_gaps.saturating_add(1);
                    batch.gaps.push(gap);
                }
                continue;
            }
            if observation.kind == ObservationKind::Dns {
                if let Some(answer) = observation.dns {
                    batch.dns_answers.push(answer);
                }
                continue;
            }
            let Some(mut event) = observation.network.take() else {
                continue;
            };
            let raw_key = event.key.clone();
            let matches = self.matching_tracks(
                &event.key,
                observation.clock.monotonic_ns,
                &observation.cursor.provider_id,
            );
            let selected = select_track(&self.tracks, &matches, &event.key);
            match selected {
                TrackSelection::One(index) => {
                    let track = &mut self.tracks[index];
                    if event.key.process_instance.is_none() && track.key.process_instance.is_some()
                    {
                        event.evidence.correlation_quality = CorrelationQuality::Candidate;
                        batch.candidates.push(CorrelationCandidate {
                            observation_id: observation.id.clone(),
                            candidate_keys: vec![track.key.clone()],
                            reason: "tuple matches but process creation evidence is unavailable"
                                .into(),
                        });
                    } else {
                        let exact_key = track.key == event.key;
                        event.key = track.key.clone();
                        event.interface = event
                            .interface
                            .clone()
                            .or_else(|| track.key.interface_id.clone());
                        event.evidence.correlation_quality = if event.key.process_instance.is_none()
                        {
                            CorrelationQuality::Unknown
                        } else if exact_key {
                            source_quality(event.evidence.observation_source)
                        } else {
                            CorrelationQuality::Correlated
                        };
                        track.last_monotonic_ns = observation.clock.monotonic_ns;
                        track
                            .providers
                            .insert(observation.cursor.provider_id.clone(), raw_key);
                        if event.kind == NetworkEventKind::Closed {
                            track.active = false;
                        }
                    }
                }
                TrackSelection::Ambiguous(indices) => {
                    event.evidence.correlation_quality = CorrelationQuality::Candidate;
                    self.stats.ambiguous_events = self.stats.ambiguous_events.saturating_add(1);
                    batch.candidates.push(CorrelationCandidate {
                        observation_id: observation.id.clone(),
                        candidate_keys: indices
                            .iter()
                            .map(|index| self.tracks[*index].key.clone())
                            .collect(),
                        reason: "multiple active flows match the observed tuple and time window"
                            .into(),
                    });
                }
                TrackSelection::None => {
                    if self.tracks.len() >= MAX_ACTIVE_TRACKS {
                        self.tracks.retain(|track| track.active);
                        if self.tracks.len() >= MAX_ACTIVE_TRACKS {
                            self.record_capacity_gap(
                                &observation,
                                "active flow correlation capacity exceeded",
                                &mut batch,
                            );
                            continue;
                        }
                    }
                    if event.kind != NetworkEventKind::Opened {
                        self.stats.orphan_events = self.stats.orphan_events.saturating_add(1);
                        event.evidence.correlation_quality = match event.evidence.observation_source
                        {
                            ObservationSource::WindowsIpHelper
                            | ObservationSource::LinuxInetDiag => CorrelationQuality::SnapshotOnly,
                            _ => CorrelationQuality::Unknown,
                        };
                    }
                    let mut key = event.key.clone();
                    if key.capture_generation == 0 {
                        key.capture_generation = observation.cursor.generation;
                    }
                    if key.flow_generation == 0 {
                        key.flow_generation = self.next_flow_generation;
                        self.next_flow_generation = self.next_flow_generation.saturating_add(1);
                    }
                    event.key = key.clone();
                    self.tracks.push(FlowTrack {
                        key,
                        active: event.kind != NetworkEventKind::Closed,
                        last_monotonic_ns: observation.clock.monotonic_ns,
                        providers: BTreeMap::from([(
                            observation.cursor.provider_id.clone(),
                            raw_key,
                        )]),
                    });
                }
            }
            // Linux packets have one path; distinct datagrams can share timestamp and size.
            if event.traffic_delta != TrafficCounters::default()
                && event.evidence.observation_source != ObservationSource::LinuxEbpf
            {
                let fingerprint = counter_fingerprint(&event, observation.clock.monotonic_ns);
                if !self.counter_fingerprints.insert(fingerprint) {
                    event.traffic_delta = TrafficCounters::default();
                    event.counter_source = CounterSource::Unavailable;
                    self.stats.deduplicated_counter_events =
                        self.stats.deduplicated_counter_events.saturating_add(1);
                } else {
                    self.counter_order
                        .push_back(counter_fingerprint(&event, observation.clock.monotonic_ns));
                    if self.counter_order.len() > MAX_RECENT_RECORDS {
                        if let Some(expired) = self.counter_order.pop_front() {
                            self.counter_fingerprints.remove(&expired);
                        }
                    }
                }
            }
            batch.events.push(event);
        }
        self.tracks.retain(|track| track.active);
        batch.stats = self.stats.clone();
        batch
    }

    pub fn stats(&self) -> &FusionStats {
        &self.stats
    }

    fn record_capacity_gap(
        &mut self,
        observation: &EvidenceObservation,
        reason: &str,
        batch: &mut FusionBatch,
    ) {
        self.stats.provider_gaps = self.stats.provider_gaps.saturating_add(1);
        let cursor = &observation.cursor;
        if let Some(gap) = batch.gaps.last_mut().filter(|gap| {
            gap.provider_id == cursor.provider_id
                && gap.generation == cursor.generation
                && gap.reason == reason
        }) {
            gap.first_missing_sequence = gap.first_missing_sequence.min(cursor.sequence);
            gap.last_missing_sequence = gap.last_missing_sequence.max(cursor.sequence);
        } else {
            batch.gaps.push(ProviderGap {
                provider_id: cursor.provider_id.clone(),
                generation: cursor.generation,
                first_missing_sequence: cursor.sequence,
                last_missing_sequence: cursor.sequence,
                detected_at_ms: observation.clock.utc_ms,
                affected_capabilities: BTreeSet::from([
                    sabicheiro_core::CaptureCapabilityKind::LifecycleEvents,
                ]),
                reason: reason.into(),
                reconciled_at_ms: None,
            });
        }
    }

    fn matching_tracks(
        &self,
        key: &ConnectionKey,
        monotonic_ns: u64,
        provider: &str,
    ) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                track.active
                    && (stable_track_identity(&track.key, key)
                        || track.providers.get(provider).is_some_and(|raw| {
                            raw == key
                                && (raw.process_instance.is_some() || raw.flow_generation != 0)
                        })
                        || (monotonic_ns.saturating_sub(track.last_monotonic_ns)
                            <= self.correlation_window_ns
                            && monotonic_ns >= track.last_monotonic_ns))
                    && tuple_compatible(&track.key, key)
            })
            .map(|(index, _)| index)
            .collect()
    }
}

fn stable_track_identity(left: &ConnectionKey, right: &ConnectionKey) -> bool {
    tuple_compatible(left, right)
        && left.process_instance.is_some()
        && left.process_instance == right.process_instance
        && (right.capture_generation == 0 || left.capture_generation == right.capture_generation)
        && (right.flow_generation == 0 || left.flow_generation == right.flow_generation)
}

impl Default for EvidenceFusion {
    fn default() -> Self {
        Self::new(DEFAULT_CORRELATION_WINDOW_NS)
    }
}

pub fn replay_evidence(observations: impl IntoIterator<Item = EvidenceObservation>) -> FusionBatch {
    EvidenceFusion::default().ingest(observations)
}

enum TrackSelection {
    None,
    One(usize),
    Ambiguous(Vec<usize>),
}

fn select_track(tracks: &[FlowTrack], matches: &[usize], event: &ConnectionKey) -> TrackSelection {
    if matches.is_empty() {
        return TrackSelection::None;
    }
    let exact = matches
        .iter()
        .copied()
        .filter(|index| tracks[*index].key == *event)
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return TrackSelection::One(exact[0]);
    }
    if event.process_instance.is_some()
        && matches
            .iter()
            .all(|index| tracks[*index].key.process_instance.is_none())
    {
        return TrackSelection::None;
    }
    let strongest_score = matches
        .iter()
        .map(|index| match_score(&tracks[*index].key, event))
        .max()
        .unwrap_or_default();
    let strongest = matches
        .iter()
        .copied()
        .filter(|index| match_score(&tracks[*index].key, event) == strongest_score)
        .collect::<Vec<_>>();
    match strongest.as_slice() {
        [index] => TrackSelection::One(*index),
        _ => TrackSelection::Ambiguous(strongest),
    }
}

fn tuple_compatible(left: &ConnectionKey, right: &ConnectionKey) -> bool {
    if left.protocol != right.protocol || left.local != right.local || left.remote != right.remote {
        return false;
    }
    if left.pid.is_some() && right.pid.is_some() && left.pid != right.pid {
        return false;
    }
    if left.interface_id.is_some()
        && right.interface_id.is_some()
        && left.interface_id != right.interface_id
    {
        return false;
    }
    if left.process_instance.is_some()
        && right.process_instance.is_some()
        && left.process_instance != right.process_instance
    {
        return false;
    }
    true
}

fn match_score(left: &ConnectionKey, right: &ConnectionKey) -> u8 {
    let process = u8::from(
        left.process_instance.is_some() && left.process_instance == right.process_instance,
    );
    let pid = u8::from(left.pid.is_some() && left.pid == right.pid);
    let interface =
        u8::from(left.interface_id.is_some() && left.interface_id == right.interface_id);
    process
        .saturating_mul(4)
        .saturating_add(pid.saturating_mul(2))
        .saturating_add(interface)
}

fn source_quality(source: ObservationSource) -> CorrelationQuality {
    match source {
        ObservationSource::WindowsIpHelper | ObservationSource::LinuxInetDiag => {
            CorrelationQuality::SnapshotOnly
        }
        _ => CorrelationQuality::Confirmed,
    }
}

fn counter_fingerprint(event: &NormalizedNetworkEvent, monotonic_ns: u64) -> String {
    // Millisecond bucketing catches the same native record traversing direct/helper
    // paths while retaining distinct packet events in separate clock buckets.
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        event.key.protocol.as_str(),
        event.key.local.label(),
        event.key.remote.label(),
        event.kind as u8,
        monotonic_ns / 1_000_000,
        event.traffic_delta.bytes_sent,
        event.traffic_delta.bytes_received,
        event.traffic_delta.packets_sent,
        event.traffic_delta.packets_received,
        event.key.pid.unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EvidenceSequencer;
    use sabicheiro_core::{
        CapabilityAvailability, CaptureCapabilityKind, ConnectionEvidence, ConnectionState,
        Direction, Endpoint, NetworkProtocol, ProcessInfo, ProviderCapability, ProviderDescriptor,
        ProviderMode,
    };

    fn observation(
        provider: &str,
        source: ObservationSource,
        monotonic_ns: u64,
        kind: NetworkEventKind,
        pid: Option<u32>,
        process_instance: Option<&str>,
        counters: TrafficCounters,
    ) -> EvidenceObservation {
        let descriptor = ProviderDescriptor {
            protocol_version: 1,
            provider_id: provider.into(),
            display_name: provider.into(),
            mode: ProviderMode::Fixture,
            generation: 1,
            capabilities: vec![ProviderCapability {
                kind: CaptureCapabilityKind::LifecycleEvents,
                availability: CapabilityAvailability::Available,
                source,
                limitation: None,
            }],
        };
        let remote = Endpoint::new("127.0.0.1".parse().expect("IP"), 443);
        let mut key = ConnectionKey::new(
            pid,
            Endpoint::new("127.0.0.1".parse().expect("IP"), 40_000),
            remote.clone(),
            NetworkProtocol::Tcp,
        );
        key.process_instance = process_instance.map(str::to_owned);
        let event = NormalizedNetworkEvent {
            kind,
            key,
            evidence: ConnectionEvidence::for_peer(
                &remote,
                ConnectionState::Established,
                source,
                monotonic_ns as i64 / 1_000_000,
            ),
            timestamp_ms: monotonic_ns as i64 / 1_000_000,
            process: ProcessInfo::default(),
            direction: Direction::Outbound,
            state: if kind == NetworkEventKind::Closed {
                ConnectionState::Closed
            } else {
                ConnectionState::Established
            },
            interface: Some("loopback".into()),
            hostname: None,
            domain: None,
            geo: None,
            asn: None,
            traffic_delta: counters,
            counter_source: CounterSource::EtwFlow,
        };
        let mut observation = EvidenceSequencer::new(descriptor, 0).wrap_network_at(
            monotonic_ns,
            monotonic_ns as i64 / 1_000_000,
            event,
        );
        observation.cursor.sequence = monotonic_ns.saturating_add(1);
        observation.id = format!(
            "{}:{}:{}:{:?}",
            observation.cursor.provider_id,
            observation.cursor.generation,
            observation.cursor.sequence,
            observation.kind
        )
        .to_ascii_lowercase();
        observation
    }

    #[test]
    fn distinct_linux_packets_in_the_same_clock_bucket_are_counted() {
        let counters = TrafficCounters {
            bytes_sent: 32,
            packets_sent: 1,
            ..TrafficCounters::default()
        };
        let first = observation(
            "linux.ebpf",
            ObservationSource::LinuxEbpf,
            1_000_001,
            NetworkEventKind::Updated,
            None,
            None,
            counters,
        );
        let second = observation(
            "linux.ebpf",
            ObservationSource::LinuxEbpf,
            1_000_002,
            NetworkEventKind::Updated,
            None,
            None,
            counters,
        );
        let mut fusion = EvidenceFusion::new(2_500_000_000);
        let batch = fusion.ingest([first.clone(), second, first]);
        assert_eq!(batch.events.len(), 2);
        assert_eq!(
            batch
                .events
                .iter()
                .map(|event| event.traffic_delta.bytes_sent)
                .sum::<u64>(),
            64
        );
        assert_eq!(batch.stats.duplicate_observations, 1);
    }

    #[test]
    fn reordered_sources_produce_the_same_canonical_projection() {
        let snapshot = observation(
            "snapshot",
            ObservationSource::WindowsIpHelper,
            1_000_000,
            NetworkEventKind::Opened,
            Some(7),
            Some("pid:7:start:1"),
            TrafficCounters::default(),
        );
        let event = observation(
            "events",
            ObservationSource::WindowsEtw,
            2_000_000,
            NetworkEventKind::Updated,
            Some(7),
            None,
            TrafficCounters {
                bytes_sent: 10,
                ..TrafficCounters::default()
            },
        );
        let forward = replay_evidence([snapshot.clone(), event.clone()]);
        let reverse = replay_evidence([event, snapshot]);
        assert_eq!(forward.events, reverse.events);
        assert_eq!(
            forward.events[1].evidence.correlation_quality,
            CorrelationQuality::Candidate
        );
        assert_eq!(
            forward.events[0].key.process_instance.as_deref(),
            Some("pid:7:start:1")
        );
    }

    #[test]
    fn retries_and_cross_path_counter_duplicates_do_not_double_count() {
        let event = observation(
            "events-a",
            ObservationSource::WindowsEtw,
            2_000_000,
            NetworkEventKind::Updated,
            Some(7),
            None,
            TrafficCounters {
                bytes_sent: 10,
                packets_sent: 1,
                ..TrafficCounters::default()
            },
        );
        let retry = event.clone();
        let duplicate_path = observation(
            "events-b",
            ObservationSource::WindowsHelper,
            2_500_000,
            NetworkEventKind::Updated,
            Some(7),
            None,
            TrafficCounters {
                bytes_sent: 10,
                packets_sent: 1,
                ..TrafficCounters::default()
            },
        );
        let batch = replay_evidence([event, retry, duplicate_path]);
        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.stats.duplicate_observations, 1);
        assert_eq!(batch.stats.deduplicated_counter_events, 1);
        assert_eq!(
            batch
                .events
                .iter()
                .map(|event| event.traffic_delta.bytes_sent)
                .sum::<u64>(),
            10
        );
    }

    #[test]
    fn pid_reuse_never_merges_distinct_process_instances() {
        let first = observation(
            "events",
            ObservationSource::WindowsEtw,
            1,
            NetworkEventKind::Opened,
            Some(7),
            Some("pid:7:start:1"),
            TrafficCounters::default(),
        );
        let reused = observation(
            "events",
            ObservationSource::WindowsEtw,
            2,
            NetworkEventKind::Opened,
            Some(7),
            Some("pid:7:start:2"),
            TrafficCounters::default(),
        );
        let batch = replay_evidence([first, reused]);
        assert_ne!(
            batch.events[0].key.flow_generation,
            batch.events[1].key.flow_generation
        );
    }

    #[test]
    fn provider_flow_identity_survives_canonicalization_and_idle_time() {
        let mut snapshot = observation(
            "linux.inet_diag",
            ObservationSource::LinuxInetDiag,
            1,
            NetworkEventKind::Opened,
            Some(7),
            Some("pid:7:start:2"),
            TrafficCounters::default(),
        );
        snapshot.network.as_mut().unwrap().key.flow_generation = 101;
        let mut native = observation(
            "linux.ebpf",
            ObservationSource::LinuxEbpf,
            2,
            NetworkEventKind::Updated,
            Some(7),
            Some("pid:7:start:2"),
            TrafficCounters::default(),
        );
        native.network.as_mut().unwrap().key.flow_generation = 909;
        let mut idle_update = native.clone();
        idle_update.id = "linux.ebpf:1:3:network".into();
        idle_update.cursor.sequence = 3;
        idle_update.clock.monotonic_ns = 10_000_000_000;
        let batch = replay_evidence([snapshot, native, idle_update]);
        assert_eq!(batch.events.len(), 3);
        assert!(
            batch
                .events
                .iter()
                .all(|event| event.key.flow_generation == 101)
        );
        assert_eq!(batch.events[0].key, batch.events[2].key);
    }

    #[test]
    fn unknown_owner_does_not_inherit_a_confirmed_process_instance() {
        let known = observation(
            "snapshot",
            ObservationSource::LinuxInetDiag,
            1,
            NetworkEventKind::Opened,
            Some(7),
            Some("pid:7:start:2"),
            TrafficCounters::default(),
        );
        let unknown = observation(
            "events",
            ObservationSource::LinuxEbpf,
            2,
            NetworkEventKind::Updated,
            Some(7),
            None,
            TrafficCounters::default(),
        );
        let batch = replay_evidence([known, unknown]);
        assert!(batch.events[1].key.process_instance.is_none());
        assert_eq!(
            batch.events[1].evidence.correlation_quality,
            CorrelationQuality::Candidate
        );
        assert_eq!(batch.candidates.len(), 1);
    }

    #[test]
    fn gaps_remain_first_class_evidence() {
        let mut sequencer = EvidenceSequencer::new(
            ProviderDescriptor {
                protocol_version: 1,
                provider_id: "events".into(),
                display_name: "events".into(),
                mode: ProviderMode::Fixture,
                generation: 1,
                capabilities: vec![],
            },
            0,
        );
        let gap = sequencer
            .record_gap(
                3,
                1,
                1,
                [CaptureCapabilityKind::LifecycleEvents],
                "overflow",
            )
            .expect("gap");
        let batch = replay_evidence([gap]);
        assert_eq!(batch.gaps.len(), 1);
        assert_eq!(batch.gaps[0].missing_count(), 3);
    }

    #[test]
    fn stable_process_flow_survives_a_long_idle_interval() {
        let mut fusion = EvidenceFusion::default();
        let first = fusion.ingest([observation(
            "snapshot",
            ObservationSource::WindowsIpHelper,
            1,
            NetworkEventKind::Opened,
            Some(7),
            Some("pid:7:start:1"),
            TrafficCounters::default(),
        )]);
        let later = fusion.ingest([observation(
            "snapshot",
            ObservationSource::WindowsIpHelper,
            10_000_000_001,
            NetworkEventKind::Updated,
            Some(7),
            Some("pid:7:start:1"),
            TrafficCounters::default(),
        )]);
        assert_eq!(first.events[0].key, later.events[0].key);
        assert_eq!(fusion.tracks.len(), 1);
    }

    #[test]
    fn closed_flows_and_recent_record_caches_remain_bounded() {
        let mut fusion = EvidenceFusion::default();
        for at in 1..=(MAX_RECENT_RECORDS as u64 + 8) {
            let batch = fusion.ingest([observation(
                "events",
                ObservationSource::WindowsEtw,
                at,
                NetworkEventKind::Closed,
                Some(7),
                Some("pid:7:start:1"),
                TrafficCounters::default(),
            )]);
            assert_eq!(batch.events.len(), 1);
            assert!(fusion.tracks.is_empty());
        }
        assert_eq!(fusion.seen_observations.len(), MAX_RECENT_RECORDS);
        let late = fusion.ingest([observation(
            "events",
            ObservationSource::WindowsEtw,
            1,
            NetworkEventKind::Closed,
            Some(7),
            Some("pid:7:start:1"),
            TrafficCounters::default(),
        )]);
        assert!(late.events.is_empty());
        assert_eq!(late.gaps.len(), 1);
    }
}
