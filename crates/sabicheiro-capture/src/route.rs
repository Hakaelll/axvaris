use crate::CaptureError;
use sabicheiro_core::{
    CorrelationQuality, NetworkPathEvidence, NetworkPathKind, ObservationSource,
};
use std::net::IpAddr;

pub fn lookup_network_path(
    destination: IpAddr,
    profile_id: impl Into<String>,
    network_generation: u64,
    vpn_observed: bool,
    observed_at_ms: i64,
) -> Result<NetworkPathEvidence, CaptureError> {
    platform_route(
        destination,
        profile_id.into(),
        network_generation,
        vpn_observed,
        observed_at_ms,
    )
}

#[cfg(target_os = "windows")]
fn platform_route(
    destination: IpAddr,
    profile_id: String,
    network_generation: u64,
    vpn_observed: bool,
    observed_at_ms: i64,
) -> Result<NetworkPathEvidence, CaptureError> {
    use windows_sys::Win32::{
        NetworkManagement::IpHelper::{ConvertInterfaceLuidToAlias, GetBestRoute2},
        Networking::WinSock::{
            AF_INET, AF_INET6, IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0, SOCKADDR_IN, SOCKADDR_IN6,
            SOCKADDR_INET,
        },
    };

    let destination_address = match destination {
        IpAddr::V4(address) => SOCKADDR_INET {
            Ipv4: SOCKADDR_IN {
                sin_family: AF_INET,
                sin_port: 0,
                sin_addr: IN_ADDR {
                    S_un: IN_ADDR_0 {
                        S_addr: u32::from_ne_bytes(address.octets()),
                    },
                },
                sin_zero: [0; 8],
            },
        },
        IpAddr::V6(address) => SOCKADDR_INET {
            Ipv6: SOCKADDR_IN6 {
                sin6_family: AF_INET6,
                sin6_port: 0,
                sin6_flowinfo: 0,
                sin6_addr: IN6_ADDR {
                    u: IN6_ADDR_0 {
                        Byte: address.octets(),
                    },
                },
                Anonymous: Default::default(),
            },
        },
    };
    let mut route = windows_sys::Win32::NetworkManagement::IpHelper::MIB_IPFORWARD_ROW2::default();
    let mut source = SOCKADDR_INET::default();
    let status = unsafe {
        GetBestRoute2(
            std::ptr::null(),
            0,
            std::ptr::null(),
            &raw const destination_address,
            0,
            &raw mut route,
            &raw mut source,
        )
    };
    if status != 0 {
        return Err(CaptureError::OperatingSystem(format!(
            "GetBestRoute2 failed with Windows error {status}"
        )));
    }
    let prefix = sockaddr_to_ip(&route.DestinationPrefix.Prefix).unwrap_or(destination);
    let gateway = sockaddr_to_ip(&route.NextHop).filter(|address| !address.is_unspecified());
    let mut alias = [0_u16; 257];
    let alias_status = unsafe {
        ConvertInterfaceLuidToAlias(
            &raw const route.InterfaceLuid,
            alias.as_mut_ptr(),
            alias.len(),
        )
    };
    let interface_id = (alias_status == 0)
        .then(|| {
            let length = alias
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(alias.len());
            String::from_utf16_lossy(&alias[..length])
        })
        .filter(|value| !value.is_empty())
        .or_else(|| Some(format!("ifindex:{}", route.InterfaceIndex)));
    Ok(NetworkPathEvidence {
        id: format!(
            "windows-route:{}:{}:{}",
            profile_id, route.InterfaceIndex, network_generation
        ),
        destination_prefix: prefix.to_string(),
        prefix_length: route.DestinationPrefix.PrefixLength,
        gateway_pseudonym: NetworkPathEvidence::redact_gateway(gateway),
        interface_id,
        metric: Some(route.Metric),
        network_generation,
        profile_id,
        kind: if route.Loopback {
            NetworkPathKind::Direct
        } else if vpn_observed {
            NetworkPathKind::Vpn
        } else if gateway.is_some() {
            NetworkPathKind::Gateway
        } else {
            NetworkPathKind::Direct
        },
        observed_at_ms,
        source: ObservationSource::WindowsIpHelper,
        quality: CorrelationQuality::SnapshotOnly,
        limitation: Some(
            "GetBestRoute2 confirms the route selected at inspection time; an earlier flow event may have used a previous route generation."
                .into(),
        ),
    })
}

#[cfg(target_os = "windows")]
fn sockaddr_to_ip(
    address: &windows_sys::Win32::Networking::WinSock::SOCKADDR_INET,
) -> Option<IpAddr> {
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    let family = unsafe { address.si_family };
    match family {
        AF_INET => Some(IpAddr::V4(std::net::Ipv4Addr::from(unsafe {
            address.Ipv4.sin_addr.S_un.S_addr.to_ne_bytes()
        }))),
        AF_INET6 => Some(IpAddr::V6(std::net::Ipv6Addr::from(unsafe {
            address.Ipv6.sin6_addr.u.Byte
        }))),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn platform_route(
    destination: IpAddr,
    profile_id: String,
    network_generation: u64,
    vpn_observed: bool,
    observed_at_ms: i64,
) -> Result<NetworkPathEvidence, CaptureError> {
    let output = std::process::Command::new("ip")
        .args(["-json", "route", "get", &destination.to_string()])
        .output()
        .map_err(|error| CaptureError::OperatingSystem(format!("starting ip route: {error}")))?;
    if !output.status.success() || output.stdout.len() > 1024 * 1024 {
        return Err(CaptureError::OperatingSystem(
            "ip route get failed or exceeded its bounded output".into(),
        ));
    }
    let values: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .map_err(|error| CaptureError::InvalidData(format!("invalid ip route JSON: {error}")))?;
    let route = values
        .first()
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| CaptureError::InvalidData("ip route returned no route".into()))?;
    let gateway = route
        .get("gateway")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.parse::<IpAddr>().ok());
    let interface_id = route
        .get("dev")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let metric = route
        .get("metric")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let prefix = route
        .get("dst")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| {
            if destination.is_ipv4() {
                "0.0.0.0/0"
            } else {
                "::/0"
            }
        });
    let (destination_prefix, prefix_length) = prefix
        .split_once('/')
        .and_then(|(address, prefix)| Some((address.to_owned(), prefix.parse::<u8>().ok()?)))
        .unwrap_or_else(|| {
            (
                prefix.to_owned(),
                if destination.is_ipv4() { 32 } else { 128 },
            )
        });
    Ok(NetworkPathEvidence {
        id: format!(
            "linux-route:{}:{}:{}",
            profile_id,
            interface_id.as_deref().unwrap_or("unknown"),
            network_generation
        ),
        destination_prefix,
        prefix_length,
        gateway_pseudonym: NetworkPathEvidence::redact_gateway(gateway),
        interface_id,
        metric,
        network_generation,
        profile_id,
        kind: if destination.is_loopback() {
            NetworkPathKind::Direct
        } else if vpn_observed {
            NetworkPathKind::Vpn
        } else if gateway.is_some() {
            NetworkPathKind::Gateway
        } else {
            NetworkPathKind::Direct
        },
        observed_at_ms,
        source: ObservationSource::LinuxInetDiag,
        quality: CorrelationQuality::SnapshotOnly,
        limitation: Some(
            "ip route get confirms the current namespace route; an earlier event may belong to another route generation or namespace."
                .into(),
        ),
    })
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform_route(
    _destination: IpAddr,
    _profile_id: String,
    _network_generation: u64,
    _vpn_observed: bool,
    _observed_at_ms: i64,
) -> Result<NetworkPathEvidence, CaptureError> {
    Err(CaptureError::OperatingSystem(
        "native route inspection is unavailable on this platform".into(),
    ))
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn windows_best_route_resolves_loopback_without_network_egress() {
        let route = lookup_network_path(
            "127.0.0.1".parse().expect("loopback"),
            "fixture-profile",
            1,
            false,
            1_000,
        )
        .expect("loopback route");
        assert_eq!(route.kind, NetworkPathKind::Direct);
        assert!(route.interface_id.is_some());
        assert_eq!(route.quality, CorrelationQuality::SnapshotOnly);
    }
}
