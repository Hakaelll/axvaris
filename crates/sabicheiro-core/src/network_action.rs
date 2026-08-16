use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkActionIntent {
    GeoIpDatasetUpdate,
    ReverseDns,
}

impl NetworkActionIntent {
    pub const fn destination_class(self) -> NetworkDestinationClass {
        match self {
            Self::GeoIpDatasetUpdate => NetworkDestinationClass::DbIpDownload,
            Self::ReverseDns => NetworkDestinationClass::SystemDnsResolver,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkDestinationClass {
    DbIpDownload,
    SystemDnsResolver,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActionAudit {
    pub id: i64,
    pub occurred_at_ms: i64,
    pub intent: NetworkActionIntent,
    pub destination_class: NetworkDestinationClass,
    pub consent_id: Option<String>,
    pub allowed: bool,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActionPolicy {
    pub offline_enforced: bool,
    pub geoip_updates_allowed: bool,
    pub reverse_dns_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkActionGrant {
    intent: NetworkActionIntent,
    destination_class: NetworkDestinationClass,
    consent_id: String,
    maximum_response_bytes: u64,
}

impl NetworkActionGrant {
    pub const fn intent(&self) -> NetworkActionIntent {
        self.intent
    }

    pub const fn destination_class(&self) -> NetworkDestinationClass {
        self.destination_class
    }

    pub fn consent_id(&self) -> &str {
        &self.consent_id
    }

    pub const fn maximum_response_bytes(&self) -> u64 {
        self.maximum_response_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NetworkActionDenied {
    #[error("offline enforced mode rejects every optional network action")]
    OfflineEnforced,
    #[error("this optional network action is disabled")]
    Disabled,
    #[error("an explicit local consent identifier is required")]
    MissingConsent,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkActionBroker {
    policy: NetworkActionPolicy,
}

impl NetworkActionBroker {
    pub const fn new(policy: NetworkActionPolicy) -> Self {
        Self { policy }
    }

    pub fn authorize(
        &self,
        intent: NetworkActionIntent,
        consent_id: impl Into<String>,
    ) -> Result<NetworkActionGrant, NetworkActionDenied> {
        if self.policy.offline_enforced {
            return Err(NetworkActionDenied::OfflineEnforced);
        }
        let allowed = match intent {
            NetworkActionIntent::GeoIpDatasetUpdate => self.policy.geoip_updates_allowed,
            NetworkActionIntent::ReverseDns => self.policy.reverse_dns_allowed,
        };
        if !allowed {
            return Err(NetworkActionDenied::Disabled);
        }
        let consent_id = consent_id.into();
        if consent_id.trim().is_empty() || consent_id.len() > 128 {
            return Err(NetworkActionDenied::MissingConsent);
        }
        let maximum_response_bytes = match intent {
            NetworkActionIntent::GeoIpDatasetUpdate => 256 * 1024 * 1024,
            NetworkActionIntent::ReverseDns => 4 * 1024,
        };
        Ok(NetworkActionGrant {
            intent,
            destination_class: intent.destination_class(),
            consent_id,
            maximum_response_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_offline_policies_cannot_create_egress_grants() {
        let default = NetworkActionBroker::new(NetworkActionPolicy::default());
        assert_eq!(
            default.authorize(NetworkActionIntent::ReverseDns, "settings"),
            Err(NetworkActionDenied::Disabled)
        );
        let offline = NetworkActionBroker::new(NetworkActionPolicy {
            offline_enforced: true,
            geoip_updates_allowed: true,
            reverse_dns_allowed: true,
        });
        assert_eq!(
            offline.authorize(NetworkActionIntent::GeoIpDatasetUpdate, "button"),
            Err(NetworkActionDenied::OfflineEnforced)
        );
    }

    #[test]
    fn grants_bind_intent_to_a_fixed_destination_and_limit() {
        let broker = NetworkActionBroker::new(NetworkActionPolicy {
            geoip_updates_allowed: true,
            ..NetworkActionPolicy::default()
        });
        let grant = broker
            .authorize(
                NetworkActionIntent::GeoIpDatasetUpdate,
                "geoip-update-button",
            )
            .expect("grant");
        assert_eq!(
            grant.destination_class(),
            NetworkDestinationClass::DbIpDownload
        );
        assert_eq!(grant.maximum_response_bytes(), 256 * 1024 * 1024);
    }
}
