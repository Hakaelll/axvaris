use crate::{Connection, ListenerScope, NetworkPathKind, NetworkProtocol};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, net::IpAddr, str::FromStr};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum LocalRuleCondition {
    Process(String),
    ExecutableRevision(String),
    Domain(String),
    Ip(IpAddr),
    Cidr(String),
    Asn(u32),
    Country(String),
    Protocol(NetworkProtocol),
    Port(u16),
    Listener(ListenerScope),
    NetworkProfile(String),
    Path(NetworkPathKind),
    FirstSeen(bool),
    MinimumBytes(u64),
    MinimumDurationMs(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum LocalRuleAction {
    Tag(String),
    Pin,
    AddToInvestigation(String),
    Notify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalRule {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub enabled: bool,
    pub match_all: bool,
    pub conditions: Vec<LocalRuleCondition>,
    pub actions: BTreeSet<LocalRuleAction>,
    pub cooldown_seconds: u32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleContext {
    pub network_profile_id: Option<String>,
    pub path_kind: NetworkPathKind,
    pub listener_scope: ListenerScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub rule_version: u32,
    pub connection_id: String,
    pub matched: bool,
    pub reasons: Vec<String>,
    pub actions: BTreeSet<LocalRuleAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionAnnotation {
    pub connection_id: String,
    pub tags: BTreeSet<String>,
    pub pinned: bool,
    pub investigation_ids: BTreeSet<String>,
    pub updated_at_ms: i64,
}

impl LocalRule {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty()
            || self.id.len() > 128
            || !self
                .id
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_'))
        {
            return Err("rule id must use 1-128 ASCII letters, numbers, '-' or '_'".into());
        }
        if self.version == 0 || self.name.trim().is_empty() || self.name.len() > 120 {
            return Err("rule version and name are invalid".into());
        }
        if self.conditions.is_empty() || self.conditions.len() > 64 || self.actions.is_empty() {
            return Err("rules need 1-64 conditions and at least one safe action".into());
        }
        if self.cooldown_seconds > 7 * 24 * 60 * 60 {
            return Err("rule cooldown exceeds seven days".into());
        }
        for condition in &self.conditions {
            match condition {
                LocalRuleCondition::Process(value)
                | LocalRuleCondition::ExecutableRevision(value)
                | LocalRuleCondition::Domain(value)
                | LocalRuleCondition::NetworkProfile(value)
                    if value.trim().is_empty() || value.len() > 512 =>
                {
                    return Err("rule text conditions need 1-512 characters".into());
                }
                LocalRuleCondition::Cidr(value) => {
                    parse_cidr(value)?;
                }
                LocalRuleCondition::Country(value)
                    if value.len() != 2
                        || !value
                            .chars()
                            .all(|character| character.is_ascii_alphabetic()) =>
                {
                    return Err("country conditions require a two-letter code".into());
                }
                _ => {}
            }
        }
        for action in &self.actions {
            match action {
                LocalRuleAction::Tag(value) | LocalRuleAction::AddToInvestigation(value)
                    if value.trim().is_empty() || value.len() > 128 =>
                {
                    return Err("rule action values need 1-128 characters".into());
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn evaluate(&self, connection: &Connection, context: &RuleContext) -> RuleMatch {
        let evaluations = self
            .conditions
            .iter()
            .map(|condition| evaluate_condition(condition, connection, context))
            .collect::<Vec<_>>();
        let matched = self.enabled
            && if self.match_all {
                evaluations.iter().all(|(matched, _)| *matched)
            } else {
                evaluations.iter().any(|(matched, _)| *matched)
            };
        RuleMatch {
            rule_id: self.id.clone(),
            rule_version: self.version,
            connection_id: connection.id.clone(),
            matched,
            reasons: evaluations
                .into_iter()
                .filter(|(condition_matched, _)| *condition_matched)
                .map(|(_, reason)| reason)
                .collect(),
            actions: if matched {
                self.actions.clone()
            } else {
                BTreeSet::new()
            },
        }
    }
}

pub fn detect_rule_conflicts(rules: &[LocalRule]) -> Vec<(String, String)> {
    let mut conflicts = Vec::new();
    for (index, left) in rules.iter().enumerate() {
        for right in &rules[index + 1..] {
            if left.conditions == right.conditions && left.match_all == right.match_all {
                conflicts.push((left.id.clone(), right.id.clone()));
            }
        }
    }
    conflicts
}

fn evaluate_condition(
    condition: &LocalRuleCondition,
    connection: &Connection,
    context: &RuleContext,
) -> (bool, String) {
    let matched = match condition {
        LocalRuleCondition::Process(value) => connection.process.name.eq_ignore_ascii_case(value),
        LocalRuleCondition::ExecutableRevision(value) => connection
            .process
            .executable_revision_id
            .as_deref()
            .is_some_and(|revision| revision == value),
        LocalRuleCondition::Domain(value) => connection
            .domain
            .as_deref()
            .or(connection.hostname.as_deref())
            .is_some_and(|domain| domain.eq_ignore_ascii_case(value)),
        LocalRuleCondition::Ip(value) => connection.key.remote.address == *value,
        LocalRuleCondition::Cidr(value) => parse_cidr(value).is_ok_and(|(network, prefix)| {
            address_in_prefix(connection.key.remote.address, network, prefix)
        }),
        LocalRuleCondition::Asn(value) => connection
            .asn
            .as_ref()
            .is_some_and(|asn| asn.number == *value),
        LocalRuleCondition::Country(value) => connection
            .geo
            .as_ref()
            .and_then(|geo| geo.country_code.as_deref())
            .is_some_and(|country| country.eq_ignore_ascii_case(value)),
        LocalRuleCondition::Protocol(value) => connection.protocol == *value,
        LocalRuleCondition::Port(value) => connection.key.remote.port == *value,
        LocalRuleCondition::Listener(value) => context.listener_scope == *value,
        LocalRuleCondition::NetworkProfile(value) => {
            context.network_profile_id.as_deref() == Some(value)
        }
        LocalRuleCondition::Path(value) => context.path_kind == *value,
        LocalRuleCondition::FirstSeen(value) => connection.first_seen_destination == *value,
        LocalRuleCondition::MinimumBytes(value) => {
            connection
                .traffic
                .bytes_sent
                .saturating_add(connection.traffic.bytes_received)
                >= *value
        }
        LocalRuleCondition::MinimumDurationMs(value) => connection.duration_ms >= *value,
    };
    (matched, format!("{condition:?}"))
}

fn parse_cidr(value: &str) -> Result<(IpAddr, u8), String> {
    let (network, prefix) = value
        .split_once('/')
        .ok_or_else(|| "CIDR needs an address and prefix".to_owned())?;
    let network = IpAddr::from_str(network).map_err(|_| "CIDR address is invalid".to_owned())?;
    let prefix = prefix
        .parse::<u8>()
        .map_err(|_| "CIDR prefix is invalid".to_owned())?;
    let maximum = if network.is_ipv4() { 32 } else { 128 };
    if prefix > maximum {
        return Err("CIDR prefix exceeds its address family".into());
    }
    Ok((network, prefix))
}

fn address_in_prefix(address: IpAddr, network: IpAddr, prefix: u8) -> bool {
    match (address, network) {
        (IpAddr::V4(address), IpAddr::V4(network)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            u32::from(address) & mask == u32::from(network) & mask
        }
        (IpAddr::V6(address), IpAddr::V6(network)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            u128::from(address) & mask == u128::from(network) & mask
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> LocalRule {
        LocalRule {
            id: "fixture".into(),
            version: 1,
            name: "Fixture".into(),
            enabled: true,
            match_all: true,
            conditions: vec![
                LocalRuleCondition::Cidr("203.0.113.0/24".into()),
                LocalRuleCondition::Port(443),
                LocalRuleCondition::FirstSeen(true),
            ],
            actions: BTreeSet::from([
                LocalRuleAction::Tag("review".into()),
                LocalRuleAction::Notify,
            ]),
            cooldown_seconds: 60,
            created_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    #[test]
    fn dry_run_and_execution_share_the_exact_evaluator() {
        let mut connection = Connection::placeholder(1);
        connection.key.remote.address = "203.0.113.8".parse().expect("IP");
        connection.key.remote.port = 443;
        connection.first_seen_destination = true;
        let dry_run = rule().evaluate(&connection, &RuleContext::default());
        let execution = rule().evaluate(&connection, &RuleContext::default());
        assert_eq!(dry_run, execution);
        assert!(execution.matched);
        assert!(!execution.actions.is_empty());
    }

    #[test]
    fn duplicate_conditions_are_reported_as_conflicts() {
        let left = rule();
        let mut right = rule();
        right.id = "other".into();
        assert_eq!(
            detect_rule_conflicts(&[left, right]),
            vec![("fixture".into(), "other".into())]
        );
    }
}
