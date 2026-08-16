use sabicheiro_core::EvidenceObservation;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    Provider,
    Fusion,
    Storage,
    Ipc,
    Render,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineQueueClass {
    Lifecycle,
    Context,
    RepeatedMetric,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineQueueStats {
    pub depth: usize,
    pub capacity: usize,
    pub maximum_age_ns: u64,
    pub dropped_lifecycle: u64,
    pub dropped_context: u64,
    pub dropped_repeated_metrics: u64,
    pub coalesced_repeated_metrics: u64,
    pub resync_required: bool,
}

#[derive(Debug, Clone)]
struct QueuedEvidence {
    observation: EvidenceObservation,
    class: PipelineQueueClass,
    coalesce_key: Option<String>,
    enqueued_monotonic_ns: u64,
}

/// A bounded pipeline stage. Lifecycle evidence is preserved ahead of context and
/// repeated metrics; coalescing is opt-in and limited to an explicit metric key.
pub struct BoundedEvidenceQueue {
    stage: PipelineStage,
    capacity: usize,
    queue: VecDeque<QueuedEvidence>,
    stats: PipelineQueueStats,
}

impl BoundedEvidenceQueue {
    pub fn new(stage: PipelineStage, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            stage,
            capacity,
            queue: VecDeque::with_capacity(capacity),
            stats: PipelineQueueStats {
                capacity,
                ..PipelineQueueStats::default()
            },
        }
    }

    pub const fn stage(&self) -> PipelineStage {
        self.stage
    }

    pub fn push(
        &mut self,
        observation: EvidenceObservation,
        class: PipelineQueueClass,
        coalesce_key: Option<String>,
        now_monotonic_ns: u64,
    ) -> bool {
        if class == PipelineQueueClass::RepeatedMetric
            && let Some(key) = coalesce_key.as_deref()
            && let Some(existing) = self.queue.iter_mut().rev().find(|queued| {
                queued.class == PipelineQueueClass::RepeatedMetric
                    && queued.coalesce_key.as_deref() == Some(key)
            })
        {
            existing.observation = observation;
            self.stats.coalesced_repeated_metrics =
                self.stats.coalesced_repeated_metrics.saturating_add(1);
            self.refresh_stats(now_monotonic_ns);
            return true;
        }
        if self.queue.len() >= self.capacity {
            let eviction = match class {
                PipelineQueueClass::Lifecycle => self
                    .queue
                    .iter()
                    .position(|queued| queued.class == PipelineQueueClass::RepeatedMetric)
                    .or_else(|| {
                        self.queue
                            .iter()
                            .position(|queued| queued.class == PipelineQueueClass::Context)
                    })
                    .or(Some(0)),
                PipelineQueueClass::Context => self
                    .queue
                    .iter()
                    .position(|queued| queued.class == PipelineQueueClass::RepeatedMetric),
                PipelineQueueClass::RepeatedMetric => None,
            };
            let Some(index) = eviction else {
                self.count_drop(class);
                self.refresh_stats(now_monotonic_ns);
                return false;
            };
            if let Some(evicted) = self.queue.remove(index) {
                self.count_drop(evicted.class);
            }
        }
        self.queue.push_back(QueuedEvidence {
            observation,
            class,
            coalesce_key,
            enqueued_monotonic_ns: now_monotonic_ns,
        });
        self.refresh_stats(now_monotonic_ns);
        true
    }

    pub fn pop(&mut self, now_monotonic_ns: u64) -> Option<EvidenceObservation> {
        let observation = self.queue.pop_front().map(|queued| queued.observation);
        self.refresh_stats(now_monotonic_ns);
        observation
    }

    pub fn mark_resynced(&mut self) {
        self.stats.resync_required = false;
    }

    pub fn stats(&mut self, now_monotonic_ns: u64) -> PipelineQueueStats {
        self.refresh_stats(now_monotonic_ns);
        self.stats.clone()
    }

    fn count_drop(&mut self, class: PipelineQueueClass) {
        match class {
            PipelineQueueClass::Lifecycle => {
                self.stats.dropped_lifecycle = self.stats.dropped_lifecycle.saturating_add(1);
                self.stats.resync_required = true;
            }
            PipelineQueueClass::Context => {
                self.stats.dropped_context = self.stats.dropped_context.saturating_add(1);
            }
            PipelineQueueClass::RepeatedMetric => {
                self.stats.dropped_repeated_metrics =
                    self.stats.dropped_repeated_metrics.saturating_add(1);
            }
        }
    }

    fn refresh_stats(&mut self, now_monotonic_ns: u64) {
        self.stats.depth = self.queue.len();
        self.stats.maximum_age_ns = self.queue.front().map_or(0, |queued| {
            now_monotonic_ns.saturating_sub(queued.enqueued_monotonic_ns)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EvidenceSequencer;
    use sabicheiro_core::{
        CapabilityAvailability, CaptureCapabilityKind, ConnectionEvidence, ConnectionKey,
        ConnectionState, CounterSource, Direction, Endpoint, NetworkEventKind, NetworkProtocol,
        NormalizedNetworkEvent, ObservationSource, ProviderCapability, ProviderDescriptor,
        ProviderMode, TrafficCounters,
    };

    fn observation(sequence: u64) -> EvidenceObservation {
        let descriptor = ProviderDescriptor {
            protocol_version: 1,
            provider_id: format!("fixture-{sequence}"),
            display_name: "Fixture".into(),
            mode: ProviderMode::Fixture,
            generation: 1,
            capabilities: vec![ProviderCapability {
                kind: CaptureCapabilityKind::LifecycleEvents,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::Demo,
                limitation: None,
            }],
        };
        let remote = Endpoint::new("127.0.0.1".parse().expect("IP"), 443);
        let event = NormalizedNetworkEvent {
            kind: NetworkEventKind::Updated,
            key: ConnectionKey::new(
                Some(1),
                Endpoint::new("127.0.0.1".parse().expect("IP"), 40_000),
                remote.clone(),
                NetworkProtocol::Tcp,
            ),
            evidence: ConnectionEvidence::for_peer(
                &remote,
                ConnectionState::Established,
                ObservationSource::Demo,
                sequence as i64,
            ),
            timestamp_ms: sequence as i64,
            process: Default::default(),
            direction: Direction::Outbound,
            state: ConnectionState::Established,
            interface: None,
            hostname: None,
            domain: None,
            geo: None,
            asn: None,
            traffic_delta: TrafficCounters::default(),
            counter_source: CounterSource::Unavailable,
        };
        EvidenceSequencer::new(descriptor, 0).wrap_network_at(sequence, sequence as i64, event)
    }

    #[test]
    fn storm_preserves_lifecycle_before_repeated_metrics() {
        let mut queue = BoundedEvidenceQueue::new(PipelineStage::Provider, 3);
        assert!(queue.push(
            observation(1),
            PipelineQueueClass::RepeatedMetric,
            Some("flow-a".into()),
            1
        ));
        assert!(queue.push(
            observation(2),
            PipelineQueueClass::RepeatedMetric,
            Some("flow-b".into()),
            2
        ));
        assert!(queue.push(observation(3), PipelineQueueClass::Context, None, 3));
        assert!(queue.push(observation(4), PipelineQueueClass::Lifecycle, None, 4));
        let stats = queue.stats(5);
        assert_eq!(stats.depth, 3);
        assert_eq!(stats.dropped_repeated_metrics, 1);
        assert_eq!(stats.dropped_lifecycle, 0);
        assert!(!stats.resync_required);
    }

    #[test]
    fn metrics_coalesce_only_with_an_explicit_matching_key() {
        let mut queue = BoundedEvidenceQueue::new(PipelineStage::Storage, 2);
        queue.push(
            observation(1),
            PipelineQueueClass::RepeatedMetric,
            Some("same-flow".into()),
            1,
        );
        queue.push(
            observation(2),
            PipelineQueueClass::RepeatedMetric,
            Some("same-flow".into()),
            2,
        );
        let stats = queue.stats(3);
        assert_eq!(stats.depth, 1);
        assert_eq!(stats.coalesced_repeated_metrics, 1);
    }

    #[test]
    fn losing_lifecycle_forces_explicit_resync() {
        let mut queue = BoundedEvidenceQueue::new(PipelineStage::Ipc, 1);
        queue.push(observation(1), PipelineQueueClass::Lifecycle, None, 1);
        queue.push(observation(2), PipelineQueueClass::Lifecycle, None, 2);
        assert!(queue.stats(3).resync_required);
        queue.mark_resynced();
        assert!(!queue.stats(3).resync_required);
    }
}
