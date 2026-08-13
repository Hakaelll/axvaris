use sabicheiro_core::{Endpoint, NetworkProtocol};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub(crate) const SOCK_DIAG_BY_FAMILY: u16 = 20;
pub(crate) const NLMSG_ERROR: u16 = 2;
pub(crate) const NLMSG_DONE: u16 = 3;
const NLM_F_REQUEST: u16 = 1;
const NLM_F_DUMP: u16 = 0x300;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NetlinkHeader {
    length: u32,
    message_type: u16,
    flags: u16,
    sequence: u32,
    port_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InetDiagSockId {
    source_port: u16,
    destination_port: u16,
    source: [u32; 4],
    destination: [u32; 4],
    interface_index: u32,
    cookie: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InetDiagRequestV2 {
    family: u8,
    protocol: u8,
    extensions: u8,
    pad: u8,
    states: u32,
    id: InetDiagSockId,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InetDiagMessage {
    family: u8,
    state: u8,
    timer: u8,
    retransmits: u8,
    id: InetDiagSockId,
    expires: u32,
    receive_queue: u32,
    write_queue: u32,
    uid: u32,
    inode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagSocketRow {
    pub local: Endpoint,
    pub remote: Option<Endpoint>,
    pub protocol: NetworkProtocol,
    pub state: u8,
    pub inode: u64,
    pub interface_index: u32,
}

pub(crate) fn request_bytes(family: u8, protocol: NetworkProtocol, sequence: u32) -> Vec<u8> {
    let request = InetDiagRequestV2 {
        family,
        protocol: match protocol {
            NetworkProtocol::Tcp => 6,
            NetworkProtocol::Udp => 17,
        },
        states: u32::MAX,
        id: InetDiagSockId {
            cookie: [u32::MAX; 2],
            ..InetDiagSockId::default()
        },
        ..InetDiagRequestV2::default()
    };
    let header = NetlinkHeader {
        length: (size_of::<NetlinkHeader>() + size_of::<InetDiagRequestV2>()) as u32,
        message_type: SOCK_DIAG_BY_FAMILY,
        flags: NLM_F_REQUEST | NLM_F_DUMP,
        sequence,
        port_id: 0,
    };
    let mut bytes = Vec::with_capacity(header.length as usize);
    append_struct(&mut bytes, &header);
    append_struct(&mut bytes, &request);
    bytes
}

pub(crate) fn parse_datagram(
    bytes: &[u8],
    expected_sequence: u32,
    protocol: NetworkProtocol,
) -> Result<(Vec<DiagSocketRow>, bool), String> {
    let mut offset = 0_usize;
    let mut rows = Vec::new();
    let mut done = false;
    while offset < bytes.len() {
        if bytes.len() - offset < size_of::<NetlinkHeader>() {
            return Err("truncated netlink header".into());
        }
        let header = read_struct::<NetlinkHeader>(&bytes[offset..])?;
        let length = header.length as usize;
        if length < size_of::<NetlinkHeader>() || offset.saturating_add(length) > bytes.len() {
            return Err("invalid netlink message length".into());
        }
        if header.sequence != expected_sequence {
            offset = offset.saturating_add(align4(length));
            continue;
        }
        let payload = &bytes[offset + size_of::<NetlinkHeader>()..offset + length];
        match header.message_type {
            NLMSG_DONE => done = true,
            NLMSG_ERROR => {
                if payload.len() < size_of::<i32>() {
                    return Err("truncated netlink error".into());
                }
                let code = i32::from_ne_bytes(payload[..4].try_into().expect("four bytes"));
                if code != 0 {
                    return Err(format!("INET_DIAG returned Linux error {}", -code));
                }
            }
            SOCK_DIAG_BY_FAMILY => {
                let message = read_struct::<InetDiagMessage>(payload)?;
                let local_address = decode_address(message.family, message.id.source)?;
                let remote_address = decode_address(message.family, message.id.destination)?;
                let local = Endpoint::new(local_address, u16::from_be(message.id.source_port));
                let parsed_remote =
                    Endpoint::new(remote_address, u16::from_be(message.id.destination_port));
                let remote = (!parsed_remote.address.is_unspecified() || parsed_remote.port != 0)
                    .then_some(parsed_remote);
                rows.push(DiagSocketRow {
                    local,
                    remote,
                    protocol,
                    state: message.state,
                    inode: u64::from(message.inode),
                    interface_index: message.id.interface_index,
                });
            }
            _ => {}
        }
        offset = offset.saturating_add(align4(length));
    }
    Ok((rows, done))
}

fn decode_address(family: u8, words: [u32; 4]) -> Result<IpAddr, String> {
    let mut bytes = [0_u8; 16];
    for (index, word) in words.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_ne_bytes());
    }
    match family as i32 {
        2 => Ok(IpAddr::V4(Ipv4Addr::new(
            bytes[0], bytes[1], bytes[2], bytes[3],
        ))),
        10 => Ok(IpAddr::V6(Ipv6Addr::from(bytes))),
        other => Err(format!("unsupported INET_DIAG address family {other}")),
    }
}

fn append_struct<T: Copy>(bytes: &mut Vec<u8>, value: &T) {
    let raw =
        unsafe { std::slice::from_raw_parts((value as *const T).cast::<u8>(), size_of::<T>()) };
    bytes.extend_from_slice(raw);
}

fn read_struct<T: Copy>(bytes: &[u8]) -> Result<T, String> {
    if bytes.len() < size_of::<T>() {
        return Err("truncated INET_DIAG structure".into());
    }
    Ok(unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<T>()) })
}

const fn align4(value: usize) -> usize {
    (value + 3) & !3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_matches_linux_uapi_layout() {
        let request = request_bytes(2, NetworkProtocol::Tcp, 7);
        assert_eq!(request.len(), 72);
        assert_eq!(
            u32::from_ne_bytes(request[0..4].try_into().expect("length")),
            72
        );
        assert_eq!(
            u16::from_ne_bytes(request[4..6].try_into().expect("type")),
            20
        );
        assert_eq!(request[16], 2);
        assert_eq!(request[17], 6);
        assert_eq!(
            u32::from_ne_bytes(request[20..24].try_into().expect("states")),
            u32::MAX
        );
    }

    #[test]
    fn parses_ipv4_diag_response_and_done_marker() {
        let mut message = InetDiagMessage {
            family: 2,
            state: 1,
            inode: 42,
            ..InetDiagMessage::default()
        };
        message.id.source_port = 50_000_u16.to_be();
        message.id.destination_port = 443_u16.to_be();
        message.id.source[0] = u32::from_ne_bytes([127, 0, 0, 1]);
        message.id.destination[0] = u32::from_ne_bytes([1, 1, 1, 1]);
        let mut bytes = Vec::new();
        let header = NetlinkHeader {
            length: (size_of::<NetlinkHeader>() + size_of::<InetDiagMessage>()) as u32,
            message_type: SOCK_DIAG_BY_FAMILY,
            sequence: 9,
            ..NetlinkHeader::default()
        };
        append_struct(&mut bytes, &header);
        append_struct(&mut bytes, &message);
        let done = NetlinkHeader {
            length: size_of::<NetlinkHeader>() as u32,
            message_type: NLMSG_DONE,
            sequence: 9,
            ..NetlinkHeader::default()
        };
        append_struct(&mut bytes, &done);

        let (rows, complete) =
            parse_datagram(&bytes, 9, NetworkProtocol::Tcp).expect("valid datagram");
        assert!(complete);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].local.label(), "127.0.0.1:50000");
        assert_eq!(
            rows[0].remote.as_ref().expect("remote").label(),
            "1.1.1.1:443"
        );
        assert_eq!(rows[0].inode, 42);
    }
}
