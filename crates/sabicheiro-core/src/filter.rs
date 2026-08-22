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
    pub expression: Option<FilterExpression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterExpression {
    Term(FilterTerm),
    And(Vec<FilterExpression>),
    Or(Vec<FilterExpression>),
    Not(Box<FilterExpression>),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FilterError {
    #[error("unterminated quoted value")]
    UnterminatedQuote,
    #[error("unknown filter field: {0}")]
    UnknownField(String),
    #[error("invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
    #[error("unexpected query token: {0}")]
    UnexpectedToken(String),
    #[error("missing closing parenthesis at position {0}")]
    MissingClosingParenthesis(usize),
    #[error("query operator is missing an operand at position {0}")]
    MissingOperand(usize),
}

impl FromStr for ConnectionFilter {
    type Err = FilterError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let lexemes = tokenize_expression(input)?;
        if lexemes.is_empty() {
            return Ok(Self::default());
        }
        let mut parser = ExpressionParser::new(&lexemes);
        let expression = parser.parse_or()?;
        if let Some(token) = parser.peek() {
            return Err(FilterError::UnexpectedToken(token.label()));
        }
        let mut terms = Vec::new();
        collect_terms(&expression, &mut terms);
        Ok(Self {
            terms,
            expression: Some(expression),
        })
    }
}

impl ConnectionFilter {
    pub fn matches(&self, connection: &Connection) -> bool {
        self.expression.as_ref().map_or_else(
            || self.terms.iter().all(|term| matches_term(term, connection)),
            |expression| matches_expression(expression, connection),
        )
    }

    #[must_use]
    pub fn to_query_string(&self) -> String {
        self.expression
            .as_ref()
            .map(expression_to_query)
            .unwrap_or_else(|| {
                self.terms
                    .iter()
                    .map(term_to_query)
                    .collect::<Vec<_>>()
                    .join(" AND ")
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum QueryLexeme {
    Word(String),
    And,
    Or,
    Not,
    LeftParenthesis(usize),
    RightParenthesis,
}

impl QueryLexeme {
    fn label(&self) -> String {
        match self {
            Self::Word(value) => value.clone(),
            Self::And => "AND".into(),
            Self::Or => "OR".into(),
            Self::Not => "NOT".into(),
            Self::LeftParenthesis(_) => "(".into(),
            Self::RightParenthesis => ")".into(),
        }
    }
}

struct ExpressionParser<'a> {
    lexemes: &'a [QueryLexeme],
    position: usize,
}

impl<'a> ExpressionParser<'a> {
    fn new(lexemes: &'a [QueryLexeme]) -> Self {
        Self {
            lexemes,
            position: 0,
        }
    }

    fn peek(&self) -> Option<&'a QueryLexeme> {
        self.lexemes.get(self.position)
    }

    fn next(&mut self) -> Option<&'a QueryLexeme> {
        let value = self.peek();
        self.position = self.position.saturating_add(usize::from(value.is_some()));
        value
    }

    fn parse_or(&mut self) -> Result<FilterExpression, FilterError> {
        let mut items = vec![self.parse_and()?];
        while matches!(self.peek(), Some(QueryLexeme::Or)) {
            self.next();
            if self.peek().is_none() || matches!(self.peek(), Some(QueryLexeme::RightParenthesis)) {
                return Err(FilterError::MissingOperand(self.position));
            }
            items.push(self.parse_and()?);
        }
        Ok(if items.len() == 1 {
            items.remove(0)
        } else {
            FilterExpression::Or(items)
        })
    }

    fn parse_and(&mut self) -> Result<FilterExpression, FilterError> {
        let mut items = vec![self.parse_primary()?];
        loop {
            match self.peek() {
                None | Some(QueryLexeme::Or | QueryLexeme::RightParenthesis) => break,
                Some(QueryLexeme::And) => {
                    self.next();
                    if self.peek().is_none()
                        || matches!(
                            self.peek(),
                            Some(
                                QueryLexeme::And | QueryLexeme::Or | QueryLexeme::RightParenthesis
                            )
                        )
                    {
                        return Err(FilterError::MissingOperand(self.position));
                    }
                    items.push(self.parse_primary()?);
                }
                _ => items.push(self.parse_primary()?),
            }
        }
        Ok(if items.len() == 1 {
            items.remove(0)
        } else {
            FilterExpression::And(items)
        })
    }

    fn parse_primary(&mut self) -> Result<FilterExpression, FilterError> {
        match self.next() {
            Some(QueryLexeme::Word(value)) => Ok(FilterExpression::Term(parse_filter_term(value)?)),
            Some(QueryLexeme::Not) => {
                if self.peek().is_none() {
                    return Err(FilterError::MissingOperand(self.position));
                }
                Ok(FilterExpression::Not(Box::new(self.parse_primary()?)))
            }
            Some(QueryLexeme::LeftParenthesis(source_position)) => {
                if matches!(self.peek(), Some(QueryLexeme::RightParenthesis)) {
                    return Err(FilterError::MissingOperand(self.position));
                }
                let expression = self.parse_or()?;
                if !matches!(self.next(), Some(QueryLexeme::RightParenthesis)) {
                    return Err(FilterError::MissingClosingParenthesis(*source_position));
                }
                Ok(expression)
            }
            Some(token) => Err(FilterError::UnexpectedToken(token.label())),
            None => Err(FilterError::MissingOperand(self.position)),
        }
    }
}

fn tokenize_expression(input: &str) -> Result<Vec<QueryLexeme>, FilterError> {
    fn push_word(tokens: &mut Vec<QueryLexeme>, current: &mut String, quoted: bool) {
        if current.is_empty() {
            return;
        }
        let value = std::mem::take(current);
        if quoted {
            tokens.push(QueryLexeme::Word(value));
            return;
        }
        match value.to_ascii_uppercase().as_str() {
            "AND" => tokens.push(QueryLexeme::And),
            "OR" => tokens.push(QueryLexeme::Or),
            "NOT" => tokens.push(QueryLexeme::Not),
            _ => tokens.push(QueryLexeme::Word(value)),
        }
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut word_had_quote = false;
    for (position, character) in input.char_indices() {
        match character {
            '"' => {
                quoted = !quoted;
                word_had_quote = true;
            }
            '(' if !quoted => {
                push_word(&mut tokens, &mut current, word_had_quote);
                word_had_quote = false;
                tokens.push(QueryLexeme::LeftParenthesis(position));
            }
            ')' if !quoted => {
                push_word(&mut tokens, &mut current, word_had_quote);
                word_had_quote = false;
                tokens.push(QueryLexeme::RightParenthesis);
            }
            value if value.is_whitespace() && !quoted => {
                push_word(&mut tokens, &mut current, word_had_quote);
                word_had_quote = false;
            }
            value => current.push(value),
        }
    }
    if quoted {
        return Err(FilterError::UnterminatedQuote);
    }
    push_word(&mut tokens, &mut current, word_had_quote);
    Ok(tokens)
}

fn parse_filter_term(token: &str) -> Result<FilterTerm, FilterError> {
    let (negated, token) = token
        .strip_prefix(['-', '!'])
        .filter(|value| !value.is_empty())
        .map_or((false, token), |value| (true, value));
    let term = if let Some((field, value)) = token.split_once(':') {
        if value.is_empty() {
            return Err(FilterError::InvalidValue {
                field: field.into(),
                value: value.into(),
            });
        }
        let lower = value.to_lowercase();
        match field.to_lowercase().as_str() {
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
            "state" => FilterTerm::State(parse_state(&lower).ok_or_else(|| invalid(field, value))?),
            "firstseen" => {
                FilterTerm::FirstSeen(parse_bool(&lower).ok_or_else(|| invalid(field, value))?)
            }
            "asn" => FilterTerm::Asn(value.parse().map_err(|_| invalid(field, value))?),
            "cidr" => {
                let (address, prefix) = parse_cidr(value).ok_or_else(|| invalid(field, value))?;
                FilterTerm::Cidr(address, prefix)
            }
            "scope" => FilterTerm::Scope(parse_scope(&lower).ok_or_else(|| invalid(field, value))?),
            "geo" => FilterTerm::Geo(lower),
            "source" => {
                FilterTerm::Source(parse_source(&lower).ok_or_else(|| invalid(field, value))?)
            }
            _ => return Err(FilterError::UnknownField(field.into())),
        }
    } else {
        FilterTerm::Text(token.to_lowercase())
    };
    Ok(if negated {
        FilterTerm::Not(Box::new(term))
    } else {
        term
    })
}

fn collect_terms(expression: &FilterExpression, terms: &mut Vec<FilterTerm>) {
    match expression {
        FilterExpression::Term(term) => terms.push(term.clone()),
        FilterExpression::And(items) | FilterExpression::Or(items) => {
            for item in items {
                collect_terms(item, terms);
            }
        }
        FilterExpression::Not(item) => collect_terms(item, terms),
    }
}

fn quote_query_value(value: &str) -> String {
    if value
        .chars()
        .any(|character| character.is_whitespace() || matches!(character, '(' | ')'))
    {
        format!("\"{value}\"")
    } else {
        value.into()
    }
}

fn term_to_query(term: &FilterTerm) -> String {
    match term {
        FilterTerm::Text(value) => quote_query_value(value),
        FilterTerm::Process(value) => format!("process:{}", quote_query_value(value)),
        FilterTerm::Country(value) => format!("country:{value}"),
        FilterTerm::Protocol(value) => format!("protocol:{}", value.as_str()),
        FilterTerm::Port(value) => format!("port:{value}"),
        FilterTerm::Direction(Direction::Inbound) => "direction:in".into(),
        FilterTerm::Direction(Direction::Outbound) => "direction:out".into(),
        FilterTerm::Direction(Direction::Unknown) => "direction:unknown".into(),
        FilterTerm::Ip(value) => format!("ip:{value}"),
        FilterTerm::Domain(value) => format!("domain:{}", quote_query_value(value)),
        FilterTerm::State(value) => format!(
            "state:{}",
            match value {
                ConnectionState::Listening => "listening",
                ConnectionState::Connecting => "connecting",
                ConnectionState::Established => "established",
                ConnectionState::Closing => "closing",
                ConnectionState::Closed => "closed",
                ConnectionState::Unknown => "unknown",
            }
        ),
        FilterTerm::FirstSeen(value) => format!("firstseen:{value}"),
        FilterTerm::Asn(value) => format!("asn:{value}"),
        FilterTerm::Cidr(address, prefix) => format!("cidr:{address}/{prefix}"),
        FilterTerm::Scope(value) => format!(
            "scope:{}",
            match value {
                AddressScope::Global => "global",
                AddressScope::Private => "private",
                AddressScope::Loopback => "loopback",
                AddressScope::LinkLocal => "link_local",
                AddressScope::Multicast => "multicast",
                AddressScope::Cgnat => "cgnat",
                AddressScope::Documentation => "documentation",
                AddressScope::Benchmark => "benchmark",
                AddressScope::Reserved => "reserved",
                AddressScope::Broadcast => "broadcast",
                AddressScope::Unspecified => "unspecified",
            }
        ),
        FilterTerm::Geo(value) => format!("geo:{}", quote_query_value(value)),
        FilterTerm::Source(value) => format!(
            "source:{}",
            match value {
                ObservationSource::WindowsIpHelper => "windows_ip_helper",
                ObservationSource::WindowsEtw => "windows_etw",
                ObservationSource::WindowsHelper => "windows_helper",
                ObservationSource::LinuxInetDiag => "linux_inet_diag",
                ObservationSource::LinuxEbpf => "linux_ebpf",
                ObservationSource::LocalDns => "local_dns",
                ObservationSource::LocalMmdb => "local_mmdb",
                ObservationSource::HandshakeMetadata => "handshake_metadata",
                ObservationSource::OfflineCatalog => "offline_catalog",
                ObservationSource::UserAnnotation => "user_annotation",
                ObservationSource::Demo => "demo",
                ObservationSource::Historical => "historical",
                ObservationSource::Unknown => "unknown",
            }
        ),
        FilterTerm::Not(inner) => format!("-{}", term_to_query(inner)),
    }
}

fn expression_to_query(expression: &FilterExpression) -> String {
    expression_to_query_at_precedence(expression, 0)
}

fn expression_to_query_at_precedence(expression: &FilterExpression, parent: u8) -> String {
    let (precedence, value) = match expression {
        FilterExpression::Term(term) => (3, term_to_query(term)),
        FilterExpression::Not(item) => (
            3,
            format!("NOT ({})", expression_to_query_at_precedence(item, 0)),
        ),
        FilterExpression::And(items) => (
            2,
            items
                .iter()
                .map(|item| expression_to_query_at_precedence(item, 2))
                .collect::<Vec<_>>()
                .join(" AND "),
        ),
        FilterExpression::Or(items) => (
            1,
            items
                .iter()
                .map(|item| expression_to_query_at_precedence(item, 1))
                .collect::<Vec<_>>()
                .join(" OR "),
        ),
    };
    if precedence < parent {
        format!("({value})")
    } else {
        value
    }
}

fn matches_expression(expression: &FilterExpression, connection: &Connection) -> bool {
    match expression {
        FilterExpression::Term(term) => matches_term(term, connection),
        FilterExpression::And(items) => items
            .iter()
            .all(|item| matches_expression(item, connection)),
        FilterExpression::Or(items) => items
            .iter()
            .any(|item| matches_expression(item, connection)),
        FilterExpression::Not(item) => !matches_expression(item, connection),
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
        FilterTerm::Geo(value) if value == "unlocated" => connection
            .geo
            .as_ref()
            .and_then(|geo| geo.coordinates())
            .is_none(),
        FilterTerm::Geo(value) if value == "plotted" => connection
            .geo
            .as_ref()
            .and_then(|geo| geo.coordinates())
            .is_some(),
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
        "windows_helper" | "helper" => Some(ObservationSource::WindowsHelper),
        "linux_inet_diag" | "inet_diag" | "netlink" => Some(ObservationSource::LinuxInetDiag),
        "linux_ebpf" | "ebpf" => Some(ObservationSource::LinuxEbpf),
        "local_dns" | "dns" => Some(ObservationSource::LocalDns),
        "local_mmdb" | "mmdb" | "geoip" => Some(ObservationSource::LocalMmdb),
        "handshake_metadata" | "handshake" => Some(ObservationSource::HandshakeMetadata),
        "offline_catalog" | "catalog" => Some(ObservationSource::OfflineCatalog),
        "user_annotation" | "annotation" => Some(ObservationSource::UserAnnotation),
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
    fn distinguishes_unlocated_from_plotted_geo_evidence() {
        let unlocated = Connection::placeholder(0);
        assert!(
            "geo:unlocated"
                .parse::<ConnectionFilter>()
                .expect("unlocated filter")
                .matches(&unlocated)
        );
        assert!(
            !"geo:plotted"
                .parse::<ConnectionFilter>()
                .expect("plotted filter")
                .matches(&unlocated)
        );

        let mut plotted = Connection::placeholder(1);
        plotted.geo = Some(GeoLocation {
            latitude: Some(40.4168),
            longitude: Some(-3.7038),
            precision: crate::GeoPrecision::Coordinates,
            placement: crate::PlacementKind::DatabaseCoordinate,
            ..GeoLocation::default()
        });
        assert!(
            "geo:plotted"
                .parse::<ConnectionFilter>()
                .expect("plotted filter")
                .matches(&plotted)
        );
        assert!(
            !"geo:unlocated"
                .parse::<ConnectionFilter>()
                .expect("unlocated filter")
                .matches(&plotted)
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

    #[test]
    fn boolean_groups_respect_and_precedence_and_negation() {
        let mut connection = Connection::placeholder(0);
        connection.process.name = "Firefox Developer Edition".into();
        connection.key.remote.port = 443;
        connection.protocol = NetworkProtocol::Tcp;
        let filter: ConnectionFilter =
            "(process:\"Firefox Developer Edition\" AND port:443) OR (protocol:udp AND NOT (port:53))"
                .parse()
                .expect("valid grouped filter");
        assert!(filter.matches(&connection));
        connection.key.remote.port = 80;
        assert!(!filter.matches(&connection));
    }

    #[test]
    fn editor_round_trip_preserves_filter_semantics() {
        let queries = [
            "process:firefox AND port:443",
            "country:ES OR country:PT",
            "(source:etw OR source:ebpf) AND -scope:loopback",
            "NOT (state:closed OR state:closing) AND protocol:tcp",
            "domain:\"example service.test\" AND firstseen:true",
            "(cidr:10.0.0.0/8 AND direction:out) OR ip:2001:db8::1",
        ];
        for query in queries {
            let parsed: ConnectionFilter = query.parse().expect(query);
            let serialized = parsed.to_query_string();
            let reparsed: ConnectionFilter = serialized.parse().expect(&serialized);
            assert_eq!(
                parsed.expression, reparsed.expression,
                "{query} -> {serialized}"
            );
        }
    }

    #[test]
    fn malformed_boolean_queries_report_structural_errors() {
        assert!(matches!(
            "process:x OR".parse::<ConnectionFilter>(),
            Err(FilterError::MissingOperand(_))
        ));
        assert!(matches!(
            "(process:x AND port:443".parse::<ConnectionFilter>(),
            Err(FilterError::MissingClosingParenthesis(_))
        ));
        assert!(matches!(
            "process:x AND AND port:443".parse::<ConnectionFilter>(),
            Err(FilterError::MissingOperand(_))
        ));
    }
}
