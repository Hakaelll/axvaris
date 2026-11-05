use crate::{
    AsnInfo, ConnectionEvidence, ConnectionKey, ConnectionState, Direction, Endpoint, GeoLocation,
    GeoPrecision, NetworkEventKind, NetworkProtocol, NormalizedNetworkEvent, ObservationSource,
    PlacementKind, ProcessInfo, TrafficCounters,
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
        process: "Demo Browser",
        executable: "demo-browser.exe",
        pid: 4_812,
        ip: "192.0.2.10",
        domain: "portal.invalid",
        country_code: "ES",
        country_name: "Spain",
        city: "Madrid",
        latitude: 40.4168,
        longitude: -3.7038,
        asn: 64_496,
        organization: "Example Network A",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Demo Browser",
        executable: "demo-browser.exe",
        pid: 4_812,
        ip: "198.51.100.20",
        domain: "media.invalid",
        country_code: "US",
        country_name: "United States",
        city: "New York",
        latitude: 40.7128,
        longitude: -74.006,
        asn: 64_497,
        organization: "Example Network B",
        protocol: NetworkProtocol::Udp,
        port: 443,
    },
    DemoDestination {
        process: "Demo Updater",
        executable: "demo-updater.exe",
        pid: 7_204,
        ip: "203.0.113.30",
        domain: "updates.invalid",
        country_code: "JP",
        country_name: "Japan",
        city: "Tokyo",
        latitude: 35.6762,
        longitude: 139.6503,
        asn: 64_498,
        organization: "Example Network C",
        protocol: NetworkProtocol::Tcp,
        port: 443,
    },
    DemoDestination {
        process: "Demo Updater",
        executable: "demo-updater.exe",
        pid: 7_204,
        ip: "192.0.2.40",
        domain: "status.invalid",
        country_code: "DE",
        country_name: "Germany",
        city: "Frankfurt",
        latitude: 50.1109,
        longitude: 8.6821,
        asn: 64_499,
        organization: "Example Network D",
        protocol: NetworkProtocol::Udp,
        port: 53,
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
        if self.tick.is_multiple_of(7) && !self.active.is_empty() {
            let active_index = self.random.range(self.active.len() as u64) as usize;
            if let Some((&destination_index, key)) = self.active.iter().nth(active_index) {
                let destination = &DESTINATIONS[destination_index];
                let event = demo_event(
                    destination,
                    key.clone(),
                    NetworkEventKind::Closed,
                    timestamp_ms,
                    &mut self.random,
                );
                self.active.remove(&destination_index);
                return vec![event];
            }
        }
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
        vec![demo_event(
            destination,
            key,
            kind,
            timestamp_ms,
            &mut self.random,
        )]
    }
}

fn demo_key(destination: &DemoDestination) -> ConnectionKey {
    let local_port = 40_000 + (destination.pid % 20_000) as u16;
    ConnectionKey {
        pid: Some(destination.pid),
        process_instance: Some(format!("demo-process:{}", destination.pid)),
        local: Endpoint::new(
            "192.168.1.42".parse::<IpAddr>().expect("valid demo IP"),
            local_port,
        ),
        remote: Endpoint::new(
            destination.ip.parse().expect("valid demo IP"),
            destination.port,
        ),
        protocol: destination.protocol,
        interface_id: Some("demo-wifi".into()),
        capture_generation: 1,
        flow_generation: u64::from(destination.pid),
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
    let state = if kind == NetworkEventKind::Closed {
        ConnectionState::Closed
    } else {
        ConnectionState::Established
    };
    let evidence =
        ConnectionEvidence::for_peer(&key.remote, state, ObservationSource::Demo, timestamp_ms);
    NormalizedNetworkEvent {
        kind,
        key,
        evidence,
        timestamp_ms,
        process: ProcessInfo {
            pid: Some(destination.pid),
            name: destination.process.into(),
            executable: Some(destination.executable.into()),
            executable_path: Some(format!("C:\\Program Files\\{}", destination.executable)),
            started_at_ms: Some(timestamp_ms.saturating_sub(3_600_000)),
            icon_data_url: None,
            ..ProcessInfo::default()
        },
        direction: Direction::Outbound,
        state,
        interface: Some("Wi-Fi".into()),
        hostname: Some(destination.domain.into()),
        domain: Some(destination.domain.into()),
        geo: Some(GeoLocation {
            country_code: Some(destination.country_code.into()),
            country_name: Some(destination.country_name.into()),
            region: None,
            city: Some(destination.city.into()),
            latitude: Some(destination.latitude),
            longitude: Some(destination.longitude),
            source: "Sabicheiro deterministic demo".into(),
            precision: GeoPrecision::Coordinates,
            placement: PlacementKind::DatabaseCoordinate,
            dataset_kind: Some("synthetic-demo".into()),
            dataset_edition: Some("Sabicheiro Demo 0.3".into()),
            ..GeoLocation::default()
        }),
        asn: Some(AsnInfo {
            number: destination.asn,
            organization: Some(destination.organization.into()),
            dataset_edition: Some("Sabicheiro Demo ASN 0.3".into()),
            generation: 1,
            ..AsnInfo::default()
        }),
        traffic_delta: TrafficCounters {
            bytes_sent: sent,
            bytes_received: received,
            packets_sent: (sent / 1_200).max(1),
            packets_received: (received / 1_200).max(1),
        },
        counter_source: crate::CounterSource::Demo,
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

    #[test]
    fn demo_is_bounded_to_four_destinations_and_one_event_per_batch() {
        let mut demo = DemoGenerator::new(9);
        let mut remote_addresses = std::collections::HashSet::new();
        for second in 0..600 {
            let events = demo.next_batch(1_000 + second * 1_000);
            assert!(events.len() <= 1);
            for event in events {
                remote_addresses.insert(event.key.remote.address);
                assert!(
                    event
                        .domain
                        .as_deref()
                        .is_some_and(|name| name.ends_with(".invalid"))
                );
            }
            assert!(demo.active.len() <= 4);
        }
        assert!(remote_addresses.len() <= 4);
    }
}
