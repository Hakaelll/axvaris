use crate::{
    AsnInfo, ConnectionKey, ConnectionState, Direction, Endpoint, GeoLocation, NetworkEventKind,
    NetworkProtocol, NormalizedNetworkEvent, ProcessInfo, TrafficCounters,
};
use std::{collections::HashMap, net::IpAddr};

#[derive(Debug, Clone)]
struct DemoDestination {
    process: &'static str,
    executable: &'static str,
    pid: u32,
    ip: &'static str,
    domain: &'static str,
    country_code: &'static str,
    country_name: &'static str,
    city: &'static str,
    latitude: f64,
    longitude: f64,
    asn: u32,
    organization: &'static str,
    protocol: NetworkProtocol,
    port: u16,
}

const DESTINATIONS: &[DemoDestination] = &[
    DemoDestination {
        process: "Firefox",
        executable: "firefox.exe",
        pid: 4_812,
        ip: "142.250.184.206",
        domain: "google.com",
        country_code: "US",
        country_name: "United States",
        city: "Mountain View",
        latitude: 37.386,
        longitude: -122.084,
        asn: 15_169,
        organization: "Google LLC",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Discord",
        executable: "Discord.exe",
        pid: 7_204,
        ip: "162.159.135.234",
        domain: "gateway.discord.gg",
        country_code: "NL",
        country_name: "Netherlands",
        city: "Amsterdam",
        latitude: 52.3676,
        longitude: 4.9041,
        asn: 13_335,
        organization: "Cloudflare, Inc.",
        protocol: NetworkProtocol::Udp,
        port: 443,
    },
    DemoDestination {
        process: "Steam",
        executable: "steam.exe",
        pid: 9_140,
        ip: "23.45.67.89",
        domain: "cdn.steampowered.com",
        country_code: "DE",
        country_name: "Germany",
        city: "Frankfurt",
        latitude: 50.1109,
        longitude: 8.6821,
        asn: 20_001,
        organization: "Akamai Technologies",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Windows Services",
        executable: "svchost.exe",
        pid: 1_268,
        ip: "13.107.42.12",
        domain: "settings-win.data.microsoft.com",
        country_code: "IE",
        country_name: "Ireland",
        city: "Dublin",
        latitude: 53.3498,
        longitude: -6.2603,
        asn: 80_75,
        organization: "Microsoft Corporation",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Dropbox",
        executable: "Dropbox.exe",
        pid: 5_622,
        ip: "162.125.6.6",
        domain: "client.dropbox.com",
        country_code: "US",
        country_name: "United States",
        city: "San Jose",
        latitude: 37.3382,
        longitude: -121.8863,
        asn: 19_670,
        organization: "Dropbox, Inc.",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Firefox",
        executable: "firefox.exe",
        pid: 4_812,
        ip: "104.16.123.96",
        domain: "developer.mozilla.org",
        country_code: "GB",
        country_name: "United Kingdom",
        city: "London",
        latitude: 51.5072,
        longitude: -0.1276,
        asn: 13_335,
        organization: "Cloudflare, Inc.",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Spotify",
        executable: "Spotify.exe",
        pid: 6_410,
        ip: "35.186.224.25",
        domain: "audio-fa.scdn.co",
        country_code: "SE",
        country_name: "Sweden",
        city: "Stockholm",
        latitude: 59.3293,
        longitude: 18.0686,
        asn: 15_169,
        organization: "Google Cloud",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Git",
        executable: "git.exe",
        pid: 10_422,
        ip: "140.82.121.4",
        domain: "github.com",
        country_code: "US",
        country_name: "United States",
        city: "San Francisco",
        latitude: 37.7749,
        longitude: -122.4194,
        asn: 36_444,
        organization: "GitHub, Inc.",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
];

#[derive(Debug, Clone)]
pub struct DemoGenerator {
    random: XorShift64,
    active: HashMap<usize, ConnectionKey>,
    tick: u64,
}

impl DemoGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            random: XorShift64::new(seed),
            active: HashMap::new(),
            tick: 0,
        }
    }

    pub fn next_batch(&mut self, timestamp_ms: i64) -> Vec<NormalizedNetworkEvent> {
        self.tick = self.tick.saturating_add(1);
        let count = 1 + self.random.range(3) as usize;
        let mut events = Vec::with_capacity(count + 1);
        for _ in 0..count {
            let index = self.random.range(DESTINATIONS.len() as u64) as usize;
            let destination = &DESTINATIONS[index];
            let kind = if self.active.contains_key(&index) {
                NetworkEventKind::Updated
            } else {
                NetworkEventKind::Opened
            };
            let key = self
                .active
                .entry(index)
                .or_insert_with(|| demo_key(destination))
                .clone();
            events.push(demo_event(
                destination,
                key,
                kind,
                timestamp_ms,
                &mut self.random,
            ));
        }

        if self.tick % 7 == 0 && !self.active.is_empty() {
            let active_index = self.random.range(self.active.len() as u64) as usize;
            if let Some((&destination_index, key)) = self.active.iter().nth(active_index) {
                let destination = &DESTINATIONS[destination_index];
                events.push(demo_event(
                    destination,
                    key.clone(),
                    NetworkEventKind::Closed,
                    timestamp_ms,
                    &mut self.random,
                ));
                self.active.remove(&destination_index);
            }
        }
        events
    }
}

fn demo_key(destination: &DemoDestination) -> ConnectionKey {
    let local_port = 40_000 + (destination.pid % 20_000) as u16;
    ConnectionKey {
        pid: Some(destination.pid),
        local: Endpoint::new(
            "192.168.1.42".parse::<IpAddr>().expect("valid demo IP"),
            local_port,
        ),
        remote: Endpoint::new(
            destination.ip.parse().expect("valid demo IP"),
            destination.port,
        ),
        protocol: destination.protocol,
    }
}

fn demo_event(
    destination: &DemoDestination,
    key: ConnectionKey,
    kind: NetworkEventKind,
    timestamp_ms: i64,
    random: &mut XorShift64,
) -> NormalizedNetworkEvent {
    let received = 18_000 + random.range(420_000);
    let sent = 2_000 + random.range(78_000);
    NormalizedNetworkEvent {
        kind,
        key,
        timestamp_ms,
        process: ProcessInfo {
            pid: Some(destination.pid),
            name: destination.process.into(),
            executable: Some(destination.executable.into()),
            executable_path: Some(format!("C:\\Program Files\\{}", destination.executable)),
            started_at_ms: Some(timestamp_ms.saturating_sub(3_600_000)),
            icon_data_url: None,
        },
        direction: Direction::Outbound,
        state: if kind == NetworkEventKind::Closed {
            ConnectionState::Closed
        } else {
            ConnectionState::Established
        },
        interface: Some("Wi-Fi".into()),
        hostname: Some(destination.domain.into()),
        domain: Some(destination.domain.into()),
        geo: Some(GeoLocation {
            country_code: destination.country_code.into(),
            country_name: destination.country_name.into(),
            region: None,
            city: Some(destination.city.into()),
            latitude: destination.latitude,
            longitude: destination.longitude,
            source: "Sabicheiro deterministic demo".into(),
        }),
        asn: Some(AsnInfo {
            number: destination.asn,
            organization: destination.organization.into(),
        }),
        traffic_delta: TrafficCounters {
            bytes_sent: sent,
            bytes_received: received,
            packets_sent: (sent / 1_200).max(1),
            packets_received: (received / 1_200).max(1),
        },
    }
}

#[derive(Debug, Clone)]
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, upper: u64) -> u64 {
        debug_assert!(upper > 0);
        self.next() % upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_is_deterministic_for_seed_and_time() {
        let mut left = DemoGenerator::new(1234);
        let mut right = DemoGenerator::new(1234);
        for timestamp in [1_000, 1_250, 1_500, 1_750, 2_000] {
            assert_eq!(left.next_batch(timestamp), right.next_batch(timestamp));
        }
    }

    #[test]
    fn demo_contains_realistic_enriched_events() {
        let event = DemoGenerator::new(7)
            .next_batch(1_000)
            .into_iter()
            .next()
            .expect("demo event");
        assert!(event.domain.is_some());
        assert!(event.geo.is_some());
        assert!(event.key.pid.is_some());
        assert!(event.traffic_delta.bytes_received > 0);
    }
}
