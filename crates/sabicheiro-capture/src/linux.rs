use crate::{
    CaptureError, NetworkCaptureProvider, ObservedSocket, SocketIdentity, infer_direction,
    interface_model, interfaces_from_sysinfo, process::ProcessResolver,
};
use sabicheiro_core::{ConnectionState, Endpoint, NetworkInterface, NetworkProtocol};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::Path,
    process::Command,
};

pub(crate) struct LinuxCaptureProvider {
    processes: ProcessResolver,
}

impl LinuxCaptureProvider {
    pub(crate) fn new() -> Self {
        Self {
            processes: ProcessResolver::new(),
        }
    }
}

impl NetworkCaptureProvider for LinuxCaptureProvider {
    fn name(&self) -> &'static str {
        "linux-proc-sockets"
    }

    fn snapshot(&mut self) -> Result<Vec<ObservedSocket>, CaptureError> {
        let mut rows = Vec::new();
        read_table(
            Path::new("/proc/net/tcp"),
            NetworkProtocol::Tcp,
            false,
            &mut rows,
        )?;
        read_table(
            Path::new("/proc/net/tcp6"),
            NetworkProtocol::Tcp,
            true,
            &mut rows,
        )?;
        read_table(
            Path::new("/proc/net/udp"),
            NetworkProtocol::Udp,
            false,
            &mut rows,
        )?;
        read_table(
            Path::new("/proc/net/udp6"),
            NetworkProtocol::Udp,
            true,
            &mut rows,
        )?;
        let wanted = rows.iter().map(|row| row.inode).collect::<HashSet<_>>();
        let owners = socket_owners(&wanted);
        self.processes.refresh(owners.values().copied());
        Ok(rows
            .into_iter()
            .map(|row| {
                let pid = owners.get(&row.inode).copied();
                let direction = infer_direction(&row.local, row.remote.as_ref(), row.state);
                ObservedSocket {
                    identity: SocketIdentity {
                        pid,
                        local: row.local,
                        remote: row.remote,
                        protocol: row.protocol,
                    },
                    process: self.processes.resolve(pid),
                    direction,
                    state: row.state,
                    interface: None,
                }
            })
            .collect())
    }

    fn interfaces(&mut self) -> Result<Vec<NetworkInterface>, CaptureError> {
        Ok(linux_interfaces().unwrap_or_else(interfaces_from_sysinfo))
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

#[derive(Debug)]
struct ProcSocketRow {
    local: Endpoint,
    remote: Option<Endpoint>,
    protocol: NetworkProtocol,
    state: ConnectionState,
    inode: u64,
}

fn read_table(
    path: &Path,
    protocol: NetworkProtocol,
    ipv6: bool,
    rows: &mut Vec<ProcSocketRow>,
) -> Result<(), CaptureError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(CaptureError::PermissionDenied(path.display().to_string()));
        }
        Err(error) => return Err(error.into()),
    };
    for (line_number, line) in contents.lines().enumerate().skip(1) {
        match parse_proc_line(line, protocol, ipv6) {
            Ok(row) => rows.push(row),
            Err(error) => tracing::debug!(
                path = %path.display(),
                line = line_number + 1,
                %error,
                "skipping malformed proc socket row"
            ),
        }
    }
    Ok(())
}

fn parse_proc_line(
    line: &str,
    protocol: NetworkProtocol,
    ipv6: bool,
) -> Result<ProcSocketRow, CaptureError> {
    let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() < 10 {
        return Err(CaptureError::InvalidData(
            "proc row has fewer than ten fields".into(),
        ));
    }
    let local = parse_proc_endpoint(fields[1], ipv6)?;
    let parsed_remote = parse_proc_endpoint(fields[2], ipv6)?;
    let state_value = u8::from_str_radix(fields[3], 16)
        .map_err(|_| CaptureError::InvalidData("invalid proc socket state".into()))?;
    let state = linux_state(state_value, protocol);
    let remote = (!parsed_remote.address.is_unspecified() || parsed_remote.port != 0)
        .then_some(parsed_remote);
    let inode = fields[9]
        .parse::<u64>()
        .map_err(|_| CaptureError::InvalidData("invalid socket inode".into()))?;
    Ok(ProcSocketRow {
        local,
        remote,
        protocol,
        state,
        inode,
    })
}

fn parse_proc_endpoint(value: &str, ipv6: bool) -> Result<Endpoint, CaptureError> {
    let (address, port) = value
        .split_once(':')
        .ok_or_else(|| CaptureError::InvalidData("endpoint lacks port separator".into()))?;
    let port = u16::from_str_radix(port, 16)
        .map_err(|_| CaptureError::InvalidData("invalid hexadecimal port".into()))?;
    let address = if ipv6 {
        if address.len() != 32 {
            return Err(CaptureError::InvalidData(
                "invalid IPv6 field length".into(),
            ));
        }
        let mut bytes = [0_u8; 16];
        for (index, chunk) in address.as_bytes().chunks_exact(8).enumerate() {
            let chunk = std::str::from_utf8(chunk)
                .map_err(|_| CaptureError::InvalidData("non-UTF8 IPv6 field".into()))?;
            let word = u32::from_str_radix(chunk, 16)
                .map_err(|_| CaptureError::InvalidData("invalid IPv6 field".into()))?;
            bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        IpAddr::V6(Ipv6Addr::from(bytes))
    } else {
        let numeric = u32::from_str_radix(address, 16)
            .map_err(|_| CaptureError::InvalidData("invalid IPv4 field".into()))?;
        IpAddr::V4(Ipv4Addr::from(numeric.to_le_bytes()))
    };
    Ok(Endpoint::new(address, port))
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

    #[test]
    fn parses_ipv4_proc_row() {
        let row = parse_proc_line(
            "0: 0100007F:C350 01010101:01BB 01 00000000:00000000 00:00000000 00000000 1000 0 12345",
            NetworkProtocol::Tcp,
            false,
        )
        .expect("row");
        assert_eq!(row.local.address.to_string(), "127.0.0.1");
        assert_eq!(row.local.port, 50_000);
        assert_eq!(row.remote.expect("remote").port, 443);
        assert_eq!(row.inode, 12_345);
    }

    #[test]
    fn provider_reads_proc_tables() {
        let mut provider = LinuxCaptureProvider::new();
        let result = provider.snapshot();
        assert!(result.is_ok(), "proc table read failed: {result:?}");
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
}
