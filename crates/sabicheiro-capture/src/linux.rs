use crate::{
    CaptureError, NetworkCaptureProvider, ObservedSocket, SocketIdentity,
    inet_diag_wire::{DiagSocketRow, parse_datagram, request_bytes},
    infer_direction, interface_model, interfaces_from_sysinfo,
    process::ProcessResolver,
};
use sabicheiro_core::{ConnectionState, NetworkInterface, NetworkProtocol, ObservationSource};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    net::IpAddr,
    process::Command,
};

pub(crate) struct LinuxCaptureProvider {
    processes: ProcessResolver,
    sequence: u32,
}

impl LinuxCaptureProvider {
    pub(crate) fn new() -> Self {
        Self {
            processes: ProcessResolver::new(),
            sequence: 1,
        }
    }
}

impl NetworkCaptureProvider for LinuxCaptureProvider {
    fn name(&self) -> &'static str {
        "linux-inet-diag"
    }

    fn snapshot(&mut self) -> Result<Vec<ObservedSocket>, CaptureError> {
        let mut rows = Vec::new();
        let socket = InetDiagSocket::open()?;
        for family in [libc::AF_INET as u8, libc::AF_INET6 as u8] {
            for protocol in [NetworkProtocol::Tcp, NetworkProtocol::Udp] {
                self.sequence = self.sequence.wrapping_add(1).max(1);
                rows.extend(socket.dump(family, protocol, self.sequence)?);
            }
        }
        let wanted = rows.iter().map(|row| row.inode).collect::<HashSet<_>>();
        let owners = socket_owners(&wanted);
        self.processes.refresh(owners.values().copied());
        Ok(rows
            .into_iter()
            .map(|row| {
                let pid = owners.get(&row.inode).copied();
                let state = linux_state(row.state, row.protocol);
                let direction = infer_direction(&row.local, row.remote.as_ref(), state);
                let interface = interface_name(row.interface_index);
                ObservedSocket {
                    identity: SocketIdentity {
                        pid,
                        local: row.local,
                        remote: row.remote,
                        protocol: row.protocol,
                    },
                    process: self.processes.resolve(pid),
                    direction,
                    state,
                    interface,
                    observation_source: ObservationSource::LinuxInetDiag,
                }
            })
            .collect())
    }

    fn interfaces(&mut self) -> Result<Vec<NetworkInterface>, CaptureError> {
        Ok(linux_interfaces().unwrap_or_else(interfaces_from_sysinfo))
    }
}

fn interface_name(index: u32) -> Option<String> {
    if index == 0 {
        return None;
    }
    let mut buffer = [0_i8; libc::IF_NAMESIZE];
    let result = unsafe { libc::if_indextoname(index, buffer.as_mut_ptr()) };
    if result.is_null() {
        return Some(format!("ifindex:{index}"));
    }
    let name = unsafe { std::ffi::CStr::from_ptr(result) }
        .to_string_lossy()
        .into_owned();
    (!name.is_empty()).then_some(name)
}

struct InetDiagSocket(std::os::fd::RawFd);

impl InetDiagSocket {
    fn open() -> Result<Self, CaptureError> {
        let descriptor = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                libc::NETLINK_INET_DIAG,
            )
        };
        if descriptor < 0 {
            return Err(os_error("opening INET_DIAG socket"));
        }
        let socket = Self(descriptor);
        let mut local: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        local.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        let bind_result = unsafe {
            libc::bind(
                descriptor,
                (&raw const local).cast::<libc::sockaddr>(),
                size_of::<libc::sockaddr_nl>() as libc::socklen_t,
            )
        };
        if bind_result != 0 {
            return Err(os_error("binding INET_DIAG socket"));
        }
        let mut kernel: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        kernel.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        let connect_result = unsafe {
            libc::connect(
                descriptor,
                (&raw const kernel).cast::<libc::sockaddr>(),
                size_of::<libc::sockaddr_nl>() as libc::socklen_t,
            )
        };
        if connect_result != 0 {
            return Err(os_error("connecting INET_DIAG socket"));
        }
        Ok(socket)
    }

    fn dump(
        &self,
        family: u8,
        protocol: NetworkProtocol,
        sequence: u32,
    ) -> Result<Vec<DiagSocketRow>, CaptureError> {
        let request = request_bytes(family, protocol, sequence);
        let sent = unsafe {
            libc::send(
                self.0,
                request.as_ptr().cast(),
                request.len(),
                libc::MSG_NOSIGNAL,
            )
        };
        if sent < 0 || sent as usize != request.len() {
            return Err(os_error("sending INET_DIAG request"));
        }
        let mut rows = Vec::new();
        let mut buffer = vec![0_u8; 4 * 1024 * 1024];
        for _ in 0..1_024 {
            let received = loop {
                let result =
                    unsafe { libc::recv(self.0, buffer.as_mut_ptr().cast(), buffer.len(), 0) };
                if result < 0
                    && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
                {
                    continue;
                }
                break result;
            };
            if received < 0 {
                return Err(os_error("reading INET_DIAG response"));
            }
            if received == 0 {
                return Err(CaptureError::InvalidData(
                    "INET_DIAG closed before completing its dump".into(),
                ));
            }
            if received as usize == buffer.len() {
                return Err(CaptureError::InvalidData(
                    "INET_DIAG datagram exceeded the 4 MiB safety buffer".into(),
                ));
            }
            let (mut batch, done) =
                parse_datagram(&buffer[..received as usize], sequence, protocol)
                    .map_err(CaptureError::InvalidData)?;
            rows.append(&mut batch);
            if rows.len() > 250_000 {
                return Err(CaptureError::InvalidData(
                    "INET_DIAG returned more than 250000 sockets".into(),
                ));
            }
            if done {
                return Ok(rows);
            }
        }
        Err(CaptureError::InvalidData(
            "INET_DIAG dump did not terminate after 1024 datagrams".into(),
        ))
    }
}

impl Drop for InetDiagSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

fn os_error(operation: &str) -> CaptureError {
    let error = std::io::Error::last_os_error();
    if matches!(error.raw_os_error(), Some(libc::EACCES) | Some(libc::EPERM)) {
        CaptureError::PermissionDenied(format!("{operation}: {error}"))
    } else {
        CaptureError::OperatingSystem(format!("{operation}: {error}"))
    }
}

fn linux_interfaces() -> Option<Vec<NetworkInterface>> {
    let output = Command::new("ip")
        .args(["-o", "address", "show"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_linux_interfaces(&String::from_utf8_lossy(&output.stdout))
}

fn parse_linux_interfaces(contents: &str) -> Option<Vec<NetworkInterface>> {
    let mut values: BTreeMap<String, Vec<IpAddr>> = BTreeMap::new();
    for line in contents.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() < 4 || !matches!(fields[2], "inet" | "inet6") {
            continue;
        }
        let name = fields[1].trim_end_matches(':').split('@').next()?;
        let address = fields[3]
            .split('/')
            .next()
            .and_then(|value| value.parse::<IpAddr>().ok());
        if let Some(address) = address {
            values.entry(name.to_owned()).or_default().push(address);
        }
    }
    let interfaces = values
        .into_iter()
        .map(|(name, addresses)| interface_model(name, addresses, true))
        .collect::<Vec<_>>();
    (!interfaces.is_empty()).then_some(interfaces)
}

fn linux_state(value: u8, protocol: NetworkProtocol) -> ConnectionState {
    if protocol == NetworkProtocol::Udp {
        return ConnectionState::Established;
    }
    match value {
        0x01 => ConnectionState::Established,
        0x02 | 0x03 => ConnectionState::Connecting,
        0x0A => ConnectionState::Listening,
        0x04..=0x09 | 0x0B => ConnectionState::Closing,
        _ => ConnectionState::Unknown,
    }
}

fn socket_owners(wanted: &HashSet<u64>) -> HashMap<u64, u32> {
    let mut owners = HashMap::new();
    let Ok(processes) = fs::read_dir("/proc") else {
        return owners;
    };
    for process in processes.flatten() {
        let Some(pid) = process
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(files) = fs::read_dir(process.path().join("fd")) else {
            continue;
        };
        for file in files.flatten() {
            let Ok(target) = fs::read_link(file.path()) else {
                continue;
            };
            let Some(target) = target.to_str() else {
                continue;
            };
            let Some(inode) = target
                .strip_prefix("socket:[")
                .and_then(|value| value.strip_suffix(']'))
                .and_then(|value| value.parse::<u64>().ok())
            else {
                continue;
            };
            if wanted.contains(&inode) {
                owners.entry(inode).or_insert(pid);
            }
        }
        if owners.len() == wanted.len() {
            break;
        }
    }
    owners
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        net::{Ipv4Addr, Ipv6Addr, TcpListener, TcpStream, UdpSocket},
        thread,
        time::Duration,
    };

    #[test]
    fn provider_reads_inet_diag_tables() {
        let mut provider = LinuxCaptureProvider::new();
        let result = provider.snapshot();
        assert!(result.is_ok(), "INET_DIAG read failed: {result:?}");
    }

    #[test]
    fn parses_interface_addresses_from_iproute_output() {
        let interfaces = parse_linux_interfaces(
            "1: lo    inet 127.0.0.1/8 scope host lo\\       valid_lft forever preferred_lft forever\n2: eth0    inet 192.168.1.42/24 brd 192.168.1.255 scope global eth0\\       valid_lft forever preferred_lft forever\n2: eth0    inet6 fe80::1234/64 scope link \\       valid_lft forever preferred_lft forever\n",
        )
        .expect("interfaces");
        let ethernet = interfaces
            .iter()
            .find(|network_interface| network_interface.name == "eth0")
            .expect("eth0");
        assert_eq!(ethernet.addresses.len(), 2);
    }

    #[test]
    fn native_provider_matches_controlled_tcp_udp_ipv4_fixtures_and_owner() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind TCP fixture");
        let server_address = listener.local_addr().expect("TCP server address");
        let client = TcpStream::connect(server_address).expect("connect TCP fixture");
        let client_address = client.local_addr().expect("TCP client address");
        let (server, _) = listener.accept().expect("accept TCP fixture");

        let udp_server = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind UDP server");
        let udp_client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind UDP client");
        let udp_server_address = udp_server.local_addr().expect("UDP server address");
        let udp_client_address = udp_client.local_addr().expect("UDP client address");
        udp_client
            .connect(udp_server_address)
            .expect("connect UDP client");
        udp_server
            .connect(udp_client_address)
            .expect("connect UDP server");
        udp_client
            .send(b"local-linux-fixture")
            .expect("send UDP proof");
        let mut proof = [0_u8; 32];
        udp_server.recv(&mut proof).expect("receive UDP proof");

        let mut provider = LinuxCaptureProvider::new();
        let sockets = provider.snapshot().expect("INET_DIAG fixture snapshot");
        let pid = std::process::id();
        assert!(sockets.iter().any(|socket| {
            socket.identity.protocol == NetworkProtocol::Tcp
                && socket.identity.pid == Some(pid)
                && socket.identity.local.port == client_address.port()
                && socket.identity.remote.as_ref().is_some_and(|remote| {
                    remote.address == server_address.ip() && remote.port == server_address.port()
                })
        }));
        assert!(sockets.iter().any(|socket| {
            socket.identity.protocol == NetworkProtocol::Udp
                && socket.identity.pid == Some(pid)
                && socket.identity.local.port == udp_client_address.port()
        }));

        drop(server);
        drop(client);
        drop(listener);
        drop(udp_server);
        drop(udp_client);
        thread::sleep(Duration::from_millis(20));
        let after = provider.snapshot().expect("post-close INET_DIAG snapshot");
        assert!(!after.iter().any(|socket| {
            socket.identity.local.port == client_address.port()
                || socket.identity.local.port == udp_client_address.port()
        }));
    }

    #[test]
    fn native_provider_reads_ipv6_when_the_local_stack_is_available() {
        let Ok(listener) = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)) else {
            return;
        };
        let address = listener.local_addr().expect("IPv6 listener address");
        let client = TcpStream::connect(address).expect("connect IPv6 fixture");
        let client_address = client.local_addr().expect("IPv6 client address");
        let (server, _) = listener.accept().expect("accept IPv6 fixture");

        let mut provider = LinuxCaptureProvider::new();
        let sockets = provider.snapshot().expect("IPv6 INET_DIAG snapshot");
        assert!(sockets.iter().any(|socket| {
            socket.identity.protocol == NetworkProtocol::Tcp
                && socket.identity.local.address.is_ipv6()
                && socket.identity.local.port == client_address.port()
                && socket.identity.pid == Some(std::process::id())
        }));

        drop(server);
        drop(client);
    }
}
