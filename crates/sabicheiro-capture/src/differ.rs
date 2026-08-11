use crate::{ObservedSocket, SocketIdentity};
use sabicheiro_core::{ConnectionKey, NetworkEventKind, NormalizedNetworkEvent, TrafficCounters};
use std::collections::HashMap;

#[derive(Default)]
pub struct SnapshotDiffer {
    previous: HashMap<SocketIdentity, ObservedSocket>,
}

impl SnapshotDiffer {
    pub fn diff(
        &mut self,
        current: Vec<ObservedSocket>,
        timestamp_ms: i64,
    ) -> Vec<NormalizedNetworkEvent> {
        let current = current
            .into_iter()
            .map(|socket| (socket.identity.clone(), socket))
            .collect::<HashMap<_, _>>();
        let mut events = Vec::with_capacity(current.len() + self.previous.len());
        for (identity, socket) in &current {
            let kind = if self.previous.contains_key(identity) {
                NetworkEventKind::Updated
            } else {
                NetworkEventKind::Opened
            };
            events.push(to_event(socket, kind, timestamp_ms));
        }
        for (identity, socket) in &self.previous {
            if !current.contains_key(identity) {
                events.push(to_event(socket, NetworkEventKind::Closed, timestamp_ms));
            }
        }
        self.previous = current;
        events
    }

    pub fn clear(&mut self, timestamp_ms: i64) -> Vec<NormalizedNetworkEvent> {
        let events = self
            .previous
            .values()
            .map(|socket| to_event(socket, NetworkEventKind::Closed, timestamp_ms))
            .collect();
        self.previous.clear();
        events
    }
}

fn to_event(
    socket: &ObservedSocket,
    kind: NetworkEventKind,
    timestamp_ms: i64,
) -> NormalizedNetworkEvent {
    let remote = socket.normalized_remote();
    NormalizedNetworkEvent {
        kind,
        key: ConnectionKey {
            pid: socket.identity.pid,
            local: socket.identity.local.clone(),
            remote,
            protocol: socket.identity.protocol,
        },
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObservedSocket;
    use sabicheiro_core::{ConnectionState, Direction, Endpoint, NetworkProtocol, ProcessInfo};

    fn socket() -> ObservedSocket {
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
                ..ProcessInfo::default()
            },
            direction: Direction::Outbound,
            state: ConnectionState::Established,
            interface: None,
        }
    }

    #[test]
    fn emits_open_update_and_close() {
        let mut differ = SnapshotDiffer::default();
        assert_eq!(
            differ.diff(vec![socket()], 1)[0].kind,
            NetworkEventKind::Opened
        );
        assert_eq!(
            differ.diff(vec![socket()], 2)[0].kind,
            NetworkEventKind::Updated
        );
        assert_eq!(differ.diff(Vec::new(), 3)[0].kind, NetworkEventKind::Closed);
    }
}
