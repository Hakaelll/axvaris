use crate::{
    CaptureError, NetworkCaptureProvider, ObservedSocket, SocketIdentity, infer_direction,
    interfaces_from_sysinfo, process::ProcessResolver,
};
use sabicheiro_core::{
    CapabilityAvailability, CaptureCapabilityKind, ConnectionState, Endpoint, NetworkInterface,
    NetworkProtocol, ObservationSource, ProviderCapability, ProviderDescriptor, ProviderMode,
};
use std::{
    ffi::{CStr, c_void},
    mem::size_of,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};
use windows_sys::{
    Win32::{
        Foundation::ERROR_BUFFER_OVERFLOW,
        NetworkManagement::{
            IpHelper::{
                GAA_FLAG_INCLUDE_PREFIX, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
                GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses, IF_TYPE_SOFTWARE_LOOPBACK,
                IP_ADAPTER_ADDRESSES_LH,
            },
            Ndis::IfOperStatusUp,
        },
        Networking::WinSock::{SOCKADDR_IN, SOCKADDR_IN6},
    },
    core::GUID,
};

const AF_INET: u32 = 2;
const AF_INET6: u32 = 23;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
const TCP_TABLE_OWNER_PID_ALL: i32 = 5;
const UDP_TABLE_OWNER_PID: i32 = 1;

#[link(name = "iphlpapi")]
unsafe extern "system" {
    fn GetExtendedTcpTable(
        table: *mut c_void,
        size: *mut u32,
        order: i32,
        family: u32,
        class: i32,
        reserved: u32,
    ) -> u32;
    fn GetExtendedUdpTable(
        table: *mut c_void,
        size: *mut u32,
        order: i32,
        family: u32,
        class: i32,
        reserved: u32,
    ) -> u32;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Tcp4Row {
    state: u32,
    local_addr: u32,
    local_port: u32,
    remote_addr: u32,
    remote_port: u32,
    pid: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Tcp6Row {
    local_addr: [u8; 16],
    local_scope_id: u32,
    local_port: u32,
    remote_addr: [u8; 16],
    remote_scope_id: u32,
    remote_port: u32,
    state: u32,
    pid: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Udp4Row {
    local_addr: u32,
    local_port: u32,
    pid: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Udp6Row {
    local_addr: [u8; 16],
    local_scope_id: u32,
    local_port: u32,
    pid: u32,
}

pub(crate) struct WindowsCaptureProvider {
    processes: ProcessResolver,
}

impl WindowsCaptureProvider {
    pub(crate) fn new() -> Self {
        Self {
            processes: ProcessResolver::new(),
        }
    }
}

impl NetworkCaptureProvider for WindowsCaptureProvider {
    fn name(&self) -> &'static str {
        "windows-ip-helper"
    }

    fn snapshot(&mut self) -> Result<Vec<ObservedSocket>, CaptureError> {
        let mut raw = Vec::new();
        raw.extend(read_tcp4()?);
        raw.extend(read_tcp6()?);
        raw.extend(read_udp4()?);
        raw.extend(read_udp6()?);
        self.processes
            .refresh(raw.iter().filter_map(|socket| socket.identity.pid));
        for socket in &mut raw {
            socket.process = self.processes.resolve(socket.identity.pid);
        }
        Ok(raw)
    }

    fn interfaces(&mut self) -> Result<Vec<NetworkInterface>, CaptureError> {
        windows_interfaces().or_else(|error| {
            tracing::warn!(%error, "stable adapter discovery failed; using the compatibility list");
            let fallback = interfaces_from_sysinfo();
            (!fallback.is_empty()).then_some(fallback).ok_or(error)
        })
    }

    fn descriptor(&self, generation: u64) -> ProviderDescriptor {
        ProviderDescriptor {
            protocol_version: 1,
            provider_id: "windows.ip_helper".into(),
            display_name: "Windows IP Helper".into(),
            mode: ProviderMode::Snapshot,
            generation: generation.max(1),
            capabilities: [
                CaptureCapabilityKind::SocketSnapshot,
                CaptureCapabilityKind::ProcessIdentity,
                CaptureCapabilityKind::Interface,
            ]
            .into_iter()
            .map(|kind| ProviderCapability {
                kind,
                availability: CapabilityAvailability::Available,
                source: ObservationSource::WindowsIpHelper,
                limitation: (kind == CaptureCapabilityKind::SocketSnapshot)
                    .then(|| "open/close timestamps are bounded by the polling interval".into()),
            })
            .collect(),
        }
    }
}

fn windows_interfaces() -> Result<Vec<NetworkInterface>, CaptureError> {
    const FAMILY_UNSPECIFIED: u32 = 0;
    const MAX_ADAPTER_BYTES: u32 = 16 * 1024 * 1024;
    let flags = GAA_FLAG_INCLUDE_PREFIX
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size = 15_000_u32;
    for _ in 0..3 {
        if size == 0 || size > MAX_ADAPTER_BYTES {
            break;
        }
        // usize alignment is sufficient for IP_ADAPTER_ADDRESSES and its linked nodes.
        let mut storage = vec![0_usize; (size as usize).div_ceil(size_of::<usize>())];
        let result = unsafe {
            GetAdaptersAddresses(
                FAMILY_UNSPECIFIED,
                flags,
                ptr::null(),
                storage.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>(),
                &mut size,
            )
        };
        if result == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if result != 0 {
            return Err(CaptureError::OperatingSystem(format!(
                "GetAdaptersAddresses failed with Windows error {result}"
            )));
        }
        let mut interfaces = Vec::new();
        let mut current = storage.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        for _ in 0..4_096 {
            let Some(adapter) = (unsafe { current.as_ref() }) else {
                break;
            };
            let permanent_name = unsafe { c_string(adapter.AdapterName) };
            let friendly_name = unsafe { wide_string(adapter.FriendlyName) }
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| permanent_name.clone());
            let native_description =
                unsafe { wide_string(adapter.Description) }.filter(|value| !value.is_empty());
            let mut addresses = Vec::new();
            let mut unicast = adapter.FirstUnicastAddress;
            for _ in 0..512 {
                let Some(address) = (unsafe { unicast.as_ref() }) else {
                    break;
                };
                if let Some(ip) = unsafe { socket_address(address.Address.lpSockaddr) } {
                    addresses.push(ip);
                }
                unicast = address.Next;
            }
            let mut interface = crate::interface_model(
                friendly_name.clone(),
                addresses,
                adapter.OperStatus == IfOperStatusUp,
            );
            let luid = unsafe { adapter.Luid.Value };
            let network_guid = format_guid(adapter.NetworkGuid);
            let stable_id = if !permanent_name.is_empty() {
                format!("windows-adapter:{}", permanent_name.to_ascii_lowercase())
            } else if luid != 0 {
                format!("windows-luid:{luid:016x}")
            } else {
                format!("windows-name:{}", friendly_name.to_ascii_lowercase())
            };
            interface.id = stable_id.clone();
            interface.stable_id = stable_id;
            interface.system_name = permanent_name;
            interface.luid = (luid != 0).then_some(luid);
            interface.network_guid = network_guid;
            interface.is_loopback =
                interface.is_loopback || adapter.IfType == IF_TYPE_SOFTWARE_LOOPBACK;
            if let Some(description) = native_description {
                interface.description = description;
            }
            interfaces.push(interface);
            current = adapter.Next;
        }
        interfaces.sort_by(|left, right| left.name.cmp(&right.name));
        return Ok(interfaces);
    }
    Err(CaptureError::InvalidData(
        "GetAdaptersAddresses returned an unstable buffer size".into(),
    ))
}

unsafe fn c_string(value: *mut u8) -> String {
    if value.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(value.cast()) }
        .to_string_lossy()
        .into_owned()
}

unsafe fn wide_string(value: *mut u16) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let mut length = 0_usize;
    while length < 32_768 && unsafe { *value.add(length) } != 0 {
        length += 1;
    }
    (length < 32_768)
        .then(|| String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(value, length) }))
}

unsafe fn socket_address(
    value: *mut windows_sys::Win32::Networking::WinSock::SOCKADDR,
) -> Option<IpAddr> {
    let family = unsafe { value.as_ref() }?.sa_family as u32;
    match family {
        AF_INET => {
            let address = unsafe { &*value.cast::<SOCKADDR_IN>() };
            let bytes = unsafe { address.sin_addr.S_un.S_un_b };
            Some(IpAddr::V4(Ipv4Addr::new(
                bytes.s_b1, bytes.s_b2, bytes.s_b3, bytes.s_b4,
            )))
        }
        AF_INET6 => {
            let address = unsafe { &*value.cast::<SOCKADDR_IN6>() };
            Some(IpAddr::V6(Ipv6Addr::from(unsafe {
                address.sin6_addr.u.Byte
            })))
        }
        _ => None,
    }
}

fn format_guid(guid: GUID) -> Option<String> {
    if guid.data1 == 0 && guid.data2 == 0 && guid.data3 == 0 && guid.data4 == [0; 8] {
        return None;
    }
    Some(format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    ))
}

fn read_tcp4() -> Result<Vec<ObservedSocket>, CaptureError> {
    let buffer = query_table(|table, size| unsafe {
        GetExtendedTcpTable(table, size, 0, AF_INET, TCP_TABLE_OWNER_PID_ALL, 0)
    })?;
    parse_rows::<Tcp4Row>(&buffer, |row| {
        let local = Endpoint::new(
            IpAddr::V4(Ipv4Addr::from(row.local_addr.to_ne_bytes())),
            network_port(row.local_port),
        );
        let remote = Endpoint::new(
            IpAddr::V4(Ipv4Addr::from(row.remote_addr.to_ne_bytes())),
            network_port(row.remote_port),
        );
        observed(
            row.pid,
            local,
            Some(remote),
            NetworkProtocol::Tcp,
            tcp_state(row.state),
        )
    })
}

fn read_tcp6() -> Result<Vec<ObservedSocket>, CaptureError> {
    let buffer = query_table(|table, size| unsafe {
        GetExtendedTcpTable(table, size, 0, AF_INET6, TCP_TABLE_OWNER_PID_ALL, 0)
    })?;
    parse_rows::<Tcp6Row>(&buffer, |row| {
        let local = Endpoint::new(
            IpAddr::V6(Ipv6Addr::from(row.local_addr)),
            network_port(row.local_port),
        );
        let remote = Endpoint::new(
            IpAddr::V6(Ipv6Addr::from(row.remote_addr)),
            network_port(row.remote_port),
        );
        observed(
            row.pid,
            local,
            Some(remote),
            NetworkProtocol::Tcp,
            tcp_state(row.state),
        )
    })
}

fn read_udp4() -> Result<Vec<ObservedSocket>, CaptureError> {
    let buffer = query_table(|table, size| unsafe {
        GetExtendedUdpTable(table, size, 0, AF_INET, UDP_TABLE_OWNER_PID, 0)
    })?;
    parse_rows::<Udp4Row>(&buffer, |row| {
        let local = Endpoint::new(
            IpAddr::V4(Ipv4Addr::from(row.local_addr.to_ne_bytes())),
            network_port(row.local_port),
        );
        observed(
            row.pid,
            local,
            None,
            NetworkProtocol::Udp,
            ConnectionState::Listening,
        )
    })
}

fn read_udp6() -> Result<Vec<ObservedSocket>, CaptureError> {
    let buffer = query_table(|table, size| unsafe {
        GetExtendedUdpTable(table, size, 0, AF_INET6, UDP_TABLE_OWNER_PID, 0)
    })?;
    parse_rows::<Udp6Row>(&buffer, |row| {
        let local = Endpoint::new(
            IpAddr::V6(Ipv6Addr::from(row.local_addr)),
            network_port(row.local_port),
        );
        observed(
            row.pid,
            local,
            None,
            NetworkProtocol::Udp,
            ConnectionState::Listening,
        )
    })
}

fn observed(
    pid: u32,
    local: Endpoint,
    remote: Option<Endpoint>,
    protocol: NetworkProtocol,
    state: ConnectionState,
) -> ObservedSocket {
    let direction = infer_direction(&local, remote.as_ref(), state);
    ObservedSocket {
        identity: SocketIdentity {
            pid: (pid != 0).then_some(pid),
            local,
            remote,
            protocol,
        },
        process: Default::default(),
        direction,
        state,
        interface: None,
        observation_source: ObservationSource::WindowsIpHelper,
    }
}

fn query_table(query: impl Fn(*mut c_void, *mut u32) -> u32) -> Result<Vec<u8>, CaptureError> {
    let mut size = 0_u32;
    let first = query(ptr::null_mut(), &mut size);
    if first != ERROR_INSUFFICIENT_BUFFER && first != 0 {
        return Err(CaptureError::OperatingSystem(format!(
            "IP Helper sizing call failed with Windows error {first}"
        )));
    }
    if size < size_of::<u32>() as u32 || size > 128 * 1024 * 1024 {
        return Err(CaptureError::InvalidData(format!(
            "IP Helper returned invalid table size {size}"
        )));
    }
    let mut buffer = vec![0_u8; size as usize];
    let result = query(buffer.as_mut_ptr().cast(), &mut size);
    if result != 0 {
        return Err(CaptureError::OperatingSystem(format!(
            "IP Helper table query failed with Windows error {result}"
        )));
    }
    buffer.truncate(size as usize);
    Ok(buffer)
}

fn parse_rows<T: Copy>(
    buffer: &[u8],
    mut map: impl FnMut(T) -> ObservedSocket,
) -> Result<Vec<ObservedSocket>, CaptureError> {
    if buffer.len() < size_of::<u32>() {
        return Err(CaptureError::InvalidData(
            "table header is truncated".into(),
        ));
    }
    let count = unsafe { ptr::read_unaligned(buffer.as_ptr().cast::<u32>()) } as usize;
    let required =
        size_of::<u32>()
            .checked_add(count.checked_mul(size_of::<T>()).ok_or_else(|| {
                CaptureError::InvalidData("row count overflows address space".into())
            })?)
            .ok_or_else(|| CaptureError::InvalidData("table size overflows".into()))?;
    if required > buffer.len() {
        return Err(CaptureError::InvalidData(format!(
            "table declares {count} rows but buffer is truncated"
        )));
    }
    let mut rows = Vec::with_capacity(count);
    let start = unsafe { buffer.as_ptr().add(size_of::<u32>()) };
    for index in 0..count {
        let row = unsafe { ptr::read_unaligned(start.add(index * size_of::<T>()).cast::<T>()) };
        rows.push(map(row));
    }
    Ok(rows)
}

fn network_port(value: u32) -> u16 {
    u16::from_be(value as u16)
}

fn tcp_state(value: u32) -> ConnectionState {
    match value {
        1 | 12 => ConnectionState::Closed,
        2 => ConnectionState::Listening,
        3 | 4 => ConnectionState::Connecting,
        5 => ConnectionState::Established,
        6..=11 => ConnectionState::Closing,
        _ => ConnectionState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn port_network_order_is_decoded() {
        assert_eq!(network_port(0x0000_BB01), 443);
    }

    #[test]
    fn provider_can_read_native_tables() {
        let mut provider = WindowsCaptureProvider::new();
        let result = provider.snapshot();
        assert!(result.is_ok(), "native table read failed: {result:?}");
    }

    #[test]
    fn provider_observes_an_established_tcp_peer() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind loopback listener");
        let address = listener.local_addr().expect("listener address");
        let client = TcpStream::connect(address).expect("connect loopback client");
        let (server, _) = listener.accept().expect("accept loopback client");

        let mut provider = WindowsCaptureProvider::new();
        let sockets = provider.snapshot().expect("native TCP snapshot");
        let current_pid = std::process::id();
        assert!(
            sockets.iter().any(|socket| {
                socket.identity.pid == Some(current_pid)
                    && socket.state == ConnectionState::Established
                    && socket.has_remote_peer()
                    && socket.identity.local.address.is_loopback()
            }),
            "the native provider did not expose the established loopback peer"
        );

        drop(server);
        drop(client);
    }

    #[test]
    fn provider_lists_unicode_interfaces_with_addresses() {
        let mut provider = WindowsCaptureProvider::new();
        let interfaces = provider.interfaces().expect("native interfaces");
        assert!(!interfaces.is_empty());
        assert!(
            interfaces
                .iter()
                .all(|interface| !interface.name.contains('\u{fffd}')),
            "native interface names must not contain replacement characters: {interfaces:?}"
        );
        assert!(
            interfaces
                .iter()
                .any(|interface| !interface.addresses.is_empty()),
            "at least one native interface should expose an address: {interfaces:?}"
        );
    }
}
