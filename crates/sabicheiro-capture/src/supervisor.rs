use sabicheiro_core::{ProviderHealth, ProviderLifecycleState};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorPolicy {
    pub initial_backoff_ms: i64,
    pub maximum_backoff_ms: i64,
    pub circuit_breaker_failures: u32,
    pub stable_reset_ms: i64,
}

impl Default for SupervisorPolicy {
    fn default() -> Self {
        Self {
            initial_backoff_ms: 250,
            maximum_backoff_ms: 30_000,
            circuit_breaker_failures: 5,
            stable_reset_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderAction {
    Start {
        provider_id: String,
        generation: u64,
    },
    Restart {
        provider_id: String,
        generation: u64,
    },
    ActivateFallback {
        provider_id: String,
        fallback_provider_id: String,
    },
    Reconcile {
        provider_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone)]
struct ManagedProvider {
    health: ProviderHealth,
    fallback_provider_id: Option<String>,
    consecutive_failures: u32,
    healthy_since_ms: Option<i64>,
}

pub struct ProviderSupervisor {
    policy: SupervisorPolicy,
    providers: BTreeMap<String, ManagedProvider>,
    actions: VecDeque<ProviderAction>,
}

impl ProviderSupervisor {
    pub fn new(policy: SupervisorPolicy) -> Self {
        Self {
            policy,
            providers: BTreeMap::new(),
            actions: VecDeque::new(),
        }
    }

    pub fn register(
        &mut self,
        provider_id: impl Into<String>,
        fallback_provider_id: Option<String>,
        now_ms: i64,
    ) {
        let provider_id = provider_id.into();
        let health = ProviderHealth {
            provider_id: provider_id.clone(),
            generation: 1,
            state: ProviderLifecycleState::Starting,
            since_ms: now_ms,
            reason: None,
            fallback_provider_id: fallback_provider_id.clone(),
            restart_attempt: 0,
            next_retry_ms: None,
        };
        self.providers.insert(
            provider_id.clone(),
            ManagedProvider {
                health,
                fallback_provider_id,
                consecutive_failures: 0,
                healthy_since_ms: None,
            },
        );
        self.actions.push_back(ProviderAction::Start {
            provider_id,
            generation: 1,
        });
    }

    pub fn mark_healthy(&mut self, provider_id: &str, now_ms: i64) {
        let Some(provider) = self.providers.get_mut(provider_id) else {
            return;
        };
        provider.health.state = ProviderLifecycleState::Healthy;
        provider.health.since_ms = now_ms;
        provider.health.reason = None;
        provider.health.next_retry_ms = None;
        provider.healthy_since_ms = Some(now_ms);
    }

    pub fn mark_degraded(&mut self, provider_id: &str, reason: impl Into<String>, now_ms: i64) {
        let Some(provider) = self.providers.get_mut(provider_id) else {
            return;
        };
        provider.health.state = ProviderLifecycleState::Degraded;
        provider.health.since_ms = now_ms;
        provider.health.reason = Some(reason.into());
    }

    pub fn mark_failure(&mut self, provider_id: &str, reason: impl Into<String>, now_ms: i64) {
        let Some(provider) = self.providers.get_mut(provider_id) else {
            return;
        };
        provider.consecutive_failures = provider.consecutive_failures.saturating_add(1);
        provider.healthy_since_ms = None;
        provider.health.restart_attempt = provider.consecutive_failures;
        provider.health.since_ms = now_ms;
        provider.health.reason = Some(reason.into());
        if provider.consecutive_failures >= self.policy.circuit_breaker_failures {
            if let Some(fallback_provider_id) = provider.fallback_provider_id.clone() {
                provider.health.state = ProviderLifecycleState::Fallback;
                provider.health.next_retry_ms =
                    Some(now_ms.saturating_add(self.policy.maximum_backoff_ms.max(1)));
                self.actions.push_back(ProviderAction::ActivateFallback {
                    provider_id: provider_id.into(),
                    fallback_provider_id,
                });
            } else {
                provider.health.state = ProviderLifecycleState::Failed;
                provider.health.next_retry_ms = None;
            }
            return;
        }
        provider.health.state = ProviderLifecycleState::Restarting;
        let exponent = provider.consecutive_failures.saturating_sub(1).min(20);
        let multiplier = 1_i64.checked_shl(exponent).unwrap_or(i64::MAX);
        let backoff = self
            .policy
            .initial_backoff_ms
            .max(1)
            .saturating_mul(multiplier)
            .min(self.policy.maximum_backoff_ms.max(1));
        provider.health.next_retry_ms = Some(now_ms.saturating_add(backoff));
    }

    pub fn environment_changed(&mut self, reason: impl Into<String>, now_ms: i64) {
        let reason = reason.into();
        for (provider_id, provider) in &mut self.providers {
            if matches!(
                provider.health.state,
                ProviderLifecycleState::Healthy
                    | ProviderLifecycleState::Degraded
                    | ProviderLifecycleState::Fallback
            ) {
                provider.health.state = ProviderLifecycleState::Degraded;
                provider.health.since_ms = now_ms;
                provider.health.reason = Some(reason.clone());
                self.actions.push_back(ProviderAction::Reconcile {
                    provider_id: provider_id.clone(),
                    reason: reason.clone(),
                });
            }
        }
    }

    pub fn tick(&mut self, now_ms: i64) {
        for (provider_id, provider) in &mut self.providers {
            if provider.health.state == ProviderLifecycleState::Healthy
                && provider.healthy_since_ms.is_some_and(|since| {
                    now_ms.saturating_sub(since) >= self.policy.stable_reset_ms
                })
            {
                provider.consecutive_failures = 0;
                provider.health.restart_attempt = 0;
            }
            if !matches!(
                provider.health.state,
                ProviderLifecycleState::Restarting | ProviderLifecycleState::Fallback
            ) || !provider
                .health
                .next_retry_ms
                .is_some_and(|retry| now_ms >= retry)
            {
                continue;
            }
            provider.health.generation = provider.health.generation.saturating_add(1);
            provider.health.state = ProviderLifecycleState::Starting;
            provider.health.since_ms = now_ms;
            provider.health.next_retry_ms = None;
            self.actions.push_back(ProviderAction::Restart {
                provider_id: provider_id.clone(),
                generation: provider.health.generation,
            });
        }
    }

    pub fn next_action(&mut self) -> Option<ProviderAction> {
        self.actions.pop_front()
    }

    pub fn health(&self, provider_id: &str) -> Option<&ProviderHealth> {
        self.providers
            .get(provider_id)
            .map(|provider| &provider.health)
    }

    pub fn health_snapshot(&self) -> Vec<ProviderHealth> {
        self.providers
            .values()
            .map(|provider| provider.health.clone())
            .collect()
    }
}

impl Default for ProviderSupervisor {
    fn default() -> Self {
        Self::new(SupervisorPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_backoff_restarts_and_then_activates_fallback() {
        let mut supervisor = ProviderSupervisor::new(SupervisorPolicy {
            initial_backoff_ms: 100,
            maximum_backoff_ms: 500,
            circuit_breaker_failures: 3,
            stable_reset_ms: 1_000,
        });
        supervisor.register("events", Some("snapshot".into()), 0);
        assert!(matches!(
            supervisor.next_action(),
            Some(ProviderAction::Start { .. })
        ));

        supervisor.mark_failure("events", "fault one", 10);
        assert_eq!(
            supervisor.health("events").unwrap().next_retry_ms,
            Some(110)
        );
        supervisor.tick(109);
        assert!(supervisor.next_action().is_none());
        supervisor.tick(110);
        assert!(matches!(
            supervisor.next_action(),
            Some(ProviderAction::Restart { generation: 2, .. })
        ));
        supervisor.mark_failure("events", "fault two", 120);
        assert_eq!(
            supervisor.health("events").unwrap().next_retry_ms,
            Some(320)
        );
        supervisor.tick(320);
        assert!(matches!(
            supervisor.next_action(),
            Some(ProviderAction::Restart { generation: 3, .. })
        ));
        supervisor.mark_failure("events", "fault three", 330);
        assert_eq!(
            supervisor.health("events").unwrap().state,
            ProviderLifecycleState::Fallback
        );
        assert!(matches!(
            supervisor.next_action(),
            Some(ProviderAction::ActivateFallback { .. })
        ));
    }

    #[test]
    fn environment_change_requests_reconciliation_without_fake_continuity() {
        let mut supervisor = ProviderSupervisor::default();
        supervisor.register("events", Some("snapshot".into()), 0);
        supervisor.next_action();
        supervisor.mark_healthy("events", 1);
        supervisor.environment_changed("resume after sleep", 5_000);
        assert_eq!(
            supervisor.health("events").unwrap().state,
            ProviderLifecycleState::Degraded
        );
        assert!(matches!(
            supervisor.next_action(),
            Some(ProviderAction::Reconcile { reason, .. }) if reason == "resume after sleep"
        ));
    }
}
