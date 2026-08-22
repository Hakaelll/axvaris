mod aggregation;
mod baseline;
mod catalog;
mod context;
mod demo;
mod evidence;
mod filter;
mod models;
mod network_action;
mod notifications;
mod rules;
mod today;

pub use aggregation::ConnectionAggregator;
pub use baseline::{BaselineBook, BaselineObservation};
pub use catalog::*;
pub use context::*;
pub use demo::DemoGenerator;
pub use evidence::*;
pub use filter::{ConnectionFilter, FilterError, FilterTerm};
pub use models::*;
pub use network_action::*;
pub use notifications::*;
pub use rules::*;
pub use today::*;

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn reconstruct_at(entries: &[HistoryEntry], at_ms: i64) -> Vec<Connection> {
    entries
        .iter()
        .filter(|entry| {
            entry.connection.first_seen_ms <= at_ms && entry.connection.last_seen_ms >= at_ms
        })
        .map(|entry| {
            let mut connection = entry.connection.clone();
            connection.active = connection.last_seen_ms >= at_ms;
            connection
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_reconstruction_uses_connection_interval() {
        let mut connection = Connection::placeholder(1_000);
        connection.first_seen_ms = 1_000;
        connection.last_seen_ms = 2_000;
        let entry = HistoryEntry {
            session_id: "session-a".into(),
            connection,
        };

        assert!(reconstruct_at(std::slice::from_ref(&entry), 999).is_empty());
        assert_eq!(reconstruct_at(&[entry], 1_500).len(), 1);
    }

    #[test]
    fn dashboard_snapshot_roundtrips_json() {
        let snapshot = DashboardSnapshot::empty(42);
        let json = serde_json::to_string(&snapshot).expect("serialize snapshot");
        let decoded: DashboardSnapshot = serde_json::from_str(&json).expect("deserialize snapshot");
        assert_eq!(decoded.generated_at_ms, 42);
        assert_eq!(decoded.capture.mode, CaptureMode::Off);
    }

    #[test]
    fn settings_from_older_versions_receive_new_local_defaults() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
              "retention":"seven_days",
              "reduce_motion":false,
              "auto_focus":true,
              "show_local_traffic":false,
              "selected_interface":null,
              "home_latitude":40.4168,
              "home_longitude":-3.7038,
              "ui_language":"en",
              "demo_seed":42
            }"#,
        )
        .expect("backward-compatible settings");
        assert_eq!(settings.feed_group_window_seconds, 30);
        assert!(!settings.reverse_dns_enabled);
        assert!(!settings.show_listening_sockets);
        assert_eq!(settings.demo_seed, 42);
    }
}
