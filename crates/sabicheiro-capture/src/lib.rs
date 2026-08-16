mod correlator;
mod differ;
mod executable;
mod fusion;
mod harness;
mod helper_protocol;
#[cfg(any(target_os = "linux", test))]
mod inet_diag_wire;
mod pipeline;
mod process;
mod recording;
mod route;
mod sequencer;
mod supervisor;
mod udp;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod windows_events;
#[cfg(target_os = "windows")]
mod windows_helper;
#[cfg(target_os = "windows")]
mod windows_helper_client;

pub use correlator::{CorrelationStats, FlowCorrelator};
pub use differ::SnapshotDiffer;
pub use executable::inspect_executable;
pub use fusion::{CorrelationCandidate, EvidenceFusion, FusionBatch, FusionStats, replay_evidence};
pub use harness::{
    CaptureHarnessReport, FixtureFlow, FixtureFlowKind, FixtureManifest, FixturePerturbation,
    ProviderHarnessRun, compare_observations, reference_manifest, run_loopback_truth_suite,
    run_system_provider_fixture, run_windows_event_fixture,
};
pub use helper_protocol::{
    HELPER_MAX_FRAME_BYTES, HELPER_PROTOCOL_VERSION, HelperDnsEvent, HelperEnvelope, HelperError,
    HelperNonce, HelperOperation, HelperResponse, HelperSession, decode_helper_frame,
    decode_helper_response_frame, encode_helper_frame,
};
pub use pipeline::{BoundedEvidenceQueue, PipelineQueueClass, PipelineQueueStats, PipelineStage};
pub use recording::RecordingController;
pub use route::lookup_network_path;
pub use sequencer::{EvidenceSequencer, ProviderSequenceTracker, network_provenance};
pub use supervisor::{ProviderAction, ProviderSupervisor, SupervisorPolicy};
pub use udp::{
    QuicEvidence, QuicMetadata, QuicPacketKind, UdpConversation, UdpConversationEvent,
    UdpConversationEventKind, UdpConversationTracker, parse_quic_metadata,
};
#[cfg(target_os = "windows")]
pub use windows_events::{DnsObservation, WindowsEventCapture};
#[cfg(target_os = "windows")]
pub use windows_helper::{HELPER_PIPE_NAME, run_windows_helper};
#[cfg(target_os = "windows")]
pub use windows_helper_client::{WindowsHelperClient, secure_helper_nonce};

use sabicheiro_core::{
    CapabilityAvailability, CaptureCapabilityKind, ConnectionEvidence, ConnectionState, Direction,
    Endpoint, NetworkInterface, NetworkProtocol, ObservationSource, ProcessInfo,
    ProviderCapability, ProviderDescriptor, ProviderMode,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture is not supported on this operating system")]
    Unsupported,
    #[error("capture permission denied: {0}")]
    PermissionDenied(String),
    #[error("operating-system capture error: {0}")]
    OperatingSystem(String),
    #[error("invalid capture data: {0}")]
    InvalidData(String),
    #[error("recording error: {0}")]
    Recording(String),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SocketIdentity {
    pub pid: Option<u32>,
    pub local: Endpoint,
    pub remote: Option<Endpoint>,
    pub protocol: NetworkProtocol,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedSocket {
    pub identity: SocketIdentity,
    pub process: ProcessInfo,
    pub direction: Direction,
    pub state: ConnectionState,
    pub interface: Option<String>,
    pub observation_source: ObservationSource,
}

impl ObservedSocket {
    /// Returns true only when the operating system exposed an actual remote peer.
    /// Bound UDP sockets and TCP listeners are useful diagnostics, but they are not
    /// remote connections and must not dominate the normal observability feed.
    pub fn has_remote_peer(&self) -> bool {
        self.state != ConnectionState::Listening
            && self
                .identity
                .remote
                .as_ref()
                .is_some_and(|remote| !remote.address.is_unspecified() && remote.port != 0)
    }

    pub fn normalized_remote(&self) -> Endpoint {
        self.identity.remote.clone().unwrap_or_else(|| {
            let address = match self.identity.local.address {
                IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
            };
            Endpoint::new(address, 0)
        })
    }

    pub fn evidence(&self, timestamp_ms: i64) -> ConnectionEvidence {
        ConnectionEvidence::for_peer(
            &self.normalized_remote(),
            self.state,
            self.observation_source,
            timestamp_ms,
        )
    }
}

pub trait NetworkCaptureProvider: Send {
    fn name(&self) -> &'static str;
    fn snapshot(&mut self) -> Result<Vec<ObservedSocket>, CaptureError>;
    fn interfaces(&mut self) -> Result<Vec<NetworkInterface>, CaptureError>;

    fn descriptor(&self, generation: u64) -> ProviderDescriptor {
        ProviderDescriptor {
            protocol_version: 1,
            provider_id: self.name().into(),
            display_name: self.name().into(),
            mode: ProviderMode::Snapshot,
            generation: generation.max(1),
            capabilities: vec![ProviderCapability {
                kind: CaptureCapabilityKind::SocketSnapshot,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::Unknown,
                limitation: Some("provider did not publish field-level capabilities".into()),
            }],
        }
    }
}

pub fn system_provider() -> Result<Box<dyn NetworkCaptureProvider>, CaptureError> {
    #[cfg(target_os = "windows")]
    {
        return Ok(Box::new(windows::WindowsCaptureProvider::new()));
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(Box::new(linux::LinuxCaptureProvider::new()));
    }
    #[allow(unreachable_code)]
    Err(CaptureError::Unsupported)
}

pub struct MockCaptureProvider {
    snapshots: std::collections::VecDeque<Vec<ObservedSocket>>,
}

impl MockCaptureProvider {
    pub fn new(snapshots: impl IntoIterator<Item = Vec<ObservedSocket>>) -> Self {
        Self {
            snapshots: snapshots.into_iter().collect(),
        }
    }
}

impl NetworkCaptureProvider for MockCaptureProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn snapshot(&mut self) -> Result<Vec<ObservedSocket>, CaptureError> {
        Ok(self.snapshots.pop_front().unwrap_or_default())
    }

    fn interfaces(&mut self) -> Result<Vec<NetworkInterface>, CaptureError> {
        Ok(Vec::new())
    }

    fn descriptor(&self, generation: u64) -> ProviderDescriptor {
        ProviderDescriptor {
            protocol_version: 1,
            provider_id: "fixture.mock".into(),
            display_name: "Deterministic fixture provider".into(),
            mode: ProviderMode::Fixture,
            generation: generation.max(1),
            capabilities: [
                CaptureCapabilityKind::LifecycleEvents,
                CaptureCapabilityKind::SocketSnapshot,
                CaptureCapabilityKind::ProcessIdentity,
                CaptureCapabilityKind::Interface,
            ]
            .into_iter()
            .map(|kind| ProviderCapability {
                kind,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::Demo,
                limitation: None,
            })
            .collect(),
        }
    }
}

pub(crate) fn infer_direction(
    local: &Endpoint,
    remote: Option<&Endpoint>,
    state: ConnectionState,
) -> Direction {
    let Some(remote) = remote else {
        return Direction::Unknown;
    };
    if state == ConnectionState::Listening || remote.address.is_unspecified() {
        return Direction::Unknown;
    }
    const EPHEMERAL_START: u16 = 49_152;
    match (
        local.port >= EPHEMERAL_START,
        remote.port >= EPHEMERAL_START,
    ) {
        (true, false) => Direction::Outbound,
        (false, true) => Direction::Inbound,
        _ => Direction::Outbound,
    }
}

pub(crate) fn interfaces_from_sysinfo() -> Vec<NetworkInterface> {
    let networks = sysinfo::Networks::new_with_refreshed_list();
    let mut result = networks
        .iter()
        .map(|(name, data)| {
            let addresses = data
                .ip_networks()
                .iter()
                .map(|network| network.addr)
                .collect();
            let is_up = matches!(
                data.operational_state(),
                sysinfo::InterfaceOperationalState::Up
                    | sysinfo::InterfaceOperationalState::Dormant
                    | sysinfo::InterfaceOperationalState::Unknown
            );
            interface_model(name.clone(), addresses, is_up)
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| left.name.cmp(&right.name));
    result
}

pub(crate) fn interface_model(
    name: String,
    mut addresses: Vec<IpAddr>,
    is_up: bool,
) -> NetworkInterface {
    addresses.sort();
    addresses.dedup();
    let lower = name.to_lowercase();
    let is_loopback = lower.contains("loopback")
        || lower == "lo"
        || (!addresses.is_empty() && addresses.iter().all(IpAddr::is_loopback));
    let is_vpn = ["vpn", "tun", "tap", "wireguard", "wg", "tailscale"]
        .iter()
        .any(|marker| lower.contains(marker));
    let is_virtual = is_vpn
        || ["virtual", "vmware", "hyper-v", "vbox", "docker", "bridge"]
            .iter()
            .any(|marker| lower.contains(marker));
    NetworkInterface {
        id: name.clone(),
        stable_id: name.clone(),
        name: name.clone(),
        system_name: name,
        description: if is_vpn {
            "VPN or tunnel interface".into()
        } else if is_virtual {
            "Virtual network interface".into()
        } else if is_loopback {
            "Loopback interface".into()
        } else {
            "Network interface".into()
        },
        luid: None,
        network_guid: None,
        addresses,
        is_up,
        is_loopback,
        is_virtual,
        is_vpn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket(remote: Option<Endpoint>, state: ConnectionState) -> ObservedSocket {
        let local = Endpoint::new("192.168.1.10".parse().expect("local IP"), 50_000);
        ObservedSocket {
            identity: SocketIdentity {
                pid: Some(7),
                local: local.clone(),
                remote: remote.clone(),
                protocol: NetworkProtocol::Tcp,
            },
            process: ProcessInfo::default(),
            direction: infer_direction(&local, remote.as_ref(), state),
            state,
            interface: None,
            observation_source: ObservationSource::Unknown,
        }
    }

    #[test]
    fn only_sockets_with_real_remote_peers_are_connections() {
        let remote = Endpoint::new("1.1.1.1".parse().expect("remote IP"), 443);
        assert!(socket(Some(remote), ConnectionState::Established).has_remote_peer());

        let wildcard = Endpoint::new("0.0.0.0".parse().expect("wildcard IP"), 0);
        assert!(!socket(Some(wildcard), ConnectionState::Listening).has_remote_peer());
        assert!(!socket(None, ConnectionState::Listening).has_remote_peer());
    }

    #[test]
    fn interface_model_preserves_unicode_names() {
        let name = "Conexión de área local* 1";
        let interface = interface_model(name.into(), Vec::new(), true);
        assert_eq!(interface.name, name);
        assert!(!interface.name.contains('\u{fffd}'));
    }
}
