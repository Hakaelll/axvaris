use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub const MAX_DNS_IMPORT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DNS_IMPORT_ROWS: usize = 100_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsImportFormat {
    Csv,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DnsImportRequest {
    pub format: DnsImportFormat,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalDnsQuery {
    pub timestamp_ms: i64,
    pub client_ip: IpAddr,
    pub query_name: String,
    pub query_type: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDnsRow {
    pub id: i64,
    pub source: String,
    pub imported_at_ms: i64,
    pub query: ExternalDnsQuery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsImportPreview {
    pub total_rows: usize,
    pub source: String,
    pub sample: Vec<ExternalDnsQuery>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DnsImportReport {
    pub inserted: usize,
    pub duplicates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalDnsFilter {
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub client_ip: Option<IpAddr>,
    pub domain: Option<String>,
    pub before_id: Option<i64>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDnsPage {
    pub rows: Vec<ExternalDnsRow>,
    pub next_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawQuery {
    timestamp: String,
    client_ip: String,
    query_name: String,
    #[serde(default)]
    query_type: Option<String>,
    #[serde(default)]
    result: Option<String>,
}

pub fn parse_dns_import(request: &DnsImportRequest) -> Result<Vec<ExternalDnsQuery>, String> {
    let source = request.source.trim();
    if source.is_empty() || source.len() > 128 || source.chars().any(char::is_control) {
        return Err("Enter a source name between 1 and 128 characters.".into());
    }
    if request.content.len() > MAX_DNS_IMPORT_BYTES {
        return Err("The import exceeds the 16 MiB limit.".into());
    }
    let text = request.content.trim_start_matches('\u{feff}');
    let raw: Vec<RawQuery> = match request.format {
        DnsImportFormat::Json => {
            serde_json::from_str(text).map_err(|error| format!("Invalid JSON: {error}"))?
        }
        DnsImportFormat::Csv => {
            let mut records = csv_records(text)?.into_iter();
            let headers = records.next().ok_or("The CSV header is missing.")?;
            let allowed = [
                "timestamp",
                "client_ip",
                "query_name",
                "query_type",
                "result",
            ];
            if headers.iter().any(|h| !allowed.contains(&h.as_str()))
                || headers
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != headers.len()
            {
                return Err(
                    "Use unique columns: timestamp, client_ip, query_name, query_type, result."
                        .into(),
                );
            }
            for required in &allowed[..3] {
                if !headers.iter().any(|h| h == required) {
                    return Err(format!("Missing CSV column: {required}"));
                }
            }
            records
                .enumerate()
                .map(|(index, fields)| {
                    if fields.len() != headers.len() {
                        return Err(format!(
                            "CSV row {} has the wrong number of columns.",
                            index + 2
                        ));
                    }
                    let values = headers
                        .iter()
                        .zip(fields)
                        .map(|(key, value)| (key.clone(), serde_json::Value::String(value)))
                        .collect::<serde_json::Map<_, _>>();
                    serde_json::from_value(serde_json::Value::Object(values))
                        .map_err(|error| format!("CSV row {}: {error}", index + 2))
                })
                .collect::<Result<Vec<_>, String>>()?
        }
    };
    if raw.is_empty() || raw.len() > MAX_DNS_IMPORT_ROWS {
        return Err("Import between 1 and 100,000 queries.".into());
    }
    raw.into_iter()
        .enumerate()
        .map(|(index, query)| {
            let validate = || -> Result<ExternalDnsQuery, String> {
                let timestamp_ms = chrono::DateTime::parse_from_rfc3339(query.timestamp.trim())
                    .map_err(|_| "Use an ISO 8601 timestamp with an explicit timezone.".to_owned())?
                    .timestamp_millis();
                let client_ip = query
                    .client_ip
                    .trim()
                    .parse()
                    .map_err(|_| "Invalid client IP address.".to_owned())?;
                let query_name = query
                    .query_name
                    .trim()
                    .trim_end_matches('.')
                    .to_ascii_lowercase();
                if query_name.is_empty()
                    || query_name.len() > 253
                    || query_name.split('.').any(|label| {
                        label.is_empty()
                            || label.len() > 63
                            || !label
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                    })
                {
                    return Err("Invalid domain. Use ASCII or punycode DNS names.".into());
                }
                fn optional(value: &Option<String>) -> Result<Option<String>, String> {
                    value
                        .as_deref()
                        .map(str::trim)
                        .filter(|v| !v.is_empty())
                        .map(|v| {
                            if v.len() > 128 || v.chars().any(char::is_control) {
                                Err("Query type or result exceeds its limit.".into())
                            } else {
                                Ok(v.to_owned())
                            }
                        })
                        .transpose()
                }
                Ok(ExternalDnsQuery {
                    timestamp_ms,
                    client_ip,
                    query_name,
                    query_type: optional(&query.query_type)?
                        .map(|value| value.to_ascii_uppercase()),
                    result: optional(&query.result)?,
                })
            };
            validate().map_err(|error| format!("Row {}: {error}", index + 1))
        })
        .collect()
}

fn csv_records(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut records = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    let mut closed = false;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' if field.is_empty() && !closed => quoted = true,
                ',' => {
                    row.push(std::mem::take(&mut field));
                    closed = false;
                }
                '\n' | '\r' => {
                    if c == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    row.push(std::mem::take(&mut field));
                    if row.iter().any(|v| !v.is_empty()) {
                        records.push(std::mem::take(&mut row));
                    } else {
                        row.clear();
                    }
                    closed = false;
                    if records.len() > MAX_DNS_IMPORT_ROWS + 1 {
                        return Err("The CSV exceeds 100,000 queries.".into());
                    }
                }
                _ if closed || c == '"' => return Err("Invalid quoting in CSV.".into()),
                _ => field.push(c),
            }
        }
    }
    if quoted {
        return Err("Unclosed CSV quotation.".into());
    }
    if !field.is_empty() || !row.is_empty() || closed {
        row.push(field);
        records.push(row);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(format: DnsImportFormat, content: &str) -> DnsImportRequest {
        DnsImportRequest {
            format,
            content: content.into(),
            source: "Home resolver".into(),
        }
    }
    #[test]
    fn csv_and_json_normalize_to_the_same_query() {
        let csv = request(
            DnsImportFormat::Csv,
            "timestamp,client_ip,query_name,query_type,result\r\n\"2026-10-12T12:00:00+02:00\",192.168.1.20,EXAMPLE.ORG.,a,NOERROR\r\n",
        );
        let json = request(
            DnsImportFormat::Json,
            r#"[{"timestamp":"2026-10-12T10:00:00Z","client_ip":"192.168.1.20","query_name":"example.org","query_type":"A","result":"NOERROR"}]"#,
        );
        assert_eq!(
            parse_dns_import(&csv).expect("CSV"),
            parse_dns_import(&json).expect("JSON")
        );
    }
    #[test]
    fn invalid_rows_and_missing_timezones_are_rejected() {
        for row in [
            "2026-10-12T10:00:00,192.168.1.1,example.org",
            "2026-10-12T10:00:00Z,no-ip,example.org",
            "2026-10-12T10:00:00Z,192.168.1.1,https://example.org",
        ] {
            assert!(
                parse_dns_import(&request(
                    DnsImportFormat::Csv,
                    &format!("timestamp,client_ip,query_name\n{row}")
                ))
                .is_err()
            );
        }
        assert!(csv_records("a\n\"unterminated").is_err());
        assert_eq!(
            csv_records("a,b\n\"two,words\",\"a\"\"b\"").expect("quoted"),
            vec![vec!["a", "b"], vec!["two,words", "a\"b"]]
        );
    }
}
