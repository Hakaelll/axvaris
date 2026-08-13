use crate::{ObservedSocket, SocketIdentity};
use sabicheiro_core::{
    ConnectionKey, CounterSource, NetworkEventKind, NormalizedNetworkEvent, TrafficCounters,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SocketFingerprint {
    identity: SocketIdentity,
    process_instance: Option<String>,
    interface_id: Option<String>,
}

impl SocketFingerprint {
    fn from_socket(socket: &ObservedSocket) -> Self {
        Self {
            identity: socket.identity.clone(),
            process_instance: socket.process.instance_id(),
            interface_id: socket.interface.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct TrackedSocket {
    socket: ObservedSocket,
    flow_generation: u64,
}

pub struct SnapshotDiffer {
    previous: HashMap<SocketFingerprint, TrackedSocket>,
    capture_generation: u64,
    next_flow_generation: u64,
    sequence: u64,
}

impl Default for SnapshotDiffer {
    fn default() -> Self {
        Self::new(1)
    }
}

impl SnapshotDiffer {
    pub fn new(capture_generation: u64) -> Self {
        Self {
            previous: HashMap::new(),
            capture_generation: capture_generation.max(1),
            next_flow_generation: 1,
            sequence: 0,
        }
    }

    pub fn diff(
        &mut self,
        current: Vec<ObservedSocket>,
        timestamp_ms: i64,
    ) -> Vec<NormalizedNetworkEvent> {
        let mut current_tracked = HashMap::with_capacity(current.len());
        let mut events = Vec::with_capacity(current.len() + self.previous.len());
        for socket in current {
            let fingerprint = SocketFingerprint::from_socket(&socket);
            let (kind, flow_generation) = self.previous.get(&fingerprint).map_or_else(
                || {
                    let flow_generation = self.next_flow_generation;
                    self.next_flow_generation = self.next_flow_generation.saturating_add(1);
                    (NetworkEventKind::Opened, flow_generation)
                },
                |tracked| (NetworkEventKind::Updated, tracked.flow_generation),
            );
            self.sequence = self.sequence.saturating_add(1);
            events.push(to_event(
                &socket,
                kind,
                timestamp_ms,
                self.capture_generation,
                flow_generation,
                self.sequence,
            ));
            current_tracked.insert(
                fingerprint,
                TrackedSocket {
                    socket,
                    flow_generation,
                },
            );
        }
        for (fingerprint, tracked) in &self.previous {
            if !current_tracked.contains_key(fingerprint) {
                self.sequence = self.sequence.saturating_add(1);
                events.push(to_event(
                    &tracked.socket,
                    NetworkEventKind::Closed,
                    timestamp_ms,
                    self.capture_generation,
                    tracked.flow_generation,
                    self.sequence,
                ));
            }
        }
        self.previous = current_tracked;
        events
    }

    pub fn clear(&mut self, timestamp_ms: i64) -> Vec<NormalizedNetworkEvent> {
        let mut events = Vec::with_capacity(self.previous.len());
        for tracked in self.previous.values() {
            self.sequence = self.sequence.saturating_add(1);
            events.push(to_event(
                &tracked.socket,
                NetworkEventKind::Closed,
                timestamp_ms,
                self.capture_generation,
                tracked.flow_generation,
                self.sequence,
            ));
        }
        self.previous.clear();
        events
    }

    pub const fn capture_generation(&self) -> u64 {
        self.capture_generation
    }
}

fn to_event(
    socket: &ObservedSocket,
    kind: NetworkEventKind,
    timestamp_ms: i64,
    capture_generation: u64,
    flow_generation: u64,
    sequence: u64,
) -> NormalizedNetworkEvent {
    let remote = socket.normalized_remote();
    let mut evidence = socket.evidence(timestamp_ms);
    evidence.sequence = sequence;
    NormalizedNetworkEvent {
        kind,
        key: ConnectionKey {
            pid: socket.identity.pid,
            process_instance: socket.process.instance_id(),
            local: socket.identity.local.clone(),
            remote,
            protocol: socket.identity.protocol,
            interface_id: socket.interface.clone(),
            capture_generation,
            flow_generation,
        },
        evidence,
        timestamp_ms,
        process: socket.process.clone(),
        direction: socket.direction,
        state: if kind == NetworkEventKind::Closed {
            sabicheiro_core::ConnectionState::Closed
        } else {
            socket.state
        },
        interface: socket.interface.clone(),
        hostname: None,
        domain: None,
        geo: None,
        asn: None,
        traffic_delta: TrafficCounters::default(),
        counter_source: CounterSource::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObservedSocket;
    use sabicheiro_core::{
        ConnectionState, Direction, Endpoint, NetworkProtocol, ObservationSource, ProcessInfo,
    };

    fn socket(started_at_ms: i64) -> ObservedSocket {
        ObservedSocket {
            identity: SocketIdentity {
                pid: Some(7),
                local: Endpoint::new("127.0.0.1".parse().expect("IP"), 40_000),
                remote: Some(Endpoint::new("1.1.1.1".parse().expect("IP"), 443)),
                protocol: NetworkProtocol::Tcp,
            },
            process: ProcessInfo {
                pid: Some(7),
                name: "test".into(),
                started_at_ms: Some(started_at_ms),
                ..ProcessInfo::default()
            },
            direction: Direction::Outbound,
            state: ConnectionState::Established,
            interface: Some("loopback".into()),
            observation_source: ObservationSource::LinuxInetDiag,
        }
    }

    #[test]
    fn emits_open_update_and_close_with_stable_identity() {
        let mut differ = SnapshotDiffer::new(9);
        let opened = differ.diff(vec![socket(100)], 1).remove(0);
        let updated = differ.diff(vec![socket(100)], 2).remove(0);
        let closed = differ.diff(Vec::new(), 3).remove(0);
        assert_eq!(opened.kind, NetworkEventKind::Opened);
        assert_eq!(updated.kind, NetworkEventKind::Updated);
        assert_eq!(closed.kind, NetworkEventKind::Closed);
        assert_eq!(opened.key, updated.key);
        assert_eq!(updated.key, closed.key);
        assert_eq!(opened.key.capture_generation, 9);
    }

    #[test]
    fn process_reuse_and_reopen_receive_new_flow_generations() {
        let mut differ = SnapshotDiffer::default();
        let first = differ.diff(vec![socket(100)], 1).remove(0);
        let _closed = differ.diff(Vec::new(), 2);
        let reopened = differ.diff(vec![socket(100)], 3).remove(0);
        let reused_pid = differ.diff(vec![socket(200)], 4);
        assert_ne!(first.key.flow_generation, reopened.key.flow_generation);
        assert!(
            reused_pid
                .iter()
                .any(|event| event.kind == NetworkEventKind::Opened)
        );
        assert!(
            reused_pid
                .iter()
                .any(|event| event.kind == NetworkEventKind::Closed)
        );
    }
}
