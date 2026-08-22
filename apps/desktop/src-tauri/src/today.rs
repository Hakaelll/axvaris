use sabicheiro_core::{
    ListenerExposure, ListenerScope, NetworkProfile, SessionComparison, SubsystemDiagnostic,
    SubsystemStatus, TodayChangeExample, TodayChangeGroup, TodayChangeKind, TodayEvidenceTarget,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub(crate) struct ListenerChanges {
    pub added: Vec<ListenerExposure>,
    pub removed: Vec<ListenerExposure>,
    pub widened: Vec<ListenerExposure>,
}

fn exact_listener_key(listener: &ListenerExposure) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        listener.protocol.as_str(),
        listener.endpoint.address,
        listener.endpoint.port,
        listener.process_instance.as_deref().unwrap_or("unknown")
    )
}

fn listener_service_key(listener: &ListenerExposure) -> String {
    format!(
        "{}\0{}\0{}",
        listener.protocol.as_str(),
        listener.endpoint.port,
        listener.process_instance.as_deref().unwrap_or("unknown")
    )
}

fn scope_rank(scope: ListenerScope) -> u8 {
    match scope {
        ListenerScope::Loopback => 0,
        ListenerScope::Vpn => 1,
        ListenerScope::Lan => 2,
        ListenerScope::Wildcard => 3,
        ListenerScope::Unknown => 0,
    }
}

pub(crate) fn compare_listeners(
    current: &[ListenerExposure],
    previous: &[ListenerExposure],
) -> ListenerChanges {
    let current_by_key = current
        .iter()
        .map(|listener| (exact_listener_key(listener), listener.clone()))
        .collect::<BTreeMap<_, _>>();
    let previous_by_key = previous
        .iter()
        .map(|listener| (exact_listener_key(listener), listener.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut added = current_by_key
        .iter()
        .filter(|(key, _)| !previous_by_key.contains_key(*key))
        .map(|(_, listener)| listener.clone())
        .collect::<Vec<_>>();
    let mut removed = previous_by_key
        .iter()
        .filter(|(key, _)| !current_by_key.contains_key(*key))
        .map(|(_, listener)| listener.clone())
        .collect::<Vec<_>>();
    let mut widened = Vec::new();

    let mut added_index = 0;
    while added_index < added.len() {
        let candidate = &added[added_index];
        let previous_index = removed.iter().position(|earlier| {
            listener_service_key(earlier) == listener_service_key(candidate)
                && scope_rank(candidate.scope) > scope_rank(earlier.scope)
        });
        if let Some(previous_index) = previous_index {
            widened.push(added.remove(added_index));
            removed.remove(previous_index);
        } else {
            added_index += 1;
        }
    }

    ListenerChanges {
        added,
        removed,
        widened,
    }
}

fn endpoint_label(listener: &ListenerExposure) -> String {
    listener.endpoint.label()
}

fn listener_group(
    kind: TodayChangeKind,
    category: &str,
    listeners: &[ListenerExposure],
    example_limit: usize,
) -> TodayChangeGroup {
    TodayChangeGroup {
        kind,
        count: listeners.len(),
        examples: listeners
            .iter()
            .take(example_limit)
            .map(|listener| TodayChangeExample {
                id: format!("{category}:{}", exact_listener_key(listener)),
                kind,
                primary: endpoint_label(listener),
                secondary: listener.process_instance.clone(),
                connection_id: None,
                query: Some("state:listening".into()),
                target: TodayEvidenceTarget::Exposure,
                observed_at_ms: listener.last_observed_ms,
            })
            .collect(),
    }
}

pub(crate) fn listener_groups(
    changes: &ListenerChanges,
    example_limit: usize,
) -> Vec<TodayChangeGroup> {
    vec![
        listener_group(
            TodayChangeKind::ListenerAdded,
            "listener-added",
            &changes.added,
            example_limit,
        ),
        listener_group(
            TodayChangeKind::ListenerRemoved,
            "listener-removed",
            &changes.removed,
            example_limit,
        ),
        listener_group(
            TodayChangeKind::ListenerWidened,
            "listener-widened",
            &changes.widened,
            example_limit,
        ),
    ]
}

pub(crate) fn profile_group(
    active_id: Option<&str>,
    previous_id: Option<&str>,
    profiles: &[NetworkProfile],
    changed: bool,
    observed_at_ms: i64,
) -> TodayChangeGroup {
    let display = |id: &str| {
        profiles
            .iter()
            .find(|profile| profile.id == id)
            .and_then(|profile| profile.display_name.clone())
            .unwrap_or_else(|| id.to_owned())
    };
    let examples = active_id
        .filter(|_| changed)
        .map(|active| TodayChangeExample {
            id: format!("network-profile:{active}:{observed_at_ms}"),
            kind: TodayChangeKind::NetworkProfile,
            primary: display(active),
            secondary: previous_id.map(display),
            connection_id: None,
            query: None,
            target: TodayEvidenceTarget::Settings,
            observed_at_ms,
        })
        .into_iter()
        .collect::<Vec<_>>();
    TodayChangeGroup {
        kind: TodayChangeKind::NetworkProfile,
        count: examples.len(),
        examples,
    }
}

pub(crate) fn provider_group(
    subsystems: &[SubsystemDiagnostic],
    example_limit: usize,
    generated_at_ms: i64,
) -> TodayChangeGroup {
    let degraded = subsystems
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                SubsystemStatus::Degraded | SubsystemStatus::Unavailable
            )
        })
        .collect::<Vec<_>>();
    TodayChangeGroup {
        kind: TodayChangeKind::ProviderDegradation,
        count: degraded.len(),
        examples: degraded
            .into_iter()
            .take(example_limit)
            .map(|item| TodayChangeExample {
                id: format!(
                    "provider:{}:{}",
                    item.subsystem,
                    item.since_ms.unwrap_or(generated_at_ms)
                ),
                kind: TodayChangeKind::ProviderDegradation,
                primary: item.subsystem.clone(),
                secondary: Some(item.message.clone()),
                connection_id: None,
                query: None,
                target: TodayEvidenceTarget::Health,
                observed_at_ms: item.since_ms.unwrap_or(generated_at_ms),
            })
            .collect(),
    }
}

pub(crate) fn bound_comparison(
    mut comparison: SessionComparison,
    row_limit: usize,
) -> SessionComparison {
    for rows in [
        &mut comparison.processes,
        &mut comparison.destinations,
        &mut comparison.countries,
        &mut comparison.ports,
    ] {
        rows.retain(|row| row.delta != 0);
        rows.truncate(row_limit);
    }
    comparison
}

pub(crate) fn traffic_sources_are_comparable(
    previous: &[String],
    current: impl IntoIterator<Item = String>,
) -> bool {
    let previous = previous.iter().cloned().collect::<BTreeSet<_>>();
    let current = current.into_iter().collect::<BTreeSet<_>>();
    !previous.is_empty()
        && previous == current
        && !previous.contains("unavailable")
        && previous.len() == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use sabicheiro_core::{Endpoint, FirewallEvidence, NetworkProtocol};

    fn listener(address: &str, scope: ListenerScope) -> ListenerExposure {
        ListenerExposure {
            endpoint: Endpoint::parse(address, 8080).expect("listener endpoint"),
            protocol: NetworkProtocol::Tcp,
            process_instance: Some("pid:7:start:1".into()),
            scope,
            firewall: FirewallEvidence::NotQueryable,
            first_observed_ms: 1,
            last_observed_ms: 2,
            limitation: "fixture".into(),
        }
    }

    #[test]
    fn listener_comparison_distinguishes_widened_scope() {
        let earlier = listener("127.0.0.1", ListenerScope::Loopback);
        let wider = listener("0.0.0.0", ListenerScope::Wildcard);
        let changes = compare_listeners(std::slice::from_ref(&wider), &[earlier]);
        assert!(changes.added.is_empty());
        assert!(changes.removed.is_empty());
        assert_eq!(changes.widened, vec![wider]);
    }

    #[test]
    fn traffic_requires_one_identical_available_counter_source() {
        assert!(traffic_sources_are_comparable(
            &["etw_flow".into()],
            ["etw_flow".into()]
        ));
        assert!(!traffic_sources_are_comparable(
            &["unavailable".into()],
            ["unavailable".into()]
        ));
    }
}
