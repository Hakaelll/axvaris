use crate::RuleMatch;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationState {
    #[default]
    Unread,
    Read,
    Pinned,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalNotificationRecord {
    pub id: String,
    pub rule_id: String,
    pub rule_version: u32,
    pub evidence_ids: Vec<String>,
    pub network_profile_id: Option<String>,
    pub title: String,
    pub detail: String,
    pub group_key: String,
    pub count: u32,
    pub first_at_ms: i64,
    pub last_at_ms: i64,
    pub snoozed_until_ms: Option<i64>,
    pub state: NotificationState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub start_minute_local: u16,
    pub end_minute_local: u16,
}

impl QuietHours {
    pub fn contains(&self, minute_local: u16) -> bool {
        if !self.enabled || self.start_minute_local > 1_439 || self.end_minute_local > 1_439 {
            return false;
        }
        if self.start_minute_local <= self.end_minute_local {
            (self.start_minute_local..self.end_minute_local).contains(&minute_local)
        } else {
            minute_local >= self.start_minute_local || minute_local < self.end_minute_local
        }
    }
}

pub struct NotificationCenterEngine {
    notifications: VecDeque<LocalNotificationRecord>,
    last_emitted_by_group: BTreeMap<(String, Option<String>, String), i64>,
    capacity: usize,
}

impl NotificationCenterEngine {
    pub fn new(capacity: usize) -> Self {
        Self {
            notifications: VecDeque::with_capacity(capacity.clamp(1, 100_000)),
            last_emitted_by_group: BTreeMap::new(),
            capacity: capacity.clamp(1, 100_000),
        }
    }

    pub fn from_records(capacity: usize, mut notifications: Vec<LocalNotificationRecord>) -> Self {
        let capacity = capacity.clamp(1, 100_000);
        notifications.sort_by(|left, right| {
            right
                .last_at_ms
                .cmp(&left.last_at_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
        notifications.truncate(capacity);
        let last_emitted_by_group = notifications
            .iter()
            .flat_map(|notification| {
                notification.evidence_ids.iter().map(|connection_id| {
                    (
                        (
                            notification.rule_id.clone(),
                            notification.network_profile_id.clone(),
                            connection_id.clone(),
                        ),
                        notification.last_at_ms,
                    )
                })
            })
            .collect();
        Self {
            notifications: notifications.into(),
            last_emitted_by_group,
            capacity,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ingest(
        &mut self,
        rule_match: &RuleMatch,
        network_profile_id: Option<String>,
        title: impl Into<String>,
        detail: impl Into<String>,
        occurred_at_ms: i64,
        cooldown_seconds: u32,
        quiet_hours: QuietHours,
        minute_local: u16,
    ) -> Option<LocalNotificationRecord> {
        if !rule_match.matched || quiet_hours.contains(minute_local) {
            return None;
        }
        let group_key = format!(
            "{}:{}:{}",
            rule_match.rule_id,
            network_profile_id.as_deref().unwrap_or("unknown-profile"),
            rule_match.connection_id
        );
        let group_identity = (
            rule_match.rule_id.clone(),
            network_profile_id.clone(),
            rule_match.connection_id.clone(),
        );
        if let Some(existing) = self.notifications.iter_mut().find(|notification| {
            notification.rule_id == rule_match.rule_id
                && notification.network_profile_id == network_profile_id
                && notification
                    .evidence_ids
                    .contains(&rule_match.connection_id)
                && notification.state != NotificationState::Resolved
        }) {
            existing.count = existing.count.saturating_add(1);
            existing.last_at_ms = occurred_at_ms;
            existing.detail = detail.into();
            return Some(existing.clone());
        }
        let cooldown_ms = i64::from(cooldown_seconds).saturating_mul(1_000);
        if self
            .last_emitted_by_group
            .get(&group_identity)
            .is_some_and(|last| occurred_at_ms.saturating_sub(*last) < cooldown_ms)
        {
            return None;
        }
        self.last_emitted_by_group
            .insert(group_identity, occurred_at_ms);
        let mut identity = Sha256::new();
        for value in [
            rule_match.rule_id.as_str(),
            rule_match.connection_id.as_str(),
            network_profile_id.as_deref().unwrap_or_default(),
        ] {
            identity.update((value.len() as u64).to_le_bytes());
            identity.update(value.as_bytes());
        }
        identity.update([u8::from(network_profile_id.is_some())]);
        identity.update(rule_match.rule_version.to_le_bytes());
        identity.update(occurred_at_ms.to_le_bytes());
        let notification = LocalNotificationRecord {
            id: format!("notification:{:x}", identity.finalize()),
            rule_id: rule_match.rule_id.clone(),
            rule_version: rule_match.rule_version,
            evidence_ids: vec![rule_match.connection_id.clone()],
            network_profile_id,
            title: title.into(),
            detail: detail.into(),
            group_key,
            count: 1,
            first_at_ms: occurred_at_ms,
            last_at_ms: occurred_at_ms,
            snoozed_until_ms: None,
            state: NotificationState::Unread,
        };
        self.notifications.push_front(notification.clone());
        while self.notifications.len() > self.capacity {
            if let Some(index) = self
                .notifications
                .iter()
                .rposition(|item| item.state != NotificationState::Pinned)
            {
                self.notifications.remove(index);
            } else {
                break;
            }
        }
        Some(notification)
    }

    pub fn items(&self) -> impl Iterator<Item = &LocalNotificationRecord> {
        self.notifications.iter()
    }

    pub fn set_state(&mut self, id: &str, state: NotificationState) -> bool {
        let Some(notification) = self
            .notifications
            .iter_mut()
            .find(|notification| notification.id == id)
        else {
            return false;
        };
        notification.state = state;
        true
    }

    pub fn snooze(&mut self, id: &str, until_ms: i64) -> bool {
        let Some(notification) = self
            .notifications
            .iter_mut()
            .find(|notification| notification.id == id)
        else {
            return false;
        };
        notification.snoozed_until_ms = Some(until_ms.max(notification.last_at_ms));
        true
    }
}

impl Default for NotificationCenterEngine {
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalRuleAction;
    use std::collections::BTreeSet;

    fn matched(connection: &str) -> RuleMatch {
        RuleMatch {
            rule_id: "rule".into(),
            rule_version: 1,
            connection_id: connection.into(),
            matched: true,
            reasons: vec!["fixture".into()],
            actions: BTreeSet::from([LocalRuleAction::Notify]),
        }
    }

    #[test]
    fn storms_group_and_capacity_remains_bounded() {
        let mut center = NotificationCenterEngine::new(2);
        for index in 0..10_000 {
            center.ingest(
                &matched("same"),
                Some("home".into()),
                "Review",
                format!("event {index}"),
                index,
                0,
                QuietHours::default(),
                600,
            );
        }
        let items = center.items().collect::<Vec<_>>();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].count, 10_000);
    }

    #[test]
    fn quiet_hours_and_cooldown_suppress_without_external_push() {
        let mut center = NotificationCenterEngine::default();
        let quiet = QuietHours {
            enabled: true,
            start_minute_local: 1_320,
            end_minute_local: 420,
        };
        assert!(
            center
                .ingest(&matched("a"), None, "x", "x", 1, 60, quiet, 60)
                .is_none()
        );
    }

    #[test]
    fn simultaneous_connections_and_profiles_have_distinct_ids() {
        let mut center = NotificationCenterEngine::default();
        let mut ids = BTreeSet::new();
        for (connection, profile) in [
            ("first", Some("home")),
            ("second", Some("home")),
            ("first", Some("work")),
            ("first", None),
            ("first", Some("unknown-profile")),
        ] {
            let notification = center
                .ingest(
                    &matched(connection),
                    profile.map(str::to_owned),
                    "Review",
                    "Evidence",
                    10_000,
                    60,
                    QuietHours::default(),
                    600,
                )
                .unwrap();
            ids.insert(notification.id);
        }
        assert_eq!(ids.len(), 5);
        assert_eq!(center.items().count(), 5);
    }

    #[test]
    fn snoozed_group_keeps_evidence_and_identity_until_and_after_expiry() {
        let mut center = NotificationCenterEngine::default();
        let mut first = center
            .ingest(
                &matched("a"),
                None,
                "Review",
                "first",
                1_000,
                0,
                QuietHours::default(),
                600,
            )
            .unwrap();
        assert!(center.snooze(&first.id, 5_000));
        // Legacy stored IDs and grouping keys must survive an update unchanged.
        first.id = "notification:rule:1:1000".into();
        let mut records = center.items().cloned().collect::<Vec<_>>();
        records[0].id = first.id.clone();
        let mut center = NotificationCenterEngine::from_records(10, records);
        for (at, detail, count) in [
            (4_999, "during snooze", 2),
            (5_000, "at expiry", 3),
            (5_001, "after expiry", 4),
        ] {
            let updated = center
                .ingest(
                    &matched("a"),
                    None,
                    "Review",
                    detail,
                    at,
                    0,
                    QuietHours::default(),
                    600,
                )
                .expect("group updates remain available for persistence");
            assert_eq!(updated.id, first.id);
            assert_eq!(updated.group_key, first.group_key);
            assert_eq!(updated.count, count);
            assert_eq!(updated.detail, detail);
            assert_eq!(updated.last_at_ms, at);
            assert_eq!(updated.snoozed_until_ms, Some(5_000));
            assert_eq!(updated.evidence_ids, ["a"]);
            assert_eq!(center.items().count(), 1);
        }
    }

    #[test]
    fn quiet_hours_use_start_inclusive_end_exclusive_local_minutes() {
        let mut quiet = QuietHours {
            enabled: true,
            start_minute_local: 540,
            end_minute_local: 1_020,
        };
        for (minute, expected) in [(539, false), (540, true), (1_019, true), (1_020, false)] {
            assert_eq!(quiet.contains(minute), expected);
        }
        quiet.start_minute_local = 1_320;
        quiet.end_minute_local = 420;
        for (minute, expected) in [
            (1_319, false),
            (1_320, true),
            (1_439, true),
            (0, true),
            (419, true),
            (420, false),
        ] {
            assert_eq!(quiet.contains(minute), expected);
        }
        quiet.enabled = false;
        assert!(!quiet.contains(0));
        quiet.enabled = true;
        quiet.end_minute_local = quiet.start_minute_local;
        assert!(!quiet.contains(1_320));
        quiet.end_minute_local = 1_440;
        assert!(!quiet.contains(1_320));
    }

    #[test]
    fn resolved_group_keeps_existing_cooldown() {
        let mut center = NotificationCenterEngine::default();
        let first = center
            .ingest(
                &matched("a"),
                None,
                "x",
                "x",
                1_000,
                5,
                QuietHours::default(),
                600,
            )
            .unwrap();
        center.set_state(&first.id, NotificationState::Resolved);
        assert!(
            center
                .ingest(
                    &matched("a"),
                    None,
                    "x",
                    "x",
                    5_999,
                    5,
                    QuietHours::default(),
                    600
                )
                .is_none()
        );
        let next = center
            .ingest(
                &matched("a"),
                None,
                "x",
                "x",
                6_000,
                5,
                QuietHours::default(),
                600,
            )
            .unwrap();
        assert_ne!(next.id, first.id);
    }
}
