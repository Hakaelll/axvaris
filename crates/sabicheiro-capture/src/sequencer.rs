use sabicheiro_core::{
    CaptureCapabilityKind, ClockQuality, CorrelationQuality, DnsAnswerEvidence, EvidenceClock,
    EvidenceField, EvidenceObservation, FieldProvenance, NormalizedNetworkEvent, ObservationSource,
    ProviderCursor, ProviderDescriptor, ProviderGap,
};
use std::{
    collections::{BTreeSet, HashMap},
    time::Instant,
};

/// Assigns provider-local ordering independently from UTC. The same provider generation
/// can therefore be replayed without trusting wall-clock adjustments.
pub struct EvidenceSequencer {
    descriptor: ProviderDescriptor,
    clock_id: String,
    origin: Instant,
    utc_origin_ms: i64,
    sequence: u64,
    dropped: u64,
    last_monotonic_ns: u64,
}

impl EvidenceSequencer {
    pub fn new(descriptor: ProviderDescriptor, utc_origin_ms: i64) -> Self {
        let clock_id = format!("{}:{}", descriptor.provider_id, descriptor.generation);
        Self {
            descriptor,
            clock_id,
            origin: Instant::now(),
            utc_origin_ms,
            sequence: 0,
            dropped: 0,
            last_monotonic_ns: 0,
        }
    }

    pub const fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    pub fn wrap_network(&mut self, event: NormalizedNetworkEvent) -> EvidenceObservation {
        let monotonic_ns = self.origin.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
        let utc_ms = self
            .utc_origin_ms
            .saturating_add((monotonic_ns / 1_000_000) as i64);
        self.wrap_network_at(monotonic_ns, utc_ms, event)
    }

    pub fn wrap_network_at(
        &mut self,
        monotonic_ns: u64,
        utc_ms: i64,
        mut event: NormalizedNetworkEvent,
    ) -> EvidenceObservation {
        let (cursor, clock) = self.next(monotonic_ns, utc_ms);
        event.evidence.sequence = cursor.sequence;
        let provenance = network_provenance(&event);
        EvidenceObservation::network(cursor, clock, event, provenance)
    }

    pub fn record_gap(
        &mut self,
        missing: u64,
        monotonic_ns: u64,
        utc_ms: i64,
        affected_capabilities: impl IntoIterator<Item = CaptureCapabilityKind>,
        reason: impl Into<String>,
    ) -> Option<EvidenceObservation> {
        if missing == 0 {
            return None;
        }
        let first_missing_sequence = self.sequence.saturating_add(1);
        self.sequence = self.sequence.saturating_add(missing);
        self.dropped = self.dropped.saturating_add(missing);
        let last_missing_sequence = self.sequence;
        let (cursor, clock) = self.next(monotonic_ns, utc_ms);
        Some(EvidenceObservation::gap(
            cursor,
            clock,
            ProviderGap {
                provider_id: self.descriptor.provider_id.clone(),
                generation: self.descriptor.generation,
                first_missing_sequence,
                last_missing_sequence,
                detected_at_ms: utc_ms,
                affected_capabilities: affected_capabilities.into_iter().collect(),
                reason: reason.into(),
                reconciled_at_ms: None,
            },
        ))
    }

    pub fn wrap_dns_at(
        &mut self,
        monotonic_ns: u64,
        utc_ms: i64,
        dns: DnsAnswerEvidence,
        quality: CorrelationQuality,
        limitation: Option<String>,
    ) -> EvidenceObservation {
        let (cursor, clock) = self.next(monotonic_ns, utc_ms);
        let source = dns.source;
        EvidenceObservation::dns(
            cursor,
            clock,
            dns,
            FieldProvenance {
                field: EvidenceField::Dns,
                source,
                capability: CaptureCapabilityKind::Dns,
                quality,
                observed_at_ms: utc_ms,
                limitation,
                invalidated_by_gap: false,
            },
        )
    }

    fn next(&mut self, monotonic_ns: u64, utc_ms: i64) -> (ProviderCursor, EvidenceClock) {
        self.sequence = self.sequence.saturating_add(1);
        self.last_monotonic_ns = self.last_monotonic_ns.max(monotonic_ns);
        (
            ProviderCursor {
                provider_id: self.descriptor.provider_id.clone(),
                generation: self.descriptor.generation,
                sequence: self.sequence,
                watermark_monotonic_ns: Some(self.last_monotonic_ns),
                dropped_before: self.dropped,
            },
            EvidenceClock {
                clock_id: self.clock_id.clone(),
                monotonic_ns: self.last_monotonic_ns,
                utc_ms,
                quality: ClockQuality::Anchored,
            },
        )
    }
}

pub fn network_provenance(event: &NormalizedNetworkEvent) -> Vec<FieldProvenance> {
    let source = event.evidence.observation_source;
    let lifecycle_capability = match source {
        ObservationSource::WindowsIpHelper | ObservationSource::LinuxInetDiag => {
            CaptureCapabilityKind::SocketSnapshot
        }
        _ => CaptureCapabilityKind::LifecycleEvents,
    };
    let lifecycle_quality = match lifecycle_capability {
        CaptureCapabilityKind::SocketSnapshot => CorrelationQuality::SnapshotOnly,
        _ => event.evidence.correlation_quality,
    };
    let mut provenance = vec![
        FieldProvenance {
            field: EvidenceField::Lifecycle,
            source,
            capability: lifecycle_capability,
            quality: lifecycle_quality,
            observed_at_ms: event.timestamp_ms,
            limitation: (lifecycle_capability == CaptureCapabilityKind::SocketSnapshot)
                .then(|| "open/close time is bounded by the snapshot interval".into()),
            invalidated_by_gap: false,
        },
        FieldProvenance {
            field: EvidenceField::LocalEndpoint,
            source,
            capability: lifecycle_capability,
            quality: event.evidence.correlation_quality,
            observed_at_ms: event.timestamp_ms,
            limitation: None,
            invalidated_by_gap: false,
        },
        FieldProvenance {
            field: EvidenceField::RemoteEndpoint,
            source,
            capability: lifecycle_capability,
            quality: event.evidence.correlation_quality,
            observed_at_ms: event.timestamp_ms,
            limitation: None,
            invalidated_by_gap: false,
        },
    ];
    if event.key.pid.is_some() || event.key.process_instance.is_some() {
        provenance.push(FieldProvenance {
            field: EvidenceField::Process,
            source,
            capability: CaptureCapabilityKind::ProcessIdentity,
            quality: event.evidence.correlation_quality,
            observed_at_ms: event.timestamp_ms,
            limitation: None,
            invalidated_by_gap: false,
        });
    }
    if event.interface.is_some() || event.key.interface_id.is_some() {
        provenance.push(FieldProvenance {
            field: EvidenceField::Interface,
            source,
            capability: CaptureCapabilityKind::Interface,
            quality: event.evidence.correlation_quality,
            observed_at_ms: event.timestamp_ms,
            limitation: None,
            invalidated_by_gap: false,
        });
    }
    if event.traffic_delta != Default::default() {
        provenance.push(FieldProvenance {
            field: EvidenceField::Counters,
            source,
            capability: CaptureCapabilityKind::FlowBytes,
            quality: event.evidence.correlation_quality,
            observed_at_ms: event.timestamp_ms,
            limitation: None,
            invalidated_by_gap: false,
        });
    }
    provenance
}

#[derive(Default)]
pub struct ProviderSequenceTracker {
    last_sequence: HashMap<(String, u64), u64>,
}

impl ProviderSequenceTracker {
    pub fn observe(
        &mut self,
        cursor: &ProviderCursor,
        detected_at_ms: i64,
        affected_capabilities: BTreeSet<CaptureCapabilityKind>,
    ) -> Option<ProviderGap> {
        let key = (cursor.provider_id.clone(), cursor.generation);
        let previous = self.last_sequence.insert(key, cursor.sequence);
        let expected = previous?.saturating_add(1);
        (cursor.sequence > expected).then(|| ProviderGap {
            provider_id: cursor.provider_id.clone(),
            generation: cursor.generation,
            first_missing_sequence: expected,
            last_missing_sequence: cursor.sequence.saturating_sub(1),
            detected_at_ms,
            affected_capabilities,
            reason: "provider sequence discontinuity".into(),
            reconciled_at_ms: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sabicheiro_core::{
        CapabilityAvailability, ConnectionEvidence, ConnectionKey, ConnectionState, CounterSource,
        Direction, Endpoint, NetworkEventKind, NetworkProtocol, ProviderCapability, ProviderMode,
        TrafficCounters,
    };

    fn descriptor() -> ProviderDescriptor {
        ProviderDescriptor {
            protocol_version: 1,
            provider_id: "fixture".into(),
            display_name: "Fixture".into(),
            mode: ProviderMode::Fixture,
            generation: 3,
            capabilities: vec![ProviderCapability {
                kind: CaptureCapabilityKind::LifecycleEvents,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::Demo,
                limitation: None,
            }],
        }
    }

    fn event(at_ms: i64) -> NormalizedNetworkEvent {
        let remote = Endpoint::new("127.0.0.1".parse().expect("IP"), 443);
        NormalizedNetworkEvent {
            kind: NetworkEventKind::Opened,
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
                at_ms,
            ),
            timestamp_ms: at_ms,
            process: Default::default(),
            direction: Direction::Outbound,
            state: ConnectionState::Established,
            interface: Some("loopback".into()),
            hostname: None,
            domain: None,
            geo: None,
            asn: None,
            traffic_delta: TrafficCounters::default(),
            counter_source: CounterSource::Unavailable,
        }
    }

    #[test]
    fn monotonic_order_does_not_regress_when_utc_or_input_clock_moves() {
        let mut sequencer = EvidenceSequencer::new(descriptor(), 1_000);
        let first = sequencer.wrap_network_at(100, 1_000, event(1_000));
        let second = sequencer.wrap_network_at(50, 900, event(900));
        assert_eq!(first.cursor.sequence, 1);
        assert_eq!(second.cursor.sequence, 2);
        assert_eq!(second.clock.monotonic_ns, 100);
        assert_eq!(second.clock.utc_ms, 900);
    }

    #[test]
    fn explicit_loss_advances_sequence_and_is_detectable() {
        let mut sequencer = EvidenceSequencer::new(descriptor(), 1_000);
        let first = sequencer.wrap_network_at(1, 1_000, event(1_000));
        let gap = sequencer
            .record_gap(
                2,
                2,
                1_001,
                [CaptureCapabilityKind::LifecycleEvents],
                "fixture overflow",
            )
            .expect("gap");
        assert_eq!(first.cursor.sequence, 1);
        assert_eq!(gap.gap.as_ref().expect("gap payload").missing_count(), 2);
        assert_eq!(gap.cursor.sequence, 4);
        assert_eq!(gap.cursor.dropped_before, 2);
    }

    #[test]
    fn tracker_keeps_provider_generations_independent() {
        let mut tracker = ProviderSequenceTracker::default();
        let capabilities = BTreeSet::from([CaptureCapabilityKind::LifecycleEvents]);
        let first = ProviderCursor {
            provider_id: "fixture".into(),
            generation: 1,
            sequence: 1,
            watermark_monotonic_ns: None,
            dropped_before: 0,
        };
        assert!(tracker.observe(&first, 1, capabilities.clone()).is_none());
        let mut third = first.clone();
        third.sequence = 3;
        let gap = tracker
            .observe(&third, 2, capabilities.clone())
            .expect("sequence gap");
        assert_eq!(gap.first_missing_sequence, 2);
        let mut restarted = first;
        restarted.generation = 2;
        assert!(tracker.observe(&restarted, 3, capabilities).is_none());
    }
}
