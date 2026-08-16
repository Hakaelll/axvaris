use crate::{CorrelationQuality, DnsAnswerEvidence, NormalizedNetworkEvent, ObservationSource};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Version of the immutable observation envelope written to the evidence journal.
pub const EVIDENCE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureCapabilityKind {
    LifecycleEvents,
    SocketSnapshot,
    FlowBytes,
    FlowPackets,
    Dns,
    ProcessIdentity,
    Interface,
    Route,
    NetworkNamespace,
    HandshakeMetadata,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAvailability {
    Available,
    Degraded,
    #[default]
    Unavailable,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapability {
    pub kind: CaptureCapabilityKind,
    pub availability: CapabilityAvailability,
    pub source: ObservationSource,
    pub limitation: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMode {
    EventStream,
    Snapshot,
    Hybrid,
    Fixture,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub protocol_version: u16,
    pub provider_id: String,
    pub display_name: String,
    pub mode: ProviderMode,
    pub generation: u64,
    pub capabilities: Vec<ProviderCapability>,
}

impl ProviderDescriptor {
    pub fn supports(&self, kind: CaptureCapabilityKind) -> bool {
        self.capabilities.iter().any(|capability| {
            capability.kind == kind
                && matches!(
                    capability.availability,
                    CapabilityAvailability::Available | CapabilityAvailability::Degraded
                )
        })
    }

    pub fn available_capabilities(&self) -> BTreeSet<CaptureCapabilityKind> {
        self.capabilities
            .iter()
            .filter(|capability| {
                matches!(
                    capability.availability,
                    CapabilityAvailability::Available | CapabilityAvailability::Degraded
                )
            })
            .map(|capability| capability.kind)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockQuality {
    Anchored,
    Estimated,
    #[default]
    Unknown,
}

/// A dual timestamp. Ordering is always performed with `monotonic_ns`; UTC is only
/// for presentation and cross-process anchoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceClock {
    pub clock_id: String,
    pub monotonic_ns: u64,
    pub utc_ms: i64,
    pub quality: ClockQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCursor {
    pub provider_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub watermark_monotonic_ns: Option<u64>,
    pub dropped_before: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderLifecycleState {
    Starting,
    Healthy,
    Degraded,
    Restarting,
    Fallback,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub generation: u64,
    pub state: ProviderLifecycleState,
    pub since_ms: i64,
    pub reason: Option<String>,
    pub fallback_provider_id: Option<String>,
    pub restart_attempt: u32,
    pub next_retry_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderGap {
    pub provider_id: String,
    pub generation: u64,
    pub first_missing_sequence: u64,
    pub last_missing_sequence: u64,
    pub detected_at_ms: i64,
    pub affected_capabilities: BTreeSet<CaptureCapabilityKind>,
    pub reason: String,
    pub reconciled_at_ms: Option<i64>,
}

impl ProviderGap {
    pub fn missing_count(&self) -> u64 {
        self.last_missing_sequence
            .saturating_sub(self.first_missing_sequence)
            .saturating_add(1)
    }

    pub fn affects(&self, capability: CaptureCapabilityKind) -> bool {
        self.affected_capabilities.contains(&capability)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceField {
    Lifecycle,
    LocalEndpoint,
    RemoteEndpoint,
    Process,
    Interface,
    Route,
    NetworkNamespace,
    Dns,
    Counters,
    Handshake,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldProvenance {
    pub field: EvidenceField,
    pub source: ObservationSource,
    pub capability: CaptureCapabilityKind,
    pub quality: CorrelationQuality,
    pub observed_at_ms: i64,
    pub limitation: Option<String>,
    pub invalidated_by_gap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    NetworkLifecycle,
    NetworkCounters,
    Dns,
    Process,
    Route,
    Listener,
    Handshake,
    ProviderHealth,
    ProviderGap,
    UserAnnotation,
}

/// Immutable journal unit. `payload` is reserved for kind-specific evidence while
/// `network` keeps the existing typed projection boundary backward compatible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceObservation {
    pub schema_version: u16,
    pub id: String,
    pub cursor: ProviderCursor,
    pub clock: EvidenceClock,
    pub kind: ObservationKind,
    pub network: Option<NormalizedNetworkEvent>,
    #[serde(default)]
    pub dns: Option<DnsAnswerEvidence>,
    pub gap: Option<ProviderGap>,
    pub payload: Value,
    pub provenance: BTreeMap<EvidenceField, FieldProvenance>,
}

impl EvidenceObservation {
    pub fn network(
        cursor: ProviderCursor,
        clock: EvidenceClock,
        event: NormalizedNetworkEvent,
        provenance: impl IntoIterator<Item = FieldProvenance>,
    ) -> Self {
        let kind = if event.traffic_delta == Default::default() {
            ObservationKind::NetworkLifecycle
        } else {
            ObservationKind::NetworkCounters
        };
        Self {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            id: observation_id(&cursor, kind),
            cursor,
            clock,
            kind,
            network: Some(event),
            dns: None,
            gap: None,
            payload: Value::Null,
            provenance: provenance
                .into_iter()
                .map(|item| (item.field, item))
                .collect(),
        }
    }

    pub fn dns(
        cursor: ProviderCursor,
        clock: EvidenceClock,
        dns: DnsAnswerEvidence,
        provenance: FieldProvenance,
    ) -> Self {
        Self {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            id: observation_id(&cursor, ObservationKind::Dns),
            cursor,
            clock,
            kind: ObservationKind::Dns,
            network: None,
            dns: Some(dns),
            gap: None,
            payload: Value::Null,
            provenance: BTreeMap::from([(EvidenceField::Dns, provenance)]),
        }
    }

    pub fn gap(cursor: ProviderCursor, clock: EvidenceClock, gap: ProviderGap) -> Self {
        Self {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            id: observation_id(&cursor, ObservationKind::ProviderGap),
            cursor,
            clock,
            kind: ObservationKind::ProviderGap,
            network: None,
            dns: None,
            gap: Some(gap),
            payload: Value::Null,
            provenance: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), EvidenceValidationError> {
        if self.schema_version != EVIDENCE_SCHEMA_VERSION {
            return Err(EvidenceValidationError::UnsupportedSchema(
                self.schema_version,
            ));
        }
        if self.cursor.provider_id.trim().is_empty() || self.id.trim().is_empty() {
            return Err(EvidenceValidationError::MissingIdentity);
        }
        if self.cursor.generation == 0 || self.cursor.sequence == 0 {
            return Err(EvidenceValidationError::InvalidCursor);
        }
        match self.kind {
            ObservationKind::NetworkLifecycle | ObservationKind::NetworkCounters
                if self.network.is_none() =>
            {
                Err(EvidenceValidationError::MissingPayload)
            }
            ObservationKind::ProviderGap if self.gap.is_none() => {
                Err(EvidenceValidationError::MissingPayload)
            }
            ObservationKind::Dns if self.dns.is_none() => {
                Err(EvidenceValidationError::MissingPayload)
            }
            _ => Ok(()),
        }
    }

    pub fn invalidate_for_gap(&mut self, gap: &ProviderGap) {
        if self.cursor.provider_id != gap.provider_id || self.cursor.generation != gap.generation {
            return;
        }
        for provenance in self.provenance.values_mut() {
            if gap.affects(provenance.capability) {
                provenance.invalidated_by_gap = true;
                provenance.quality = CorrelationQuality::Unavailable;
                provenance.limitation = Some(format!(
                    "provider gap {}-{}: {}",
                    gap.first_missing_sequence, gap.last_missing_sequence, gap.reason
                ));
            }
        }
    }
}

fn observation_id(cursor: &ProviderCursor, kind: ObservationKind) -> String {
    format!(
        "{}:{}:{}:{kind:?}",
        cursor.provider_id, cursor.generation, cursor.sequence
    )
    .to_ascii_lowercase()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceValidationError {
    UnsupportedSchema(u16),
    MissingIdentity,
    InvalidCursor,
    MissingPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EvidenceQuery {
    pub from_utc_ms: Option<i64>,
    pub to_utc_ms: Option<i64>,
    pub session_id: Option<String>,
    pub provider_id: Option<String>,
    pub process_instance: Option<String>,
    pub remote_ip: Option<String>,
    pub kind: Option<ObservationKind>,
    pub after_monotonic_ns: Option<u64>,
    pub limit: usize,
}

impl Default for EvidenceQuery {
    fn default() -> Self {
        Self {
            from_utc_ms: None,
            to_utc_ms: None,
            session_id: None,
            provider_id: None,
            process_instance: None,
            remote_ip: None,
            kind: None,
            after_monotonic_ns: None,
            limit: 1_000,
        }
    }
}

impl EvidenceQuery {
    pub fn bounded_limit(&self) -> usize {
        self.limit.clamp(1, 100_000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionTier {
    DetailedEvidence,
    Connections,
    TrafficSamples,
    DailyRollups,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierRetention {
    pub tier: RetentionTier,
    pub days: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionEstimate {
    pub cutoff_utc_ms: i64,
    pub rows_by_tier: BTreeMap<RetentionTier, u64>,
    pub estimated_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionBatchResult {
    pub removed_by_tier: BTreeMap<RetentionTier, u64>,
    pub remaining_eligible_rows: u64,
    pub cancelled: bool,
    pub compact_recommended: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceJournalStats {
    pub observations: u64,
    pub first_utc_ms: Option<i64>,
    pub last_utc_ms: Option<i64>,
    pub providers: usize,
    pub gaps: u64,
    pub bytes_on_disk: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub source_path: String,
    pub destination_path: String,
    pub backup_path: String,
    pub recovered_rows: u64,
    pub lost_rows: u64,
    pub recovered_tables: Vec<String>,
    pub skipped_tables: Vec<String>,
    pub gaps_marked: u64,
    pub quick_check_ok: bool,
    pub detail: String,
}

impl std::fmt::Display for EvidenceValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported evidence schema {version}")
            }
            Self::MissingIdentity => formatter.write_str("missing observation identity"),
            Self::InvalidCursor => {
                formatter.write_str("provider generation/sequence must be non-zero")
            }
            Self::MissingPayload => {
                formatter.write_str("observation payload does not match its kind")
            }
        }
    }
}

impl std::error::Error for EvidenceValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConnectionEvidence, ConnectionKey, ConnectionState, CounterSource, Direction, Endpoint,
        NetworkEventKind, NetworkProtocol, ProcessInfo, TrafficCounters,
    };

    fn event() -> NormalizedNetworkEvent {
        let local = Endpoint::new("127.0.0.1".parse().expect("local IP"), 41_000);
        let remote = Endpoint::new("127.0.0.1".parse().expect("remote IP"), 443);
        NormalizedNetworkEvent {
            kind: NetworkEventKind::Opened,
            key: ConnectionKey::new(Some(7), local, remote.clone(), NetworkProtocol::Tcp),
            evidence: ConnectionEvidence::for_peer(
                &remote,
                ConnectionState::Established,
                ObservationSource::Demo,
                10,
            ),
            timestamp_ms: 10,
            process: ProcessInfo::default(),
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

    fn cursor(sequence: u64) -> ProviderCursor {
        ProviderCursor {
            provider_id: "fixture".into(),
            generation: 1,
            sequence,
            watermark_monotonic_ns: Some(100),
            dropped_before: 0,
        }
    }

    #[test]
    fn observation_identity_and_serialization_are_deterministic() {
        let provenance = FieldProvenance {
            field: EvidenceField::Lifecycle,
            source: ObservationSource::Demo,
            capability: CaptureCapabilityKind::LifecycleEvents,
            quality: CorrelationQuality::Confirmed,
            observed_at_ms: 10,
            limitation: None,
            invalidated_by_gap: false,
        };
        let observation = EvidenceObservation::network(
            cursor(1),
            EvidenceClock {
                clock_id: "fixture-clock".into(),
                monotonic_ns: 100,
                utc_ms: 10,
                quality: ClockQuality::Anchored,
            },
            event(),
            [provenance],
        );
        observation.validate().expect("valid observation");
        let first = serde_json::to_string(&observation).expect("serialize");
        let second = serde_json::to_string(&observation).expect("serialize again");
        assert_eq!(first, second);
        assert_eq!(observation.id, "fixture:1:1:networklifecycle");
    }

    #[test]
    fn a_gap_invalidates_only_fields_from_affected_capabilities() {
        let lifecycle = FieldProvenance {
            field: EvidenceField::Lifecycle,
            source: ObservationSource::WindowsEtw,
            capability: CaptureCapabilityKind::LifecycleEvents,
            quality: CorrelationQuality::Confirmed,
            observed_at_ms: 10,
            limitation: None,
            invalidated_by_gap: false,
        };
        let process = FieldProvenance {
            field: EvidenceField::Process,
            source: ObservationSource::WindowsIpHelper,
            capability: CaptureCapabilityKind::ProcessIdentity,
            quality: CorrelationQuality::Confirmed,
            observed_at_ms: 10,
            limitation: None,
            invalidated_by_gap: false,
        };
        let mut observation = EvidenceObservation::network(
            cursor(2),
            EvidenceClock {
                clock_id: "clock".into(),
                monotonic_ns: 200,
                utc_ms: 10,
                quality: ClockQuality::Anchored,
            },
            event(),
            [lifecycle, process],
        );
        observation.invalidate_for_gap(&ProviderGap {
            provider_id: "fixture".into(),
            generation: 1,
            first_missing_sequence: 1,
            last_missing_sequence: 1,
            detected_at_ms: 11,
            affected_capabilities: BTreeSet::from([CaptureCapabilityKind::LifecycleEvents]),
            reason: "overflow".into(),
            reconciled_at_ms: None,
        });
        assert!(observation.provenance[&EvidenceField::Lifecycle].invalidated_by_gap);
        assert!(!observation.provenance[&EvidenceField::Process].invalidated_by_gap);
    }
}
