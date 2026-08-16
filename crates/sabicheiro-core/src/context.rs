use crate::{
    CorrelationQuality, Endpoint, NetworkProtocol, NormalizedNetworkEvent, ObservationSource,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    net::IpAddr,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPathKind {
    Direct,
    Gateway,
    Vpn,
    SystemProxy,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkProfile {
    pub id: String,
    pub display_name: Option<String>,
    pub vpn_observed: bool,
    pub created_at_ms: i64,
    pub sensitive_fields_redacted: bool,
}

impl NetworkProfile {
    /// Derives a stable local pseudonym and deliberately discards the sensitive input.
    pub fn pseudonymous(
        sensitive_signature: &str,
        display_name: Option<String>,
        vpn_observed: bool,
        created_at_ms: i64,
    ) -> Self {
        let digest = Sha256::digest(sensitive_signature.as_bytes());
        Self {
            id: format!("network-{}", &format!("{digest:x}")[..24]),
            display_name,
            vpn_observed,
            created_at_ms,
            sensitive_fields_redacted: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPathEvidence {
    pub id: String,
    pub destination_prefix: String,
    pub prefix_length: u8,
    pub gateway_pseudonym: Option<String>,
    pub interface_id: Option<String>,
    pub metric: Option<u32>,
    pub network_generation: u64,
    pub profile_id: String,
    pub kind: NetworkPathKind,
    pub observed_at_ms: i64,
    pub source: ObservationSource,
    pub quality: CorrelationQuality,
    pub limitation: Option<String>,
}

impl NetworkPathEvidence {
    pub fn redact_gateway(gateway: Option<IpAddr>) -> Option<String> {
        gateway.map(|address| {
            let digest = Sha256::digest(address.to_string().as_bytes());
            format!("gateway-{}", &format!("{digest:x}")[..16])
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsAnswerEvidence {
    pub id: String,
    pub query_name: String,
    pub canonical_name: Option<String>,
    pub addresses: Vec<IpAddr>,
    pub ttl_seconds: u32,
    pub resolver: Option<String>,
    pub process_instance: Option<String>,
    pub observed_at_ms: i64,
    pub expires_at_ms: i64,
    pub source: ObservationSource,
}

impl DnsAnswerEvidence {
    pub fn new(
        id: impl Into<String>,
        query_name: impl Into<String>,
        addresses: impl IntoIterator<Item = IpAddr>,
        ttl_seconds: u32,
        process_instance: Option<String>,
        observed_at_ms: i64,
        source: ObservationSource,
    ) -> Self {
        let mut addresses = addresses.into_iter().collect::<Vec<_>>();
        addresses.sort();
        addresses.dedup();
        let ttl_seconds = ttl_seconds.clamp(1, 86_400);
        Self {
            id: id.into(),
            query_name: query_name.into().trim_end_matches('.').to_ascii_lowercase(),
            canonical_name: None,
            addresses,
            ttl_seconds,
            resolver: None,
            process_instance,
            observed_at_ms,
            expires_at_ms: observed_at_ms.saturating_add(i64::from(ttl_seconds) * 1_000),
            source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsCausalCandidate {
    pub answer_id: String,
    pub query_name: String,
    pub quality: CorrelationQuality,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsCausalRelation {
    pub confirmed: Option<DnsCausalCandidate>,
    pub candidates: Vec<DnsCausalCandidate>,
    pub limitation: Option<String>,
}

pub struct DnsCausalGraph {
    answers_by_address: BTreeMap<IpAddr, VecDeque<DnsAnswerEvidence>>,
    capacity_per_address: usize,
}

impl DnsCausalGraph {
    pub fn with_capacity_per_address(capacity: usize) -> Self {
        Self {
            answers_by_address: BTreeMap::new(),
            capacity_per_address: capacity.clamp(1, 1_024),
        }
    }

    pub fn observe(&mut self, answer: DnsAnswerEvidence) {
        for address in &answer.addresses {
            let entries = self.answers_by_address.entry(*address).or_default();
            if !entries.iter().any(|existing| existing.id == answer.id) {
                entries.push_back(answer.clone());
            }
            while entries.len() > self.capacity_per_address {
                entries.pop_front();
            }
        }
    }

    pub fn relate(&mut self, event: &NormalizedNetworkEvent) -> DnsCausalRelation {
        let Some(entries) = self.answers_by_address.get_mut(&event.key.remote.address) else {
            return DnsCausalRelation {
                limitation: Some(
                    "no visible preceding DNS answer; cache, DoH or literal IP may apply".into(),
                ),
                ..DnsCausalRelation::default()
            };
        };
        entries.retain(|answer| answer.expires_at_ms >= event.timestamp_ms);
        let inferred_instance = event.process.instance_id();
        let process_instance = event
            .key
            .process_instance
            .as_deref()
            .or(inferred_instance.as_deref());
        let mut exact = Vec::new();
        let mut candidates = Vec::new();
        for answer in entries.iter().filter(|answer| {
            answer.observed_at_ms <= event.timestamp_ms
                && answer.expires_at_ms >= event.timestamp_ms
        }) {
            let same_process = answer.source != ObservationSource::LocalDns
                && answer
                    .process_instance
                    .as_deref()
                    .zip(process_instance)
                    .is_some_and(|(answer_process, event_process)| answer_process == event_process);
            let candidate = DnsCausalCandidate {
                answer_id: answer.id.clone(),
                query_name: answer.query_name.clone(),
                quality: if same_process {
                    CorrelationQuality::Confirmed
                } else {
                    CorrelationQuality::Candidate
                },
                reason: if same_process {
                    "same process instance, address and valid TTL window".into()
                } else {
                    "address and TTL match, but process causality is unavailable or shared".into()
                },
            };
            if same_process {
                exact.push(candidate);
            } else {
                candidates.push(candidate);
            }
        }
        exact.sort_by(|left, right| left.answer_id.cmp(&right.answer_id));
        candidates.sort_by(|left, right| left.answer_id.cmp(&right.answer_id));
        if exact.len() == 1 {
            DnsCausalRelation {
                confirmed: exact.pop(),
                candidates,
                limitation: None,
            }
        } else {
            candidates.extend(exact.into_iter().map(|mut candidate| {
                candidate.quality = CorrelationQuality::Candidate;
                candidate.reason = "multiple same-process DNS answers remain compatible".into();
                candidate
            }));
            candidates.sort_by(|left, right| left.answer_id.cmp(&right.answer_id));
            DnsCausalRelation {
                confirmed: None,
                candidates,
                limitation: Some(
                    "shared caches, CDN reuse and Happy Eyeballs can leave multiple candidates"
                        .into(),
                ),
            }
        }
    }
}

impl Default for DnsCausalGraph {
    fn default() -> Self {
        Self::with_capacity_per_address(64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeMetadata {
    pub tls_version: Option<String>,
    pub sni: Option<String>,
    pub alpn: Vec<String>,
    pub quic_version: Option<u32>,
    pub bytes_examined: usize,
    pub observed_at_ms: i64,
    pub source: ObservationSource,
    pub persisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    #[error("handshake metadata mode is disabled")]
    Disabled,
    #[error("handshake exceeds the bounded metadata budget")]
    BudgetExceeded,
    #[error("truncated or unsupported TLS ClientHello")]
    InvalidClientHello,
}

pub struct EphemeralHandshakeBuffer {
    bytes: Vec<u8>,
    maximum_bytes: usize,
}

impl EphemeralHandshakeBuffer {
    pub fn new(bytes: Vec<u8>, maximum_bytes: usize) -> Result<Self, HandshakeError> {
        let maximum_bytes = maximum_bytes.clamp(1, 64 * 1024);
        if bytes.len() > maximum_bytes {
            return Err(HandshakeError::BudgetExceeded);
        }
        Ok(Self {
            bytes,
            maximum_bytes,
        })
    }

    pub fn inspect_tls(
        &self,
        enabled: bool,
        observed_at_ms: i64,
    ) -> Result<HandshakeMetadata, HandshakeError> {
        if !enabled {
            return Err(HandshakeError::Disabled);
        }
        parse_client_hello(&self.bytes, self.maximum_bytes, observed_at_ms)
    }

    pub fn clear(&mut self) {
        self.bytes.fill(0);
        self.bytes.clear();
    }
}

impl Drop for EphemeralHandshakeBuffer {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

fn parse_client_hello(
    bytes: &[u8],
    maximum_bytes: usize,
    observed_at_ms: i64,
) -> Result<HandshakeMetadata, HandshakeError> {
    if bytes.len() > maximum_bytes || bytes.len() < 9 || bytes[0] != 22 || bytes[5] != 1 {
        return Err(HandshakeError::InvalidClientHello);
    }
    let record_len = usize::from(u16::from_be_bytes([bytes[3], bytes[4]]));
    if record_len + 5 > bytes.len() {
        return Err(HandshakeError::InvalidClientHello);
    }
    let mut offset = 9;
    let legacy_version = take_u16(bytes, &mut offset)?;
    take(bytes, &mut offset, 32)?;
    let session_len = usize::from(take(bytes, &mut offset, 1)?[0]);
    take(bytes, &mut offset, session_len)?;
    let cipher_len = usize::from(take_u16(bytes, &mut offset)?);
    take(bytes, &mut offset, cipher_len)?;
    let compression_len = usize::from(take(bytes, &mut offset, 1)?[0]);
    take(bytes, &mut offset, compression_len)?;
    let extensions_len = usize::from(take_u16(bytes, &mut offset)?);
    let extensions_end = offset
        .checked_add(extensions_len)
        .filter(|end| *end <= bytes.len())
        .ok_or(HandshakeError::InvalidClientHello)?;
    let mut sni = None;
    let mut alpn = Vec::new();
    let mut version = legacy_version;
    while offset + 4 <= extensions_end {
        let kind = take_u16(bytes, &mut offset)?;
        let length = usize::from(take_u16(bytes, &mut offset)?);
        let extension = take(bytes, &mut offset, length)?;
        match kind {
            0 if extension.len() >= 5 => {
                let length = usize::from(u16::from_be_bytes([extension[3], extension[4]]));
                if 5 + length <= extension.len() {
                    sni = std::str::from_utf8(&extension[5..5 + length])
                        .ok()
                        .map(|name| name.to_ascii_lowercase());
                }
            }
            16 if extension.len() >= 3 => {
                let mut index = 2;
                while index < extension.len() {
                    let length = usize::from(extension[index]);
                    index += 1;
                    if index + length > extension.len() {
                        break;
                    }
                    if let Ok(value) = std::str::from_utf8(&extension[index..index + length]) {
                        alpn.push(value.to_owned());
                    }
                    index += length;
                }
            }
            43 if extension.len() >= 3 => {
                let length = usize::from(extension[0]);
                for item in extension[1..].chunks_exact(2).take(length / 2) {
                    version = version.max(u16::from_be_bytes([item[0], item[1]]));
                }
            }
            _ => {}
        }
    }
    let tls_version = match version {
        0x0304 => "TLS 1.3",
        0x0303 => "TLS 1.2",
        0x0302 => "TLS 1.1",
        0x0301 => "TLS 1.0",
        _ => "unknown TLS version",
    };
    Ok(HandshakeMetadata {
        tls_version: Some(tls_version.into()),
        sni,
        alpn,
        quic_version: None,
        bytes_examined: bytes.len(),
        observed_at_ms,
        source: ObservationSource::HandshakeMetadata,
        persisted: false,
    })
}

fn take<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    length: usize,
) -> Result<&'a [u8], HandshakeError> {
    let end = offset
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .ok_or(HandshakeError::InvalidClientHello)?;
    let value = &bytes[*offset..end];
    *offset = end;
    Ok(value)
}

fn take_u16(bytes: &[u8], offset: &mut usize) -> Result<u16, HandshakeError> {
    let value = take(bytes, offset, 2)?;
    Ok(u16::from_be_bytes([value[0], value[1]]))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenerScope {
    Loopback,
    Lan,
    Vpn,
    Wildcard,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirewallEvidence {
    Allowed,
    Blocked,
    Ambiguous,
    #[default]
    NotQueryable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListenerExposure {
    pub endpoint: Endpoint,
    pub protocol: NetworkProtocol,
    pub process_instance: Option<String>,
    pub scope: ListenerScope,
    pub firewall: FirewallEvidence,
    pub first_observed_ms: i64,
    pub last_observed_ms: i64,
    pub limitation: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TodayCardState {
    pub id: String,
    pub dismissed: bool,
    pub pinned: bool,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ListenerSnapshot {
    pub id: Option<i64>,
    pub session_id: Option<String>,
    pub observed_at_ms: i64,
    pub snapshot_sha256: String,
    pub listeners: Vec<ListenerExposure>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HandshakeModeAudit {
    pub occurred_at_ms: i64,
    pub enabled: bool,
    pub maximum_bytes: u32,
    pub maximum_packets: u32,
    pub duration_limit_ms: u64,
    pub detail: String,
}

impl ListenerExposure {
    pub fn classify(
        endpoint: Endpoint,
        protocol: NetworkProtocol,
        process_instance: Option<String>,
        interface_is_vpn: bool,
        observed_at_ms: i64,
    ) -> Self {
        let scope = if endpoint.address.is_loopback() {
            ListenerScope::Loopback
        } else if endpoint.address.is_unspecified() {
            ListenerScope::Wildcard
        } else if interface_is_vpn {
            ListenerScope::Vpn
        } else if !endpoint.scope.is_global() {
            ListenerScope::Lan
        } else {
            ListenerScope::Unknown
        };
        Self {
            endpoint,
            protocol,
            process_instance,
            scope,
            firewall: FirewallEvidence::NotQueryable,
            first_observed_ms: observed_at_ms,
            last_observed_ms: observed_at_ms,
            limitation: "bind scope is local evidence; it does not prove Internet reachability"
                .into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentitySnapshot {
    pub profile_id: String,
    pub process_revision: String,
    pub domains: BTreeSet<String>,
    pub prefixes: BTreeSet<String>,
    pub asns: BTreeSet<u32>,
    pub countries: BTreeSet<String>,
    pub route_ids: BTreeSet<String>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityChange {
    pub field: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub comparable: bool,
    pub reason: String,
}

pub fn compare_identity(left: &IdentitySnapshot, right: &IdentitySnapshot) -> Vec<IdentityChange> {
    if left.profile_id != right.profile_id {
        return vec![IdentityChange {
            field: "network_profile".into(),
            before: vec![left.profile_id.clone()],
            after: vec![right.profile_id.clone()],
            comparable: false,
            reason: "different network profiles are not treated as one baseline".into(),
        }];
    }
    let mut changes = Vec::new();
    push_change(
        &mut changes,
        "process_revision",
        BTreeSet::from([left.process_revision.clone()]),
        BTreeSet::from([right.process_revision.clone()]),
    );
    push_change(
        &mut changes,
        "domain",
        left.domains.clone(),
        right.domains.clone(),
    );
    push_change(
        &mut changes,
        "prefix",
        left.prefixes.clone(),
        right.prefixes.clone(),
    );
    push_change(
        &mut changes,
        "asn",
        left.asns.iter().map(ToString::to_string).collect(),
        right.asns.iter().map(ToString::to_string).collect(),
    );
    push_change(
        &mut changes,
        "country",
        left.countries.clone(),
        right.countries.clone(),
    );
    push_change(
        &mut changes,
        "route",
        left.route_ids.clone(),
        right.route_ids.clone(),
    );
    changes
}

fn push_change(
    changes: &mut Vec<IdentityChange>,
    field: &str,
    before: BTreeSet<String>,
    after: BTreeSet<String>,
) {
    if before != after {
        changes.push(IdentityChange {
            field: field.into(),
            before: before.into_iter().collect(),
            after: after.into_iter().collect(),
            comparable: true,
            reason: "observed values changed within the same local network profile".into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConnectionEvidence, ConnectionKey, ConnectionState, CounterSource, Direction,
        NetworkEventKind, TrafficCounters,
    };

    fn event(process: Option<&str>, at_ms: i64) -> NormalizedNetworkEvent {
        let remote = Endpoint::new("203.0.113.10".parse().expect("IP"), 443);
        let mut key = ConnectionKey::new(
            Some(7),
            Endpoint::new("192.0.2.10".parse().expect("IP"), 50_000),
            remote.clone(),
            NetworkProtocol::Tcp,
        );
        key.process_instance = process.map(str::to_owned);
        NormalizedNetworkEvent {
            kind: NetworkEventKind::Opened,
            key,
            evidence: ConnectionEvidence::for_peer(
                &remote,
                crate::ConnectionState::Established,
                ObservationSource::WindowsEtw,
                at_ms,
            ),
            timestamp_ms: at_ms,
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
        }
    }

    #[test]
    fn dns_requires_one_same_process_answer_for_confirmation() {
        let mut graph = DnsCausalGraph::default();
        graph.observe(DnsAnswerEvidence::new(
            "answer-a",
            "example.test",
            ["203.0.113.10".parse().expect("IP")],
            60,
            Some("process-a".into()),
            1_000,
            ObservationSource::WindowsEtw,
        ));
        graph.observe(DnsAnswerEvidence::new(
            "answer-shared",
            "cdn.test",
            ["203.0.113.10".parse().expect("IP")],
            60,
            None,
            1_001,
            ObservationSource::WindowsEtw,
        ));
        let relation = graph.relate(&event(Some("process-a"), 2_000));
        assert_eq!(
            relation
                .confirmed
                .as_ref()
                .map(|item| item.answer_id.as_str()),
            Some("answer-a")
        );
        assert_eq!(relation.candidates.len(), 1);
        assert!(graph.relate(&event(None, 2_000)).confirmed.is_none());
    }

    #[test]
    fn listener_wildcard_never_claims_external_reachability() {
        let exposure = ListenerExposure::classify(
            Endpoint::new("0.0.0.0".parse().expect("IP"), 8080),
            NetworkProtocol::Tcp,
            None,
            false,
            1,
        );
        assert_eq!(exposure.scope, ListenerScope::Wildcard);
        assert_eq!(exposure.firewall, FirewallEvidence::NotQueryable);
        assert!(exposure.limitation.contains("does not prove"));
    }

    #[test]
    fn network_profile_discards_sensitive_signature() {
        let profile = NetworkProfile::pseudonymous("ssid|10.0.0.1|wifi", None, false, 1);
        assert!(!profile.id.contains("10.0.0.1"));
        assert!(profile.sensitive_fields_redacted);
    }

    #[test]
    fn handshake_buffers_are_bounded_and_disabled_by_default() {
        let mut buffer = EphemeralHandshakeBuffer::new(vec![0; 16], 64).expect("buffer");
        assert_eq!(buffer.inspect_tls(false, 1), Err(HandshakeError::Disabled));
        buffer.clear();
        assert!(buffer.bytes.is_empty());
    }

    #[test]
    fn different_profiles_are_not_compared_as_one_baseline() {
        let snapshot = |profile: &str| IdentitySnapshot {
            profile_id: profile.into(),
            process_revision: "revision".into(),
            domains: BTreeSet::new(),
            prefixes: BTreeSet::new(),
            asns: BTreeSet::new(),
            countries: BTreeSet::new(),
            route_ids: BTreeSet::new(),
            observed_at_ms: 1,
        };
        let changes = compare_identity(&snapshot("home"), &snapshot("vpn"));
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].comparable);
    }
}
