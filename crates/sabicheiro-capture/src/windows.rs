use crate::{
    CaptureError, NetworkCaptureProvider, ObservedSocket, SocketIdentity, infer_direction,
    interfaces_from_sysinfo, process::ProcessResolver,
};
use sabicheiro_core::{ConnectionState, Endpoint, NetworkInterface, NetworkProtocol};
use std::{
    ffi::c_void,
    mem::size_of,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
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
        // sysinfo uses GetAdaptersAddresses on Windows. Unlike redirected Windows
        // PowerShell output, its UTF-16 friendly names and unicast addresses are
        // converted without a legacy code-page round trip.
        Ok(interfaces_from_sysinfo())
    }
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
