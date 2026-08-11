use crate::{Connection, ConnectionState, Direction, NetworkProtocol};
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
            let Some((field, value)) = token.split_once(':') else {
                terms.push(FilterTerm::Text(token.to_lowercase()));
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
                _ => return Err(FilterError::UnknownField(field.into())),
            };
            terms.push(term);
        }
        Ok(Self { terms })
    }
}

impl ConnectionFilter {
    pub fn matches(&self, connection: &Connection) -> bool {
        self.terms.iter().all(|term| match term {
            FilterTerm::Text(value) => searchable_text(connection).contains(value),
            FilterTerm::Process(value) => connection.process.name.to_lowercase().contains(value),
            FilterTerm::Country(value) => connection.geo.as_ref().is_some_and(|geo| {
                geo.country_code.eq_ignore_ascii_case(value)
                    || geo
                        .country_name
                        .to_lowercase()
                        .contains(&value.to_lowercase())
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
        })
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
        values.push(geo.country_code.clone());
        values.push(geo.country_name.clone());
        values.extend(geo.city.iter().cloned());
    }
    if let Some(asn) = &connection.asn {
        values.push(asn.number.to_string());
        values.push(asn.organization.clone());
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
            country_code: "US".into(),
            country_name: "United States".into(),
            region: None,
            city: Some("New York".into()),
            latitude: 40.7,
            longitude: -74.0,
            source: "test".into(),
        });
        assert!(
            "process:fire country:US port:443 new"
                .parse::<ConnectionFilter>()
                .expect("filter")
                .matches(&connection)
        );
    }
}
