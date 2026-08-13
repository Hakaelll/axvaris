use crate::{ConnectionKey, GeoLocation, SignalKind, UnusualSignal};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct ProcessBaseline {
    samples: u64,
    destinations: HashSet<String>,
    countries: HashSet<String>,
    ports: HashSet<u16>,
    recent_connections_ms: VecDeque<i64>,
    transfer_samples: u64,
    mean_bytes_per_second: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaselineBook {
    processes: HashMap<String, ProcessBaseline>,
}

#[derive(Debug, Clone)]
pub struct BaselineObservation<'a> {
    pub process_key: &'a str,
    pub destination_key: &'a str,
    pub connection: &'a ConnectionKey,
    pub geo: Option<&'a GeoLocation>,
    pub timestamp_ms: i64,
}

impl BaselineBook {
    pub fn observe(&mut self, observation: BaselineObservation<'_>) -> Vec<UnusualSignal> {
        let previously_seen_process = self.processes.contains_key(observation.process_key);
        let has_any_process_history = !self.processes.is_empty();
        let baseline = self
            .processes
            .entry(observation.process_key.to_owned())
            .or_default();
        let mut signals = Vec::new();

        if has_any_process_history && !previously_seen_process {
            signals.push(UnusualSignal {
                kind: SignalKind::NewNetworkProcess,
                title: "New network-enabled process".into(),
                reason: format!(
                    "This is the first locally retained network connection for {}.",
                    observation.process_key
                ),
                typical: Some("This is an observation, not a security verdict.".into()),
            });
        }

        let new_destination =
            baseline.samples > 0 && !baseline.destinations.contains(observation.destination_key);
        if new_destination {
            signals.push(UnusualSignal {
                kind: SignalKind::NewDestination,
                title: "New destination".into(),
                reason: format!(
                    "First observed connection from {} to {}.",
                    observation.process_key, observation.destination_key
                ),
                typical: None,
            });
        }

        if new_destination && baseline.samples >= 11 && baseline.destinations.len() == 11 {
            signals.push(UnusualSignal {
                kind: SignalKind::DestinationDiversity,
                title: "Higher destination diversity".into(),
                reason: format!(
                    "{} has now contacted 12 distinct destinations in the retained local baseline.",
                    observation.process_key
                ),
                typical: Some("Previously observed destination set: 11 or fewer.".into()),
            });
        }

        if let Some(geo) = observation.geo {
            let country_code = geo.country_code.as_deref();
            if baseline.samples >= 3
                && country_code.is_some_and(|code| !baseline.countries.contains(code))
            {
                let mut typical = baseline.countries.iter().cloned().collect::<Vec<_>>();
                typical.sort();
                signals.push(UnusualSignal {
                    kind: SignalKind::NewCountry,
                    title: "Unusual for this process".into(),
                    reason: format!(
                        "First observed connection from {} to {}.",
                        observation.process_key,
                        geo.country_label()
                            .unwrap_or_else(|| "an unknown country".into())
                    ),
                    typical: (!typical.is_empty())
                        .then(|| format!("Previously observed: {}", typical.join(", "))),
                });
            }
            if let Some(country_code) = country_code {
                baseline.countries.insert(country_code.to_owned());
            }
        }

        let remote_port = observation.connection.remote.port;
        if baseline.samples >= 8 && !baseline.ports.contains(&remote_port) {
            signals.push(UnusualSignal {
                kind: SignalKind::UnusualPort,
                title: "Unusual port".into(),
                reason: format!(
                    "{} has not previously used remote port {}.",
                    observation.process_key, remote_port
                ),
                typical: None,
            });
        }

        while baseline
            .recent_connections_ms
            .front()
            .is_some_and(|time| observation.timestamp_ms.saturating_sub(*time) > 4_000)
        {
            baseline.recent_connections_ms.pop_front();
        }
        baseline
            .recent_connections_ms
            .push_back(observation.timestamp_ms);
        if baseline.recent_connections_ms.len() == 25 {
            signals.push(UnusualSignal {
                kind: SignalKind::ConnectionBurst,
                title: "Unusual activity".into(),
                reason: format!(
                    "{} created 25 connections within four seconds.",
                    observation.process_key
                ),
                typical: Some("Typical observed range is below 25 in four seconds.".into()),
            });
        }

        baseline
            .destinations
            .insert(observation.destination_key.to_owned());
        baseline.ports.insert(remote_port);
        baseline.samples = baseline.samples.saturating_add(1);
        signals
    }

    pub fn observe_activity(
        &mut self,
        process_key: &str,
        bytes: u64,
        elapsed_ms: i64,
        duration_ms: u64,
    ) -> Vec<UnusualSignal> {
        let baseline = self.processes.entry(process_key.to_owned()).or_default();
        let elapsed_seconds = elapsed_ms.max(1) as f64 / 1_000.0;
        let rate = bytes as f64 / elapsed_seconds;
        let mut signals = Vec::new();
        if baseline.transfer_samples >= 8
            && baseline.mean_bytes_per_second > 0.0
            && rate >= 1_048_576.0
            && rate > baseline.mean_bytes_per_second * 6.0
        {
            signals.push(UnusualSignal {
                kind: SignalKind::HighBandwidth,
                title: "Higher bandwidth than usual".into(),
                reason: format!(
                    "{} transferred approximately {:.1} MiB/s in this interval.",
                    process_key,
                    rate / 1_048_576.0
                ),
                typical: Some(format!(
                    "Retained local mean: {:.1} KiB/s.",
                    baseline.mean_bytes_per_second / 1_024.0
                )),
            });
        }
        if bytes > 0 {
            let samples = baseline.transfer_samples as f64;
            baseline.mean_bytes_per_second =
                (baseline.mean_bytes_per_second * samples + rate) / (samples + 1.0);
            baseline.transfer_samples = baseline.transfer_samples.saturating_add(1);
        }
        if duration_ms >= 30 * 60 * 1_000 {
            signals.push(UnusualSignal {
                kind: SignalKind::PersistentConnection,
                title: "Persistent connection".into(),
                reason: format!("{} has kept this connection open for at least 30 minutes.", process_key),
                typical: Some("Long-lived connections can be normal for messaging, media and sync applications.".into()),
            });
        }
        signals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Endpoint, NetworkProtocol};

    fn key(port: u16) -> ConnectionKey {
        ConnectionKey {
            pid: Some(4),
            process_instance: Some("pid:4:start:1".into()),
            local: Endpoint::new("127.0.0.1".parse().expect("IP"), 40_000),
            remote: Endpoint::new("1.1.1.1".parse().expect("IP"), port),
            protocol: NetworkProtocol::Tcp,
            interface_id: Some("test".into()),
            capture_generation: 1,
            flow_generation: u64::from(port),
        }
    }

    #[test]
    fn new_destination_is_neutral_and_explainable() {
        let mut book = BaselineBook::default();
        let first = key(443);
        assert!(
            book.observe(BaselineObservation {
                process_key: "Firefox",
                destination_key: "one.example",
                connection: &first,
                geo: None,
                timestamp_ms: 1,
            })
            .is_empty()
        );

        let second = key(443);
        let signals = book.observe(BaselineObservation {
            process_key: "Firefox",
            destination_key: "two.example",
            connection: &second,
            geo: None,
            timestamp_ms: 2,
        });
        assert_eq!(signals[0].kind, SignalKind::NewDestination);
        assert!(signals[0].reason.contains("First observed"));
    }

    #[test]
    fn high_transfer_signal_is_explainable_and_statistical() {
        let mut book = BaselineBook::default();
        for _ in 0..8 {
            assert!(
                book.observe_activity("Browser", 10_000, 1_000, 1_000)
                    .is_empty()
            );
        }
        let signals = book.observe_activity("Browser", 8 * 1_048_576, 1_000, 2_000);
        assert!(
            signals
                .iter()
                .any(|signal| signal.kind == SignalKind::HighBandwidth)
        );
        assert!(signals.iter().all(|signal| !signal.reason.is_empty()));
    }
}
