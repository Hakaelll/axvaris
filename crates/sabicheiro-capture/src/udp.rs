use sabicheiro_core::Endpoint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_UDP_IDLE_TIMEOUT_NS: u64 = 30_000_000_000;
const MAX_QUIC_CONNECTION_ID_BYTES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuicPacketKind {
    Initial,
    ZeroRtt,
    Handshake,
    Retry,
    VersionNegotiation,
    OtherLongHeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuicMetadata {
    pub version: u32,
    pub packet_kind: QuicPacketKind,
    pub destination_connection_id: String,
    pub source_connection_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "quality", rename_all = "snake_case")]
pub enum QuicEvidence {
    None,
    PortHint { port: u16 },
    Handshake { metadata: QuicMetadata },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UdpConversation {
    pub id: String,
    pub pid: Option<u32>,
    pub process_instance: Option<String>,
    pub local: Endpoint,
    pub remote: Endpoint,
    pub first_seen_monotonic_ns: u64,
    pub last_seen_monotonic_ns: u64,
    pub datagrams: u64,
    pub route_generation: u64,
    pub quic: QuicEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UdpConversationEventKind {
    Opened,
    Activity,
    Migrated,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UdpConversationEvent {
    pub kind: UdpConversationEventKind,
    pub conversation: UdpConversation,
    pub previous_local: Option<Endpoint>,
    pub previous_remote: Option<Endpoint>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TupleKey {
    pid: Option<u32>,
    process_instance: Option<String>,
    local: Endpoint,
    remote: Endpoint,
}

pub struct UdpConversationTracker {
    active: HashMap<TupleKey, UdpConversation>,
    quic_by_destination_id: HashMap<String, TupleKey>,
    idle_timeout_ns: u64,
    next_generation: u64,
}

impl UdpConversationTracker {
    pub fn new(idle_timeout_ns: u64) -> Self {
        Self {
            active: HashMap::new(),
            quic_by_destination_id: HashMap::new(),
            idle_timeout_ns: idle_timeout_ns.max(1),
            next_generation: 1,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn observe_datagram(
        &mut self,
        pid: Option<u32>,
        process_instance: Option<String>,
        local: Endpoint,
        remote: Option<Endpoint>,
        monotonic_ns: u64,
        route_generation: u64,
        payload_prefix: Option<&[u8]>,
    ) -> Option<UdpConversationEvent> {
        let remote =
            remote.filter(|remote| !remote.address.is_unspecified() && remote.port != 0)?;
        let metadata = payload_prefix.and_then(parse_quic_metadata);
        let quic = metadata.clone().map_or_else(
            || {
                if is_quic_hint_port(remote.port) {
                    QuicEvidence::PortHint { port: remote.port }
                } else {
                    QuicEvidence::None
                }
            },
            |metadata| QuicEvidence::Handshake { metadata },
        );
        let key = TupleKey {
            pid,
            process_instance: process_instance.clone(),
            local: local.clone(),
            remote: remote.clone(),
        };
        if let Some(connection_id) = metadata
            .as_ref()
            .map(|metadata| metadata.destination_connection_id.as_str())
            .filter(|connection_id| !connection_id.is_empty())
            && let Some(previous_key) = self.quic_by_destination_id.get(connection_id).cloned()
            && previous_key != key
            && let Some(mut conversation) = self.active.remove(&previous_key)
        {
            let previous_local = conversation.local.clone();
            let previous_remote = conversation.remote.clone();
            conversation.local = local;
            conversation.remote = remote;
            conversation.last_seen_monotonic_ns = monotonic_ns;
            conversation.datagrams = conversation.datagrams.saturating_add(1);
            conversation.route_generation = route_generation;
            conversation.quic = quic;
            self.active.insert(key.clone(), conversation.clone());
            self.quic_by_destination_id
                .insert(connection_id.to_owned(), key);
            return Some(UdpConversationEvent {
                kind: UdpConversationEventKind::Migrated,
                conversation,
                previous_local: Some(previous_local),
                previous_remote: Some(previous_remote),
                reason: "QUIC destination connection ID continued on a new tuple".into(),
            });
        }
        if let Some(conversation) = self.active.get_mut(&key) {
            conversation.last_seen_monotonic_ns = monotonic_ns;
            conversation.datagrams = conversation.datagrams.saturating_add(1);
            conversation.route_generation = route_generation;
            if matches!(&quic, QuicEvidence::Handshake { .. }) {
                conversation.quic = quic;
            }
            return Some(UdpConversationEvent {
                kind: UdpConversationEventKind::Activity,
                conversation: conversation.clone(),
                previous_local: None,
                previous_remote: None,
                reason: "datagram observed for the active UDP tuple".into(),
            });
        }
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        let conversation = UdpConversation {
            id: format!("udp:{generation}"),
            pid,
            process_instance,
            local,
            remote,
            first_seen_monotonic_ns: monotonic_ns,
            last_seen_monotonic_ns: monotonic_ns,
            datagrams: 1,
            route_generation,
            quic,
        };
        if let QuicEvidence::Handshake { metadata } = &conversation.quic
            && !metadata.destination_connection_id.is_empty()
        {
            self.quic_by_destination_id
                .insert(metadata.destination_connection_id.clone(), key.clone());
        }
        self.active.insert(key, conversation.clone());
        Some(UdpConversationEvent {
            kind: UdpConversationEventKind::Opened,
            conversation,
            previous_local: None,
            previous_remote: None,
            reason: "first datagram observed for a tuple with a real peer".into(),
        })
    }

    pub fn expire(&mut self, monotonic_ns: u64) -> Vec<UdpConversationEvent> {
        let expired = self
            .active
            .iter()
            .filter(|(_, conversation)| {
                monotonic_ns.saturating_sub(conversation.last_seen_monotonic_ns)
                    >= self.idle_timeout_ns
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        expired
            .into_iter()
            .filter_map(|key| {
                let conversation = self.active.remove(&key)?;
                if let QuicEvidence::Handshake { metadata } = &conversation.quic {
                    self.quic_by_destination_id
                        .remove(&metadata.destination_connection_id);
                }
                Some(UdpConversationEvent {
                    kind: UdpConversationEventKind::Expired,
                    conversation,
                    previous_local: None,
                    previous_remote: None,
                    reason: "UDP idle timeout elapsed; no TCP-style close was inferred".into(),
                })
            })
            .collect()
    }

    pub fn active(&self) -> impl Iterator<Item = &UdpConversation> {
        self.active.values()
    }
}

impl Default for UdpConversationTracker {
    fn default() -> Self {
        Self::new(DEFAULT_UDP_IDLE_TIMEOUT_NS)
    }
}

pub fn parse_quic_metadata(payload: &[u8]) -> Option<QuicMetadata> {
    if payload.len() < 7 || payload[0] & 0xc0 != 0xc0 {
        return None;
    }
    let version = u32::from_be_bytes(payload.get(1..5)?.try_into().ok()?);
    let destination_length = usize::from(*payload.get(5)?);
    if destination_length > MAX_QUIC_CONNECTION_ID_BYTES {
        return None;
    }
    let destination_start: usize = 6;
    let destination_end = destination_start.checked_add(destination_length)?;
    let destination = payload.get(destination_start..destination_end)?;
    let source_length = usize::from(*payload.get(destination_end)?);
    if source_length > MAX_QUIC_CONNECTION_ID_BYTES {
        return None;
    }
    let source_start = destination_end.checked_add(1)?;
    let source_end = source_start.checked_add(source_length)?;
    let source = payload.get(source_start..source_end)?;
    let packet_kind = if version == 0 {
        QuicPacketKind::VersionNegotiation
    } else {
        match (payload[0] >> 4) & 0x03 {
            0 => QuicPacketKind::Initial,
            1 => QuicPacketKind::ZeroRtt,
            2 => QuicPacketKind::Handshake,
            3 => QuicPacketKind::Retry,
            _ => QuicPacketKind::OtherLongHeader,
        }
    };
    Some(QuicMetadata {
        version,
        packet_kind,
        destination_connection_id: hex(destination),
        source_connection_id: hex(source),
    })
}

fn is_quic_hint_port(port: u16) -> bool {
    matches!(port, 443 | 784 | 853 | 8853)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[usize::from(byte >> 4)] as char);
        result.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use sabicheiro_core::NetworkProtocol;

    fn endpoint(address: &str, port: u16) -> Endpoint {
        Endpoint::new(address.parse().expect("IP"), port)
    }

    fn quic_initial(destination_id: &[u8]) -> Vec<u8> {
        let mut packet = vec![0xc0, 0, 0, 0, 1, destination_id.len() as u8];
        packet.extend_from_slice(destination_id);
        packet.push(2);
        packet.extend_from_slice(&[0xaa, 0xbb]);
        packet
    }

    #[test]
    fn bound_udp_without_a_peer_never_creates_a_destination() {
        let mut tracker = UdpConversationTracker::default();
        assert!(
            tracker
                .observe_datagram(Some(1), None, endpoint("0.0.0.0", 53_000), None, 1, 1, None,)
                .is_none()
        );
        assert_eq!(tracker.active().count(), 0);
    }

    #[test]
    fn udp_expires_without_inventing_a_close_event() {
        let mut tracker = UdpConversationTracker::new(10);
        let opened = tracker
            .observe_datagram(
                Some(1),
                Some("pid:1:start:1".into()),
                endpoint("127.0.0.1", 50_000),
                Some(endpoint("127.0.0.1", 53)),
                1,
                1,
                None,
            )
            .expect("opened");
        assert_eq!(opened.kind, UdpConversationEventKind::Opened);
        assert!(tracker.expire(10).is_empty());
        let expired = tracker.expire(11);
        assert_eq!(expired[0].kind, UdpConversationEventKind::Expired);
        assert!(expired[0].reason.contains("no TCP-style close"));
    }

    #[test]
    fn quic_handshake_is_distinct_from_a_port_hint() {
        let mut tracker = UdpConversationTracker::default();
        let hint = tracker
            .observe_datagram(
                Some(1),
                None,
                endpoint("127.0.0.1", 50_000),
                Some(endpoint("127.0.0.1", 443)),
                1,
                1,
                None,
            )
            .expect("hint");
        assert!(matches!(
            hint.conversation.quic,
            QuicEvidence::PortHint { .. }
        ));
        let packet = quic_initial(&[1, 2, 3, 4]);
        let confirmed = tracker
            .observe_datagram(
                Some(1),
                None,
                endpoint("127.0.0.1", 50_001),
                Some(endpoint("127.0.0.1", 443)),
                2,
                1,
                Some(&packet),
            )
            .expect("handshake");
        assert!(matches!(
            confirmed.conversation.quic,
            QuicEvidence::Handshake { .. }
        ));
    }

    #[test]
    fn quic_connection_id_survives_ipv6_route_migration() {
        let mut tracker = UdpConversationTracker::default();
        let packet = quic_initial(&[9, 8, 7, 6]);
        let opened = tracker
            .observe_datagram(
                Some(1),
                Some("pid:1:start:1".into()),
                endpoint("::1", 50_000),
                Some(endpoint("::1", 443)),
                1,
                1,
                Some(&packet),
            )
            .expect("opened");
        let migrated = tracker
            .observe_datagram(
                Some(1),
                Some("pid:1:start:1".into()),
                endpoint("::1", 50_001),
                Some(endpoint("::1", 443)),
                2,
                2,
                Some(&packet),
            )
            .expect("migrated");
        assert_eq!(migrated.kind, UdpConversationEventKind::Migrated);
        assert_eq!(migrated.conversation.id, opened.conversation.id);
        assert_eq!(migrated.conversation.route_generation, 2);
        assert_eq!(tracker.active().count(), 1);
    }

    #[test]
    fn parser_rejects_truncation_and_oversized_connection_ids() {
        assert!(parse_quic_metadata(&[]).is_none());
        assert!(parse_quic_metadata(&[0xc0, 0, 0, 0, 1, 21, 0]).is_none());
        let metadata = parse_quic_metadata(&quic_initial(&[1, 2, 3])).expect("metadata");
        assert_eq!(metadata.version, 1);
        assert_eq!(metadata.packet_kind, QuicPacketKind::Initial);
        assert_eq!(metadata.destination_connection_id, "010203");
    }

    #[test]
    fn protocol_model_remains_udp() {
        assert_eq!(NetworkProtocol::Udp.as_str(), "udp");
    }
}
