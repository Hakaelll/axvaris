use crate::NetworkProtocol;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SERVICES: &str = include_str!("../assets/iana-services-2026-08.csv");
const SUFFIXES: &str = include_str!("../assets/public-suffix-2026-08.dat");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineCatalogIdentity {
    pub name: String,
    pub snapshot_date: String,
    pub source: String,
    pub license: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineServiceHint {
    pub observed_port: u16,
    pub observed_protocol: NetworkProtocol,
    pub service_name: String,
    pub description: String,
    pub wording: String,
    pub catalog: OfflineCatalogIdentity,
}

pub fn service_hint(protocol: NetworkProtocol, port: u16) -> Option<OfflineServiceHint> {
    let protocol_name = protocol.as_str();
    SERVICES
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.splitn(4, ',');
            Some((
                fields.next()?,
                fields.next()?.parse::<u16>().ok()?,
                fields.next()?,
                fields.next()?,
            ))
        })
        .find(|(_, candidate_port, candidate_protocol, _)| {
            *candidate_port == port && *candidate_protocol == protocol_name
        })
        .map(|(name, _, _, description)| OfflineServiceHint {
            observed_port: port,
            observed_protocol: protocol,
            service_name: name.into(),
            description: description.into(),
            wording: format!(
                "Habitual service for {protocol_name}/{port}; the observed port and protocol remain the primary evidence."
            ),
            catalog: services_identity(),
        })
}

pub fn registrable_domain(name: &str) -> Option<String> {
    let normalized = name.trim().trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() || normalized.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    let labels = normalized.split('.').collect::<Vec<_>>();
    if labels.len() < 2
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return None;
    }
    let suffixes = SUFFIXES
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect::<Vec<_>>();
    let suffix_labels = (1..labels.len())
        .rev()
        .find(|count| suffixes.contains(&labels[labels.len() - count..].join(".").as_str()))
        .unwrap_or(1);
    let start = labels.len().checked_sub(suffix_labels + 1)?;
    Some(labels[start..].join("."))
}

pub fn offline_catalog_identities() -> Vec<OfflineCatalogIdentity> {
    vec![services_identity(), suffixes_identity()]
}

fn services_identity() -> OfflineCatalogIdentity {
    identity(
        "IANA service names (release subset)",
        "https://www.iana.org/assignments/service-names-port-numbers/",
        "IANA licensing terms",
        SERVICES,
    )
}

fn suffixes_identity() -> OfflineCatalogIdentity {
    identity(
        "Public Suffix List (release subset)",
        "https://publicsuffix.org/list/",
        "Mozilla Public License 2.0",
        SUFFIXES,
    )
}

fn identity(name: &str, source: &str, license: &str, contents: &str) -> OfflineCatalogIdentity {
    OfflineCatalogIdentity {
        name: name.into(),
        snapshot_date: "2026-08-13".into(),
        source: source.into(),
        license: license.into(),
        sha256: format!("{:x}", Sha256::digest(contents.as_bytes())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_never_replace_the_observed_port_or_protocol() {
        let hint = service_hint(NetworkProtocol::Udp, 443).expect("QUIC hint");
        assert_eq!(hint.observed_port, 443);
        assert_eq!(hint.observed_protocol, NetworkProtocol::Udp);
        assert!(hint.wording.contains("primary evidence"));
        assert_eq!(service_hint(NetworkProtocol::Tcp, 65_000), None);
    }

    #[test]
    fn suffix_snapshot_groups_domains_locally_and_rejects_ips() {
        assert_eq!(
            registrable_domain("cdn.images.example.co.uk."),
            Some("example.co.uk".into())
        );
        assert_eq!(registrable_domain("203.0.113.7"), None);
        assert!(
            offline_catalog_identities()
                .iter()
                .all(|catalog| catalog.sha256.len() == 64)
        );
    }
}
