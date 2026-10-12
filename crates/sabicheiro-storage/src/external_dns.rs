use super::*;
use sabicheiro_core::{
    DnsImportReport, ExternalDnsFilter, ExternalDnsPage, ExternalDnsQuery, ExternalDnsRow,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) const EXTERNAL_DNS_SQL: &str = "
CREATE TABLE external_dns_queries (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 fingerprint TEXT NOT NULL UNIQUE,
 timestamp_ms INTEGER NOT NULL,
 client_ip TEXT NOT NULL,
 query_name TEXT NOT NULL,
 source TEXT NOT NULL,
 imported_at_ms INTEGER NOT NULL,
 query_json TEXT NOT NULL
);
CREATE INDEX idx_external_dns_time ON external_dns_queries(timestamp_ms, id);
CREATE INDEX idx_external_dns_client ON external_dns_queries(client_ip, timestamp_ms);
";

impl Storage {
    pub fn database_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn import_external_dns(
        &mut self,
        source: &str,
        queries: &[ExternalDnsQuery],
        cancelled: &AtomicBool,
    ) -> Result<DnsImportReport, StorageError> {
        let transaction = self.connection.transaction()?;
        let mut report = DnsImportReport::default();
        let imported_at_ms = sabicheiro_core::now_ms();
        {
            let mut insert = transaction.prepare_cached("INSERT OR IGNORE INTO external_dns_queries(fingerprint,timestamp_ms,client_ip,query_name,source,imported_at_ms,query_json) VALUES (?1,?2,?3,?4,?5,?6,?7)")?;
            for query in queries {
                if cancelled.load(Ordering::Acquire) {
                    return Err(StorageError::InvalidEvidence(
                        "DNS import cancelled; no queries were committed".into(),
                    ));
                }
                let json = serde_json::to_string(query)?;
                let fingerprint = format!(
                    "{:x}",
                    Sha256::digest(format!("{}\0{}", source.trim(), json).as_bytes())
                );
                let count = insert.execute(params![
                    fingerprint,
                    query.timestamp_ms,
                    query.client_ip.to_string(),
                    query.query_name,
                    source.trim(),
                    imported_at_ms,
                    json
                ])?;
                report.inserted += count;
                report.duplicates += usize::from(count == 0);
            }
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(StorageError::InvalidEvidence(
                "DNS import cancelled; no queries were committed".into(),
            ));
        }
        transaction.commit()?;
        Ok(report)
    }

    pub fn query_external_dns(
        &self,
        filter: &ExternalDnsFilter,
    ) -> Result<ExternalDnsPage, StorageError> {
        if filter
            .from_ms
            .zip(filter.to_ms)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(StorageError::InvalidEvidence(
                "The start must precede the end".into(),
            ));
        }
        let limit = filter.limit.clamp(1, 500);
        let mut statement = self.connection.prepare_cached("SELECT id, source, imported_at_ms, query_json FROM external_dns_queries WHERE (?1 IS NULL OR timestamp_ms >= ?1) AND (?2 IS NULL OR timestamp_ms <= ?2) AND (?3 IS NULL OR client_ip = ?3) AND (?4 IS NULL OR instr(query_name, ?4) > 0) AND (?5 IS NULL OR id < ?5) ORDER BY id DESC LIMIT ?6")?;
        let raw = statement
            .query_map(
                params![
                    filter.from_ms,
                    filter.to_ms,
                    filter.client_ip.map(|ip| ip.to_string()),
                    filter
                        .domain
                        .as_ref()
                        .map(|value| value.to_ascii_lowercase()),
                    filter.before_id,
                    limit as i64 + 1
                ],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let mut rows = raw
            .into_iter()
            .map(|(id, source, imported_at_ms, json)| {
                Ok(ExternalDnsRow {
                    id,
                    source,
                    imported_at_ms,
                    query: serde_json::from_str(&json)?,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        let next_id = if has_more {
            rows.last().map(|row| row.id)
        } else {
            None
        };
        Ok(ExternalDnsPage { rows, next_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_is_atomic_deduplicated_and_paginated() {
        let mut storage = Storage::in_memory().expect("storage");
        let query = ExternalDnsQuery {
            timestamp_ms: 1_000,
            client_ip: "192.168.1.20".parse().expect("IP"),
            query_name: "example.org".into(),
            query_type: None,
            result: None,
        };
        let queries = vec![
            query.clone(),
            ExternalDnsQuery {
                timestamp_ms: 2_000,
                ..query
            },
        ];
        let cancel = AtomicBool::new(true);
        assert!(
            storage
                .import_external_dns("resolver", &queries, &cancel)
                .is_err()
        );
        cancel.store(false, Ordering::Release);
        assert_eq!(
            storage
                .import_external_dns("resolver", &queries, &cancel)
                .expect("first")
                .inserted,
            2
        );
        assert_eq!(
            storage
                .import_external_dns("resolver", &queries, &cancel)
                .expect("retry")
                .duplicates,
            2
        );
        let mut filter = ExternalDnsFilter {
            from_ms: None,
            to_ms: None,
            client_ip: None,
            domain: Some("example".into()),
            before_id: None,
            limit: 1,
        };
        let first = storage.query_external_dns(&filter).expect("first page");
        assert_eq!(first.rows.len(), 1);
        filter.before_id = first.next_id;
        let second = storage.query_external_dns(&filter).expect("second page");
        assert_eq!(second.rows.len(), 1);
        assert_ne!(first.rows[0].id, second.rows[0].id);
        assert!(second.next_id.is_none());
        filter.from_ms = Some(3_000);
        assert!(
            storage
                .query_external_dns(&filter)
                .expect("filtered")
                .rows
                .is_empty()
        );
    }

    #[test]
    fn version_seven_upgrade_keeps_evidence_and_readable_backup() {
        let directory = tempfile::tempdir().expect("directory");
        let path = directory.path().join("history.sqlite3");
        let storage = Storage::open(&path).expect("initial storage");
        storage.start_session("retained", 1_000).expect("session");
        storage
            .connection
            .execute_batch(
                "DROP TABLE external_dns_queries; DELETE FROM schema_migrations WHERE version = 8;",
            )
            .expect("version seven fixture");
        drop(storage);
        let storage = Storage::open(&path).expect("upgrade");
        assert_eq!(
            storage
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM sessions WHERE id = 'retained'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .expect("retained session"),
            1
        );
        assert_eq!(
            storage
                .connection
                .query_row("SELECT COUNT(*) FROM external_dns_queries", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .expect("DNS table"),
            0
        );
        let backup =
            SqliteConnection::open(path.with_extension("pre-v8-from-v7.bak")).expect("backup");
        assert_eq!(
            backup
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .expect("original version"),
            7
        );
        assert_eq!(
            backup
                .query_row(
                    "SELECT COUNT(*) FROM sessions WHERE id = 'retained'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .expect("backup evidence"),
            1
        );
    }
}
