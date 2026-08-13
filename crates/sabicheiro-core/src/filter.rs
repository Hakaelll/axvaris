use crate::{
    AddressScope, Connection, ConnectionState, Direction, NetworkProtocol, ObservationSource,
    canonicalize_address,
};
use std::{fmt, net::IpAddr, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterTerm {
    Text(String),
    Process(String),
    Country(String),
    Protocol(NetworkProtocol),
    Port(u16),
    Direction(Direction),
    Ip(IpAddr),
    Domain(String),
    State(ConnectionState),
    FirstSeen(bool),
    Asn(u32),
    Cidr(IpAddr, u8),
    Scope(AddressScope),
    Geo(String),
    Source(ObservationSource),
    Not(Box<FilterTerm>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionFilter {
    pub terms: Vec<FilterTerm>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FilterError {
    #[error("unterminated quoted value")]
    UnterminatedQuote,
    #[error("unknown filter field: {0}")]
    UnknownField(String),
    #[error("invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
}

impl FromStr for ConnectionFilter {
    type Err = FilterError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let tokens = tokenize(input)?;
        let mut terms = Vec::with_capacity(tokens.len());
        for token in tokens {
            let (negated, token) = token
                .strip_prefix(['-', '!'])
                .filter(|value| !value.is_empty())
                .map_or((false, token.as_str()), |value| (true, value));
            let Some((field, value)) = token.split_once(':') else {
                let term = FilterTerm::Text(token.to_lowercase());
                terms.push(if negated {
                    FilterTerm::Not(Box::new(term))
                } else {
                    term
                });
                continue;
            };
            if value.is_empty() {
                return Err(FilterError::InvalidValue {
                    field: field.into(),
                    value: value.into(),
                });
            }
            let lower = value.to_lowercase();
            let term = match field.to_lowercase().as_str() {
                "process" => FilterTerm::Process(lower),
                "country" => FilterTerm::Country(value.to_uppercase()),
                "protocol" => match lower.as_str() {
                    "tcp" => FilterTerm::Protocol(NetworkProtocol::Tcp),
                    "udp" => FilterTerm::Protocol(NetworkProtocol::Udp),
                    _ => return Err(invalid(field, value)),
                },
                "port" => FilterTerm::Port(value.parse().map_err(|_| invalid(field, value))?),
                "direction" => match lower.as_str() {
                    "in" | "inbound" => FilterTerm::Direction(Direction::Inbound),
                    "out" | "outbound" => FilterTerm::Direction(Direction::Outbound),
                    "unknown" => FilterTerm::Direction(Direction::Unknown),
                    _ => return Err(invalid(field, value)),
                },
                "ip" => FilterTerm::Ip(value.parse().map_err(|_| invalid(field, value))?),
                "domain" => FilterTerm::Domain(lower),
                "state" => {
                    FilterTerm::State(parse_state(&lower).ok_or_else(|| invalid(field, value))?)
                }
                "firstseen" => {
                    FilterTerm::FirstSeen(parse_bool(&lower).ok_or_else(|| invalid(field, value))?)
                }
                "asn" => FilterTerm::Asn(value.parse().map_err(|_| invalid(field, value))?),
                "cidr" => {
                    let (address, prefix) =
                        parse_cidr(value).ok_or_else(|| invalid(field, value))?;
                    FilterTerm::Cidr(address, prefix)
                }
                "scope" => {
                    FilterTerm::Scope(parse_scope(&lower).ok_or_else(|| invalid(field, value))?)
                }
                "geo" => FilterTerm::Geo(lower),
                "source" => {
                    FilterTerm::Source(parse_source(&lower).ok_or_else(|| invalid(field, value))?)
                }
                _ => return Err(FilterError::UnknownField(field.into())),
            };
            terms.push(if negated {
                FilterTerm::Not(Box::new(term))
            } else {
                term
            });
        }
        Ok(Self { terms })
    }
}

impl ConnectionFilter {
    pub fn matches(&self, connection: &Connection) -> bool {
        self.terms.iter().all(|term| matches_term(term, connection))
    }
}

fn matches_term(term: &FilterTerm, connection: &Connection) -> bool {
    match term {
        FilterTerm::Text(value) => searchable_text(connection).contains(value),
        FilterTerm::Process(value) => connection.process.name.to_lowercase().contains(value),
        FilterTerm::Country(value) => connection.geo.as_ref().is_some_and(|geo| {
            geo.country_code
                .as_deref()
                .is_some_and(|code| code.eq_ignore_ascii_case(value))
                || geo
                    .country_name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(&value.to_lowercase()))
        }),
        FilterTerm::Protocol(protocol) => connection.protocol == *protocol,
        FilterTerm::Port(port) => {
            connection.key.local.port == *port || connection.key.remote.port == *port
        }
        FilterTerm::Direction(direction) => connection.direction == *direction,
        FilterTerm::Ip(ip) => {
            connection.key.local.address == *ip || connection.key.remote.address == *ip
        }
        FilterTerm::Domain(value) => connection
            .domain
            .as_deref()
            .or(connection.hostname.as_deref())
            .is_some_and(|domain| domain.to_lowercase().contains(value)),
        FilterTerm::State(state) => connection.state == *state,
        FilterTerm::FirstSeen(value) => connection.first_seen_destination == *value,
        FilterTerm::Asn(number) => connection
            .asn
            .as_ref()
            .is_some_and(|asn| asn.number == *number),
        FilterTerm::Cidr(network, prefix) => {
            cidr_contains(*network, *prefix, connection.key.local.address)
                || cidr_contains(*network, *prefix, connection.key.remote.address)
        }
        FilterTerm::Scope(scope) => connection.key.remote.scope == *scope,
        FilterTerm::Geo(value) => connection.geo.as_ref().is_some_and(|geo| {
            geo.country_code
                .iter()
                .chain(geo.country_name.iter())
                .chain(geo.region.iter())
                .chain(geo.city.iter())
                .any(|field| field.to_lowercase().contains(value))
        }),
        FilterTerm::Source(source) => connection.evidence.observation_source == *source,
        FilterTerm::Not(inner) => !matches_term(inner, connection),
    }
}

fn parse_cidr(value: &str) -> Option<(IpAddr, u8)> {
    let (address, prefix) = value.split_once('/')?;
    let address = canonicalize_address(address.parse().ok()?);
    let prefix: u8 = prefix.parse().ok()?;
    let maximum = if address.is_ipv4() { 32 } else { 128 };
    (prefix <= maximum).then_some((address, prefix))
}

fn cidr_contains(network: IpAddr, prefix: u8, address: IpAddr) -> bool {
    let address = canonicalize_address(address);
    match (network, address) {
        (IpAddr::V4(network), IpAddr::V4(address)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(prefix))
            };
            u32::from(network) & mask == u32::from(address) & mask
        }
        (IpAddr::V6(network), IpAddr::V6(address)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(prefix))
            };
            u128::from(network) & mask == u128::from(address) & mask
        }
        _ => false,
    }
}

fn parse_scope(value: &str) -> Option<AddressScope> {
    match value {
        "global" | "public" => Some(AddressScope::Global),
        "private" | "lan" => Some(AddressScope::Private),
        "loopback" | "localhost" => Some(AddressScope::Loopback),
        "link_local" | "link-local" => Some(AddressScope::LinkLocal),
        "multicast" => Some(AddressScope::Multicast),
        "cgnat" => Some(AddressScope::Cgnat),
        "documentation" => Some(AddressScope::Documentation),
        "benchmark" => Some(AddressScope::Benchmark),
        "reserved" => Some(AddressScope::Reserved),
        "broadcast" => Some(AddressScope::Broadcast),
        "unspecified" => Some(AddressScope::Unspecified),
        _ => None,
    }
}

fn parse_source(value: &str) -> Option<ObservationSource> {
    match value {
        "windows_ip_helper" | "ip_helper" => Some(ObservationSource::WindowsIpHelper),
        "windows_etw" | "etw" => Some(ObservationSource::WindowsEtw),
        "linux_inet_diag" | "inet_diag" | "netlink" => Some(ObservationSource::LinuxInetDiag),
        "local_dns" | "dns" => Some(ObservationSource::LocalDns),
        "local_mmdb" | "mmdb" | "geoip" => Some(ObservationSource::LocalMmdb),
        "demo" => Some(ObservationSource::Demo),
        "historical" | "history" => Some(ObservationSource::Historical),
        "unknown" => Some(ObservationSource::Unknown),
        _ => None,
    }
}

fn invalid(field: &str, value: &str) -> FilterError {
    FilterError::InvalidValue {
        field: field.into(),
        value: value.into(),
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" | "yes" | "1" => Some(true),
        "false" | "no" | "0" => Some(false),
        _ => None,
    }
}

fn parse_state(value: &str) -> Option<ConnectionState> {
    match value {
        "listen" | "listening" => Some(ConnectionState::Listening),
        "connecting" => Some(ConnectionState::Connecting),
        "active" | "established" => Some(ConnectionState::Established),
        "closing" => Some(ConnectionState::Closing),
        "closed" => Some(ConnectionState::Closed),
        "unknown" => Some(ConnectionState::Unknown),
        _ => None,
    }
}

fn tokenize(input: &str) -> Result<Vec<String>, FilterError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in input.chars() {
        match character {
            '"' => quoted = !quoted,
            value if value.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            value => current.push(value),
        }
    }
    if quoted {
        return Err(FilterError::UnterminatedQuote);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn searchable_text(connection: &Connection) -> String {
    let mut values = vec![
        connection.process.name.clone(),
        connection.key.local.address.to_string(),
        connection.key.remote.address.to_string(),
        connection.protocol.as_str().into(),
        connection.key.remote.port.to_string(),
    ];
    if let Some(domain) = &connection.domain {
        values.push(domain.clone());
    }
    if let Some(hostname) = &connection.hostname {
        values.push(hostname.clone());
    }
    if let Some(geo) = &connection.geo {
        values.extend(geo.country_code.iter().cloned());
        values.extend(geo.country_name.iter().cloned());
        values.extend(geo.city.iter().cloned());
    }
    if let Some(asn) = &connection.asn {
        values.push(asn.number.to_string());
        values.extend(asn.organization.iter().cloned());
    }
    values.join(" ").to_lowercase()
}

impl fmt::Display for ConnectionFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} filter terms", self.terms.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Connection, GeoLocation};

    #[test]
    fn parses_composed_query() {
        let filter: ConnectionFilter = "process:firefox country:US port:443 direction:out"
            .parse()
            .expect("valid filter");
        assert_eq!(filter.terms.len(), 4);
    }

    #[test]
    fn rejects_unknown_fields() {
        assert_eq!(
            "risk:high".parse::<ConnectionFilter>(),
            Err(FilterError::UnknownField("risk".into()))
        );
    }

    #[test]
    fn matches_typed_and_free_text_terms() {
        let mut connection = Connection::placeholder(0);
        connection.process.name = "Firefox".into();
        connection.key.remote.port = 443;
        connection.geo = Some(GeoLocation {
            country_code: Some("US".into()),
            country_name: Some("United States".into()),
            region: None,
            city: Some("New York".into()),
            latitude: Some(40.7),
            longitude: Some(-74.0),
            source: "test".into(),
            precision: crate::GeoPrecision::Coordinates,
            placement: crate::PlacementKind::DatabaseCoordinate,
            ..GeoLocation::default()
        });
        assert!(
            "process:fire country:US port:443 new"
                .parse::<ConnectionFilter>()
                .expect("filter")
                .matches(&connection)
        );
    }

    #[test]
    fn matches_cidr_scope_geo_source_and_negation() {
        let mut connection = Connection::placeholder(0);
        connection.key.remote = crate::Endpoint::new(
            "203.0.114.8".parse().expect("globally routable fixture"),
            443,
        );
        connection.evidence.observation_source = ObservationSource::WindowsIpHelper;
        connection.geo = Some(GeoLocation {
            country_code: Some("ES".into()),
            country_name: Some("Spain".into()),
            city: Some("Madrid".into()),
            ..GeoLocation::default()
        });
        assert!(
            "cidr:203.0.114.0/24 scope:global geo:madrid source:ip_helper -port:80"
                .parse::<ConnectionFilter>()
                .expect("advanced filter")
                .matches(&connection)
        );
        assert!(
            "!geo:paris"
                .parse::<ConnectionFilter>()
                .expect("negative filter")
                .matches(&connection)
        );
    }

    #[test]
    fn rejects_invalid_cidr_prefix_and_source() {
        assert!(matches!(
            "cidr:10.0.0.0/33".parse::<ConnectionFilter>(),
            Err(FilterError::InvalidValue { .. })
        ));
        assert!(matches!(
            "source:cloud".parse::<ConnectionFilter>(),
            Err(FilterError::InvalidValue { .. })
        ));
    }
}
