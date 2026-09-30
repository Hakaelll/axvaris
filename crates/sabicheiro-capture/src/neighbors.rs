use crate::CaptureError;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Entries are read from the operating system's existing neighbor cache. This
/// does not probe the LAN and therefore does not enumerate every connected host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalNeighbor {
    pub address: IpAddr,
    pub mac_address: Option<String>,
    pub interface_id: String,
    pub is_router: bool,
    pub state: String,
}

fn format_mac(bytes: &[u8]) -> Option<String> {
    (bytes.len() == 6 && bytes[0] & 1 == 0 && bytes.iter().any(|byte| *byte != 0)).then(|| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    })
}

#[cfg(target_os = "windows")]
pub fn local_neighbors() -> Result<Vec<LocalNeighbor>, CaptureError> {
    use std::ptr;
    use windows_sys::Win32::{
        Foundation::ERROR_NOT_FOUND,
        NetworkManagement::IpHelper::{FreeMibTable, GetIpNetTable2, MIB_IPNET_TABLE2},
        Networking::WinSock::{AF_UNSPEC, NlnsIncomplete, NlnsUnreachable},
    };

    let mut table: *mut MIB_IPNET_TABLE2 = ptr::null_mut();
    let status = unsafe { GetIpNetTable2(AF_UNSPEC, &mut table) };
    if status == ERROR_NOT_FOUND {
        return Ok(Vec::new());
    }
    if status != 0 {
        return Err(CaptureError::OperatingSystem(format!(
            "GetIpNetTable2 failed with Windows error {status}"
        )));
    }
    if table.is_null() {
        return Err(CaptureError::InvalidData(
            "GetIpNetTable2 returned a null table".into(),
        ));
    }
    struct TableGuard(*mut MIB_IPNET_TABLE2);
    impl Drop for TableGuard {
        fn drop(&mut self) {
            unsafe { FreeMibTable(self.0.cast()) };
        }
    }
    let _guard = TableGuard(table);
    let count = (unsafe { (*table).NumEntries } as usize).min(8_192);
    let rows = unsafe { std::slice::from_raw_parts((*table).Table.as_ptr(), count) };
    let mut neighbors = Vec::with_capacity(count);
    for row in rows {
        if row.State == NlnsIncomplete || row.State == NlnsUnreachable {
            continue;
        }
        let Some(address) = (unsafe { crate::windows::inet_address(&row.Address) }) else {
            continue;
        };
        if address.is_multicast() || address.is_loopback() || address.is_unspecified() {
            continue;
        }
        let mac_length = (row.PhysicalAddressLength as usize).min(row.PhysicalAddress.len());
        neighbors.push(LocalNeighbor {
            address,
            mac_address: format_mac(&row.PhysicalAddress[..mac_length]),
            interface_id: row.InterfaceIndex.to_string(),
            is_router: unsafe { row.Anonymous.Flags } & 1 != 0,
            state: match row.State {
                5 => "reachable",
                4 => "stale",
                3 => "delay",
                2 => "probe",
                6 => "permanent",
                _ => "other",
            }
            .into(),
        });
    }
    Ok(neighbors)
}

#[cfg(target_os = "linux")]
pub fn local_neighbors() -> Result<Vec<LocalNeighbor>, CaptureError> {
    use std::process::Command;
    let output = Command::new("ip").args(["-j", "neigh", "show"]).output()?;
    if !output.status.success() {
        return Err(CaptureError::OperatingSystem(
            "ip -j neigh show failed; install iproute2".into(),
        ));
    }
    parse_ip_neighbors(&output.stdout)
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn local_neighbors() -> Result<Vec<LocalNeighbor>, CaptureError> {
    Err(CaptureError::Unsupported)
}

#[cfg(any(target_os = "linux", test))]
fn parse_ip_neighbors(bytes: &[u8]) -> Result<Vec<LocalNeighbor>, CaptureError> {
    use serde_json::Value;
    if bytes.len() > 1024 * 1024 {
        return Err(CaptureError::InvalidData(
            "neighbor table exceeds 1 MiB".into(),
        ));
    }
    let entries = serde_json::from_slice::<Vec<Value>>(bytes)
        .map_err(|error| CaptureError::InvalidData(error.to_string()))?;
    Ok(entries
        .into_iter()
        .take(8_192)
        .filter_map(|entry| {
            let address = entry.get("dst")?.as_str()?.parse::<IpAddr>().ok()?;
            if address.is_multicast() || address.is_loopback() || address.is_unspecified() {
                return None;
            }
            let state = entry
                .get("state")
                .and_then(|value| {
                    value
                        .as_str()
                        .or_else(|| value.as_array()?.first()?.as_str())
                })
                .unwrap_or("unknown");
            if matches!(state, "INCOMPLETE" | "FAILED") {
                return None;
            }
            let mac_address = entry
                .get("lladdr")
                .and_then(Value::as_str)
                .and_then(parse_mac);
            Some(LocalNeighbor {
                address,
                mac_address,
                interface_id: entry
                    .get("dev")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                is_router: entry
                    .get("router")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                state: state.to_ascii_lowercase(),
            })
        })
        .collect())
}

#[cfg(any(target_os = "linux", test))]
fn parse_mac(value: &str) -> Option<String> {
    let octets = value
        .split(':')
        .map(|octet| u8::from_str_radix(octet, 16).ok())
        .collect::<Option<Vec<_>>>()?;
    format_mac(&octets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_addresses_require_six_real_octets() {
        assert_eq!(
            format_mac(&[0, 17, 34, 51, 68, 85]).as_deref(),
            Some("00:11:22:33:44:55")
        );
        assert_eq!(format_mac(&[0; 6]), None);
        assert_eq!(format_mac(&[255; 6]), None);
        assert_eq!(format_mac(&[1, 2, 3]), None);
    }

    #[test]
    fn linux_neighbors_ignore_incomplete_and_invalid_entries() {
        let json = br#"[
          {"dst":"192.168.1.1","dev":"wlan0","lladdr":"00:11:22:33:44:55","state":"REACHABLE"},
          {"dst":"fe80::1","dev":"wlan0","lladdr":"00:11:22:33:44:55","state":"STALE","router":true},
          {"dst":"192.168.1.2","dev":"wlan0","state":["INCOMPLETE"]},
          {"dst":"bad","dev":"wlan0","state":"REACHABLE"}
        ]"#;
        let entries = parse_ip_neighbors(json).expect("valid neighbor JSON");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].mac_address, entries[1].mac_address);
        assert!(entries[1].is_router);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_neighbor_cache_is_readable_without_a_scan() {
        let entries = local_neighbors().expect("read local Windows neighbor cache");
        assert!(entries.len() <= 8_192);
        assert!(entries.iter().all(|entry| !entry.address.is_multicast()));
    }
}
