use crate::{
    ListenerExposure, LocalNotificationRecord, LocalRule, NetworkProfile, SessionComparison,
    SessionSummary, TodayCardState, TrafficCounters,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodayChangeKind {
    Process,
    ApplicationDestination,
    Domain,
    Asn,
    Country,
    ExecutableRevision,
    NetworkProfile,
    ListenerAdded,
    ListenerRemoved,
    ListenerWidened,
    ProviderDegradation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodayEvidenceTarget {
    Live,
    Investigate,
    Exposure,
    Health,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodayChangeExample {
    pub id: String,
    pub kind: TodayChangeKind,
    pub primary: String,
    pub secondary: Option<String>,
    pub connection_id: Option<String>,
    pub query: Option<String>,
    pub target: TodayEvidenceTarget,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodayChangeGroup {
    pub kind: TodayChangeKind,
    pub count: usize,
    pub examples: Vec<TodayChangeExample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodaySessionComparison {
    pub previous_session: SessionSummary,
    pub current_session_id: String,
    pub comparison: SessionComparison,
    pub previous_traffic: TrafficCounters,
    pub current_traffic: TrafficCounters,
    pub traffic_comparable: bool,
    pub traffic_limitation: Option<String>,
    pub listener_added: Vec<ListenerExposure>,
    pub listener_removed: Vec<ListenerExposure>,
    pub listener_widened: Vec<ListenerExposure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodayPayloadLimits {
    pub examples_per_category: usize,
    pub comparison_rows_per_category: usize,
    pub card_states: usize,
    pub profiles: usize,
    pub rules: usize,
    pub notifications: usize,
}

impl Default for TodayPayloadLimits {
    fn default() -> Self {
        Self {
            examples_per_category: 3,
            comparison_rows_per_category: 8,
            card_states: 500,
            profiles: 32,
            rules: 200,
            notifications: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodayBootstrap {
    pub generated_at_ms: i64,
    pub history_scope: String,
    pub limits: TodayPayloadLimits,
    pub changes: Vec<TodayChangeGroup>,
    pub card_states: Vec<TodayCardState>,
    pub profiles: Vec<NetworkProfile>,
    pub rules: Vec<LocalRule>,
    pub notifications: Vec<LocalNotificationRecord>,
    pub active_profile_id: Option<String>,
    pub previous_session: Option<TodaySessionComparison>,
}
