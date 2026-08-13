use sabicheiro_core::{
    ConnectionKey, CorrelationQuality, NormalizedNetworkEvent, ObservationSource,
};
use std::collections::VecDeque;

const DEFAULT_CORRELATION_WINDOW_MS: i64 = 2_500;
const MAX_CANDIDATES: usize = 8_192;

#[derive(Debug, Clone)]
struct Candidate {
    key: ConnectionKey,
    observed_at_ms: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CorrelationStats {
    pub correlated: u64,
    pub approximate: u64,
    pub orphaned: u64,
}

/// Correlates high-frequency event records with authoritative socket snapshots.
/// It never distributes aggregate counters and it refuses ambiguous candidates.
pub struct FlowCorrelator {
    candidates: VecDeque<Candidate>,
    window_ms: i64,
    stats: CorrelationStats,
}

impl Default for FlowCorrelator {
    fn default() -> Self {
        Self::new(DEFAULT_CORRELATION_WINDOW_MS)
    }
}

impl FlowCorrelator {
    pub fn new(window_ms: i64) -> Self {
        Self {
            candidates: VecDeque::new(),
            window_ms: window_ms.max(1),
            stats: CorrelationStats::default(),
        }
    }

    pub fn observe_snapshots<'a>(
        &mut self,
        events: impl IntoIterator<Item = &'a NormalizedNetworkEvent>,
        now_ms: i64,
    ) {
        self.prune(now_ms);
        for event in events {
            if !matches!(
                event.evidence.observation_source,
                ObservationSource::WindowsIpHelper | ObservationSource::LinuxInetDiag
            ) {
                continue;
            }
            if let Some(candidate) = self
                .candidates
                .iter_mut()
                .find(|candidate| candidate.key == event.key)
            {
                candidate.observed_at_ms = event.timestamp_ms;
            } else {
                self.candidates.push_back(Candidate {
                    key: event.key.clone(),
                    observed_at_ms: event.timestamp_ms,
                });
            }
        }
        while self.candidates.len() > MAX_CANDIDATES {
            self.candidates.pop_front();
        }
    }

    pub fn correlate(&mut self, event: &mut NormalizedNetworkEvent) -> CorrelationQuality {
        self.prune(event.timestamp_ms);
        if event.evidence.observation_source != ObservationSource::WindowsEtw {
            return event.evidence.correlation_quality;
        }
        let mut matches = self
            .candidates
            .iter()
            .filter(|candidate| tuple_matches(&candidate.key, &event.key))
            .collect::<Vec<_>>();
        matches.sort_by_key(|candidate| {
            event
                .timestamp_ms
                .saturating_sub(candidate.observed_at_ms)
                .unsigned_abs()
        });
        let quality = match matches.as_slice() {
            [] => {
                self.stats.orphaned = self.stats.orphaned.saturating_add(1);
                CorrelationQuality::Unavailable
            }
            [candidate] => {
                adopt_candidate(event, &candidate.key);
                self.stats.correlated = self.stats.correlated.saturating_add(1);
                CorrelationQuality::Correlated
            }
            [first, second, ..]
                if candidate_score(&first.key, &event.key)
                    > candidate_score(&second.key, &event.key) =>
            {
                adopt_candidate(event, &first.key);
                self.stats.approximate = self.stats.approximate.saturating_add(1);
                CorrelationQuality::Approximate
            }
            _ => {
                // Ambiguous events retain their counters as orphan evidence instead of
                // assigning bytes to an arbitrary flow.
                self.stats.orphaned = self.stats.orphaned.saturating_add(1);
                CorrelationQuality::Unavailable
            }
        };
        event.evidence.correlation_quality = quality;
        if quality == CorrelationQuality::Unavailable {
            event.evidence.role_reason = "orphan_etw_event".into();
        } else {
            event.evidence.role_reason = "ip_helper_etw_correlation".into();
        }
        quality
    }

    pub const fn stats(&self) -> CorrelationStats {
        self.stats
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.stats = CorrelationStats::default();
    }

    fn prune(&mut self, now_ms: i64) {
        let cutoff = now_ms.saturating_sub(self.window_ms);
        self.candidates
            .retain(|candidate| candidate.observed_at_ms >= cutoff);
    }
}

fn tuple_matches(candidate: &ConnectionKey, event: &ConnectionKey) -> bool {
    candidate.pid == event.pid
        && candidate.protocol == event.protocol
        && candidate.local == event.local
        && candidate.remote == event.remote
}

fn candidate_score(candidate: &ConnectionKey, event: &ConnectionKey) -> u8 {
    let process = u8::from(
        candidate.process_instance.is_some()
            && candidate.process_instance == event.process_instance,
    );
    let interface =
        u8::from(candidate.interface_id.is_some() && candidate.interface_id == event.interface_id);
    process.saturating_mul(2).saturating_add(interface)
}

fn adopt_candidate(event: &mut NormalizedNetworkEvent, candidate: &ConnectionKey) {
    event.key = candidate.clone();
    event.interface = candidate
        .interface_id
        .clone()
        .or_else(|| event.interface.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use sabicheiro_core::{
        ConnectionEvidence, ConnectionState, CounterSource, Direction, Endpoint, NetworkEventKind,
        NetworkProtocol, ProcessInfo, TrafficCounters,
    };

    fn event(source: ObservationSource, timestamp_ms: i64) -> NormalizedNetworkEvent {
        let remote = Endpoint::new("1.1.1.1".parse().expect("IP"), 443);
        NormalizedNetworkEvent {
            kind: NetworkEventKind::Updated,
            key: ConnectionKey {
                pid: Some(42),
                process_instance: Some("pid:42:start:1".into()),
                local: Endpoint::new("192.168.1.10".parse().expect("IP"), 50_000),
                remote: remote.clone(),
                protocol: NetworkProtocol::Tcp,
                interface_id: Some("ethernet".into()),
                capture_generation: u64::from(source == ObservationSource::WindowsIpHelper),
                flow_generation: u64::from(source == ObservationSource::WindowsIpHelper),
            },
            evidence: ConnectionEvidence::for_peer(
                &remote,
                ConnectionState::Established,
                source,
                timestamp_ms,
            ),
            timestamp_ms,
            process: ProcessInfo::default(),
            direction: Direction::Outbound,
            state: ConnectionState::Established,
            interface: Some("ethernet".into()),
            hostname: None,
            domain: None,
            geo: None,
            asn: None,
            traffic_delta: TrafficCounters::default(),
            counter_source: CounterSource::Unavailable,
        }
    }

    #[test]
    fn correlates_a_single_etw_event_to_the_snapshot_identity() {
        let snapshot = event(ObservationSource::WindowsIpHelper, 1_000);
        let expected = snapshot.key.clone();
        let mut correlator = FlowCorrelator::new(2_500);
        correlator.observe_snapshots([&snapshot], 1_000);
        let mut etw = event(ObservationSource::WindowsEtw, 1_100);
        etw.key.capture_generation = 0;
        etw.key.flow_generation = 0;
        assert_eq!(
            correlator.correlate(&mut etw),
            CorrelationQuality::Correlated
        );
        assert_eq!(etw.key, expected);
        assert_eq!(correlator.stats().correlated, 1);
    }

    #[test]
    fn stale_or_unknown_events_remain_orphans() {
        let snapshot = event(ObservationSource::WindowsIpHelper, 1_000);
        let mut correlator = FlowCorrelator::new(50);
        correlator.observe_snapshots([&snapshot], 1_000);
        let mut etw = event(ObservationSource::WindowsEtw, 2_000);
        assert_eq!(
            correlator.correlate(&mut etw),
            CorrelationQuality::Unavailable
        );
        assert_eq!(correlator.stats().orphaned, 1);
    }
}
