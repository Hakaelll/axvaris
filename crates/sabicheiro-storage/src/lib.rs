use rusqlite::{Connection as SqliteConnection, OptionalExtension, Transaction, params};
use sabicheiro_core::{
    AppSettings, BaselineBook, Bookmark, Connection, ConnectionFilter, ConnectionHistoryStats,
    CounterSource, ExportFormat, ExportRequest, ExportResult, FocusRules, HistoryEntry,
    HistoryQuery, IgnoreRule, RecordingState, RecordingStatus, RecordingSummary, RetentionPolicy,
    RuleKind, SessionSummary, TimelineQuery, TrafficCounters, TrafficSample, TransferRates,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

const SCHEMA_VERSION: i64 = 3;
const MAX_QUERY_LIMIT: usize = 10_000;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid export path: {0}")]
    InvalidExportPath(String),
    #[error("invalid history filter: {0}")]
    InvalidFilter(String),
    #[error("invalid local focus rule: {0}")]
    InvalidRule(String),
    #[error("invalid recording metadata: {0}")]
    InvalidRecording(String),
}

pub struct Storage {
    connection: SqliteConnection,
    path: Option<PathBuf>,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = SqliteConnection::open(path)?;
        let mut storage = Self {
            connection,
            path: Some(path.to_path_buf()),
        };
        storage.configure()?;
        storage.migrate()?;
        Ok(storage)
    }

    pub fn in_memory() -> Result<Self, StorageError> {
        let connection = SqliteConnection::open_in_memory()?;
        let mut storage = Self {
            connection,
            path: None,
        };
        storage.configure()?;
        storage.migrate()?;
        Ok(storage)
    }

    fn configure(&self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 3000;",
        )?;
        Ok(())
    }

    fn migrate(&mut self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
               version INTEGER PRIMARY KEY,
               applied_at_ms INTEGER NOT NULL
             );",
        )?;
        let current = self
            .connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, Option<i64>>(0)
            })?
            .unwrap_or(0);
        if current < 1 {
            let transaction = self.connection.transaction()?;
            migration_v1(&transaction)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![1, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        if current < 2 {
            let transaction = self.connection.transaction()?;
            migration_v2(&transaction)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![2, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        if current < 3 {
            let transaction = self.connection.transaction()?;
            migration_v3(&transaction)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![SCHEMA_VERSION, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        Ok(())
    }

    pub fn start_session(&self, id: &str, started_at_ms: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO sessions(id, started_at_ms) VALUES (?1, ?2)
             ON CONFLICT(id) DO NOTHING",
            params![id, started_at_ms],
        )?;
        Ok(())
    }

    pub fn finish_session(&self, summary: &SessionSummary) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE sessions SET
               ended_at_ms = ?2,
               connection_count = ?3,
               process_count = ?4,
               country_count = ?5,
               destination_count = ?6,
               recording_count = ?7,
               bytes_sent = ?8,
               bytes_received = ?9,
               packets_sent = ?10,
               packets_received = ?11
             WHERE id = ?1",
            params![
                summary.id,
                summary.ended_at_ms,
                summary.connection_count as i64,
                summary.process_count as i64,
                summary.country_count as i64,
                summary.destination_count as i64,
                summary.recording_count as i64,
                to_sql_u64(summary.traffic.bytes_sent),
                to_sql_u64(summary.traffic.bytes_received),
                to_sql_u64(summary.traffic.packets_sent),
                to_sql_u64(summary.traffic.packets_received),
            ],
        )?;
        Ok(())
    }

    pub fn save_connections(
        &mut self,
        session_id: &str,
        connections: &[Connection],
    ) -> Result<(), StorageError> {
        if connections.is_empty() {
            return Ok(());
        }
        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare_cached(
                "INSERT INTO connections(
                   id, session_id, first_seen_ms, last_seen_ms, process_name, pid,
                   remote_ip, remote_port, local_ip, local_port, protocol, direction,
                   state, domain, country_code, asn, active, connection_json
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, ?17, ?18
                 )
                 ON CONFLICT(id) DO UPDATE SET
                   last_seen_ms=excluded.last_seen_ms,
                   process_name=excluded.process_name,
                   pid=excluded.pid,
                   state=excluded.state,
                   domain=excluded.domain,
                   country_code=excluded.country_code,
                   asn=excluded.asn,
                   active=excluded.active,
                   connection_json=excluded.connection_json",
            )?;
            for connection in connections {
                let json = serde_json::to_string(connection)?;
                statement.execute(params![
                    connection.id,
                    session_id,
                    connection.first_seen_ms,
                    connection.last_seen_ms,
                    connection.process.name,
                    connection.process.pid.map(i64::from),
                    connection.key.remote.address.to_string(),
                    i64::from(connection.key.remote.port),
                    connection.key.local.address.to_string(),
                    i64::from(connection.key.local.port),
                    connection.protocol.as_str(),
                    format!("{:?}", connection.direction).to_lowercase(),
                    format!("{:?}", connection.state).to_lowercase(),
                    connection
                        .domain
                        .as_deref()
                        .or(connection.hostname.as_deref()),
                    connection.geo.as_ref().map(|geo| geo.country_code.as_str()),
                    connection.asn.as_ref().map(|asn| i64::from(asn.number)),
                    i64::from(connection.active),
                    json,
                ])?;
            }
        }
        transaction.execute(
            "UPDATE sessions SET connection_count = (
               SELECT COUNT(*) FROM connections WHERE session_id = ?1
             ) WHERE id = ?1",
            params![session_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn query_history(&self, query: &HistoryQuery) -> Result<Vec<HistoryEntry>, StorageError> {
        let limit = query.limit.clamp(1, MAX_QUERY_LIMIT) as i64;
        let mut statement = self.connection.prepare(
            "SELECT session_id, connection_json FROM connections
             WHERE (?1 IS NULL OR last_seen_ms >= ?1)
               AND (?2 IS NULL OR first_seen_ms <= ?2)
               AND (?3 IS NULL OR session_id = ?3)
             ORDER BY last_seen_ms DESC
             LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![query.from_ms, query.to_ms, query.session_id, limit],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        let filter = query
            .query
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::parse::<ConnectionFilter>)
            .transpose()
            .map_err(|error| StorageError::InvalidFilter(error.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            let (session_id, json) = row?;
            let connection: Connection = serde_json::from_str(&json)?;
            if filter
                .as_ref()
                .is_none_or(|candidate| candidate.matches(&connection))
            {
                entries.push(HistoryEntry {
                    session_id,
                    connection,
                });
            }
        }
        Ok(entries)
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<SessionSummary>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, started_at_ms, ended_at_ms, connection_count, process_count,
                    country_count, destination_count, recording_count, bytes_sent,
                    bytes_received, packets_sent, packets_received
             FROM sessions ORDER BY started_at_ms DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, 1_000) as i64], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                started_at_ms: row.get(1)?,
                ended_at_ms: row.get(2)?,
                connection_count: row.get::<_, i64>(3)?.max(0) as usize,
                process_count: row.get::<_, i64>(4)?.max(0) as usize,
                country_count: row.get::<_, i64>(5)?.max(0) as usize,
                destination_count: row.get::<_, i64>(6)?.max(0) as usize,
                recording_count: row.get::<_, i64>(7)?.max(0) as usize,
                traffic: TrafficCounters {
                    bytes_sent: from_sql_u64(row.get(8)?),
                    bytes_received: from_sql_u64(row.get(9)?),
                    packets_sent: from_sql_u64(row.get(10)?),
                    packets_received: from_sql_u64(row.get(11)?),
                },
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), StorageError> {
        let json = serde_json::to_string(settings)?;
        self.connection.execute(
            "INSERT INTO settings(key, value_json) VALUES ('app', ?1)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            params![json],
        )?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<AppSettings, StorageError> {
        let json = self
            .connection
            .query_row(
                "SELECT value_json FROM settings WHERE key='app'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value))
            .transpose()
            .map(|settings| settings.unwrap_or_default())
            .map_err(Into::into)
    }

    pub fn save_baselines(&self, baselines: &BaselineBook) -> Result<(), StorageError> {
        let json = serde_json::to_string(baselines)?;
        self.connection.execute(
            "INSERT INTO settings(key, value_json) VALUES ('baselines', ?1)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            params![json],
        )?;
        Ok(())
    }

    pub fn load_baselines(&self) -> Result<BaselineBook, StorageError> {
        let json = self
            .connection
            .query_row(
                "SELECT value_json FROM settings WHERE key='baselines'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value))
            .transpose()
            .map(|baselines| baselines.unwrap_or_default())
            .map_err(Into::into)
    }

    pub fn list_focus_rules(&self) -> Result<FocusRules, StorageError> {
        let mut bookmark_statement = self.connection.prepare(
            "SELECT id, kind, value, created_at_ms FROM bookmarks ORDER BY created_at_ms DESC, id DESC",
        )?;
        let bookmark_rows = bookmark_statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut bookmarks = Vec::new();
        for row in bookmark_rows {
            let (id, kind, value, created_at_ms) = row?;
            bookmarks.push(Bookmark {
                id,
                kind: kind
                    .parse::<RuleKind>()
                    .map_err(|error| StorageError::InvalidRule(error.to_owned()))?,
                value,
                created_at_ms,
            });
        }

        let mut ignore_statement = self.connection.prepare(
            "SELECT id, kind, value, enabled, created_at_ms FROM ignore_rules ORDER BY created_at_ms DESC, id DESC",
        )?;
        let ignore_rows = ignore_statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        let mut ignore_rules = Vec::new();
        for row in ignore_rows {
            let (id, kind, value, enabled, created_at_ms) = row?;
            ignore_rules.push(IgnoreRule {
                id,
                kind: kind
                    .parse::<RuleKind>()
                    .map_err(|error| StorageError::InvalidRule(error.to_owned()))?,
                value,
                enabled: enabled != 0,
                created_at_ms,
            });
        }
        Ok(FocusRules {
            bookmarks,
            ignore_rules,
        })
    }

    pub fn add_bookmark(
        &self,
        kind: RuleKind,
        value: &str,
        created_at_ms: i64,
    ) -> Result<Bookmark, StorageError> {
        if kind == RuleKind::Localhost {
            return Err(StorageError::InvalidRule(
                "localhost is available only as an ignore rule".into(),
            ));
        }
        let value = normalize_rule_value(kind, value)?;
        self.connection.execute(
            "INSERT INTO bookmarks(kind, value, created_at_ms) VALUES (?1, ?2, ?3)
             ON CONFLICT(kind, value) DO NOTHING",
            params![kind.as_str(), value, created_at_ms],
        )?;
        self.connection
            .query_row(
                "SELECT id, created_at_ms FROM bookmarks WHERE kind = ?1 AND value = ?2",
                params![kind.as_str(), value],
                |row| {
                    Ok(Bookmark {
                        id: row.get(0)?,
                        kind,
                        value: value.clone(),
                        created_at_ms: row.get(1)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn remove_bookmark(&self, id: i64) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?
            > 0)
    }

    pub fn add_ignore_rule(
        &self,
        kind: RuleKind,
        value: &str,
        created_at_ms: i64,
    ) -> Result<IgnoreRule, StorageError> {
        let value = normalize_rule_value(kind, value)?;
        self.connection.execute(
            "INSERT INTO ignore_rules(kind, value, enabled, created_at_ms) VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(kind, value) DO UPDATE SET enabled = 1",
            params![kind.as_str(), value, created_at_ms],
        )?;
        self.connection
            .query_row(
                "SELECT id, enabled, created_at_ms FROM ignore_rules WHERE kind = ?1 AND value = ?2",
                params![kind.as_str(), value],
                |row| {
                    Ok(IgnoreRule {
                        id: row.get(0)?,
                        kind,
                        value: value.clone(),
                        enabled: row.get::<_, i64>(1)? != 0,
                        created_at_ms: row.get(2)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn set_ignore_rule_enabled(&self, id: i64, enabled: bool) -> Result<bool, StorageError> {
        Ok(self.connection.execute(
            "UPDATE ignore_rules SET enabled = ?2 WHERE id = ?1",
            params![id, i64::from(enabled)],
        )? > 0)
    }

    pub fn remove_ignore_rule(&self, id: i64) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM ignore_rules WHERE id = ?1", params![id])?
            > 0)
    }

    pub fn connection_history_stats(
        &self,
        remote_ip: &str,
    ) -> Result<ConnectionHistoryStats, StorageError> {
        let (first_ever_seen_ms, connections, sessions) = self.connection.query_row(
            "SELECT MIN(first_seen_ms), COUNT(*), COUNT(DISTINCT session_id)
             FROM connections WHERE remote_ip = ?1",
            params![remote_ip],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;
        Ok(ConnectionHistoryStats {
            first_ever_seen_ms,
            previous_connections: connections.saturating_sub(1) as usize,
            sessions: sessions.max(0) as usize,
        })
    }

    pub fn start_recording(
        &self,
        session_id: Option<&str>,
        state: &RecordingState,
    ) -> Result<i64, StorageError> {
        let path = state
            .output_path
            .as_deref()
            .ok_or_else(|| StorageError::InvalidRecording("recording path is missing".into()))?;
        let started_at_ms = state.started_at_ms.ok_or_else(|| {
            StorageError::InvalidRecording("recording start time is missing".into())
        })?;
        self.connection.execute(
            "INSERT INTO recordings(session_id, path, started_at_ms, bytes_written, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id,
                path,
                started_at_ms,
                to_sql_u64(state.bytes_written),
                recording_status_name(&state.status),
            ],
        )?;
        if let Some(session_id) = session_id {
            self.connection.execute(
                "UPDATE sessions SET recording_count = (
                   SELECT COUNT(*) FROM recordings WHERE session_id = ?1
                 ) WHERE id = ?1",
                params![session_id],
            )?;
        }
        Ok(self.connection.last_insert_rowid())
    }

    pub fn finish_recording(
        &self,
        id: i64,
        state: &RecordingState,
        ended_at_ms: i64,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.execute(
            "UPDATE recordings SET ended_at_ms = ?2, bytes_written = ?3, status = ?4 WHERE id = ?1",
            params![
                id,
                ended_at_ms,
                to_sql_u64(state.bytes_written),
                recording_status_name(&state.status),
            ],
        )? > 0)
    }

    pub fn list_recordings(
        &self,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordingSummary>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, session_id, path, started_at_ms, ended_at_ms, bytes_written, status
             FROM recordings WHERE (?1 IS NULL OR session_id = ?1)
             ORDER BY started_at_ms DESC LIMIT ?2",
        )?;
        let rows =
            statement.query_map(params![session_id, limit.clamp(1, 1_000) as i64], |row| {
                let status = recording_status_from_name(&row.get::<_, String>(6)?);
                Ok(RecordingSummary {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    path: row.get(2)?,
                    started_at_ms: row.get(3)?,
                    ended_at_ms: row.get(4)?,
                    bytes_written: from_sql_u64(row.get(5)?),
                    status,
                })
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn cleanup(&self, retention: RetentionPolicy, now_ms: i64) -> Result<usize, StorageError> {
        let Some(days) = retention.days() else {
            return Ok(0);
        };
        if days == 0 {
            let removed = self.connection.execute("DELETE FROM connections", [])?;
            self.connection.execute("DELETE FROM traffic_samples", [])?;
            self.connection.execute("DELETE FROM recordings", [])?;
            self.connection.execute("DELETE FROM sessions", [])?;
            return Ok(removed);
        }
        let cutoff = now_ms.saturating_sub(days.saturating_mul(86_400_000));
        let removed = self.connection.execute(
            "DELETE FROM connections WHERE last_seen_ms < ?1",
            params![cutoff],
        )?;
        self.connection.execute(
            "DELETE FROM sessions WHERE ended_at_ms IS NOT NULL AND ended_at_ms < ?1",
            params![cutoff],
        )?;
        self.connection.execute(
            "DELETE FROM recordings WHERE COALESCE(ended_at_ms, started_at_ms) < ?1",
            params![cutoff],
        )?;
        self.connection.execute(
            "DELETE FROM traffic_samples WHERE timestamp_ms < ?1",
            params![cutoff],
        )?;
        Ok(removed)
    }

    pub fn save_traffic_samples(&mut self, samples: &[TrafficSample]) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO traffic_samples(
                   session_id, timestamp_ms, process_key, source,
                   bytes_sent, bytes_received, packets_sent, packets_received,
                   down_bytes_per_second, up_bytes_per_second, packets_per_second
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(session_id, timestamp_ms, process_key, source) DO UPDATE SET
                   bytes_sent = excluded.bytes_sent,
                   bytes_received = excluded.bytes_received,
                   packets_sent = excluded.packets_sent,
                   packets_received = excluded.packets_received,
                   down_bytes_per_second = excluded.down_bytes_per_second,
                   up_bytes_per_second = excluded.up_bytes_per_second,
                   packets_per_second = excluded.packets_per_second",
            )?;
            for sample in samples {
                statement.execute(params![
                    sample.session_id,
                    sample.timestamp_ms,
                    sample.process_key.as_deref().unwrap_or(""),
                    sample.source.as_str(),
                    sample.counters.bytes_sent.min(i64::MAX as u64) as i64,
                    sample.counters.bytes_received.min(i64::MAX as u64) as i64,
                    sample.counters.packets_sent.min(i64::MAX as u64) as i64,
                    sample.counters.packets_received.min(i64::MAX as u64) as i64,
                    sample.rates.down_bytes_per_second,
                    sample.rates.up_bytes_per_second,
                    sample.rates.packets_per_second,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn query_timeline(
        &self,
        query: &TimelineQuery,
    ) -> Result<Vec<TrafficSample>, StorageError> {
        if query.to_ms < query.from_ms {
            return Err(StorageError::InvalidFilter(
                "timeline end precedes its start".into(),
            ));
        }
        let bucket = query.resolution.bucket_ms();
        let mut statement = self.connection.prepare(
            "SELECT (timestamp_ms / ?1) * ?1 AS bucket_ms,
                    session_id, NULLIF(process_key, ''), source,
                    SUM(bytes_sent), SUM(bytes_received),
                    SUM(packets_sent), SUM(packets_received),
                    AVG(down_bytes_per_second), AVG(up_bytes_per_second),
                    AVG(packets_per_second)
             FROM traffic_samples
             WHERE timestamp_ms BETWEEN ?2 AND ?3
               AND (?4 IS NULL OR session_id = ?4)
               AND (?5 IS NULL OR process_key = ?5)
               AND ((?5 IS NULL AND process_key = '') OR ?5 IS NOT NULL)
             GROUP BY bucket_ms, session_id, process_key, source
             ORDER BY bucket_ms ASC
             LIMIT 10000",
        )?;
        let rows = statement.query_map(
            params![
                bucket,
                query.from_ms,
                query.to_ms,
                query.session_id,
                query.process_key,
            ],
            |row| {
                let source: String = row.get(3)?;
                Ok(TrafficSample {
                    timestamp_ms: row.get(0)?,
                    session_id: row.get(1)?,
                    process_key: row.get(2)?,
                    source: match source.as_str() {
                        "interface_aggregate" => CounterSource::InterfaceAggregate,
                        "etw_flow" => CounterSource::EtwFlow,
                        "demo" => CounterSource::Demo,
                        _ => CounterSource::Unavailable,
                    },
                    counters: TrafficCounters {
                        bytes_sent: row.get::<_, i64>(4)?.max(0) as u64,
                        bytes_received: row.get::<_, i64>(5)?.max(0) as u64,
                        packets_sent: row.get::<_, i64>(6)?.max(0) as u64,
                        packets_received: row.get::<_, i64>(7)?.max(0) as u64,
                    },
                    rates: TransferRates {
                        down_bytes_per_second: row.get(8)?,
                        up_bytes_per_second: row.get(9)?,
                        packets_per_second: row.get(10)?,
                    },
                })
            },
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn export(&self, request: &ExportRequest) -> Result<ExportResult, StorageError> {
        let path = validate_export_path(&request.path, request.format)?;
        let query = HistoryQuery {
            from_ms: request.from_ms,
            to_ms: request.to_ms,
            session_id: request.session_id.clone(),
            query: request.query.clone(),
            limit: MAX_QUERY_LIMIT,
        };
        let mut entries = self.query_history(&query)?;
        if let Some(process_key) = &request.process_key {
            let process_key = process_key.to_lowercase();
            entries.retain(|entry| {
                entry.connection.process.name.to_lowercase() == process_key
                    || entry.connection.process.pid.is_some_and(|pid| {
                        format!("{}:{pid}", entry.connection.process.name).to_lowercase()
                            == process_key
                    })
            });
        }
        let data = match request.format {
            ExportFormat::Json => serde_json::to_vec_pretty(&entries)?,
            ExportFormat::Csv => export_csv(&entries).into_bytes(),
        };
        fs::write(&path, &data)?;
        Ok(ExportResult {
            path: path.to_string_lossy().into_owned(),
            records: entries.len(),
            bytes_written: data.len() as u64,
        })
    }

    pub fn database_size_bytes(&self) -> u64 {
        self.path
            .as_ref()
            .and_then(|path| fs::metadata(path).ok())
            .map_or(0, |metadata| metadata.len())
    }
}

fn migration_v1(transaction: &Transaction<'_>) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(
        "CREATE TABLE sessions (
           id TEXT PRIMARY KEY,
           started_at_ms INTEGER NOT NULL,
           ended_at_ms INTEGER,
           connection_count INTEGER NOT NULL DEFAULT 0,
           process_count INTEGER NOT NULL DEFAULT 0,
           country_count INTEGER NOT NULL DEFAULT 0,
           bytes_sent INTEGER NOT NULL DEFAULT 0,
           bytes_received INTEGER NOT NULL DEFAULT 0,
           packets_sent INTEGER NOT NULL DEFAULT 0,
           packets_received INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE connections (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
           first_seen_ms INTEGER NOT NULL,
           last_seen_ms INTEGER NOT NULL,
           process_name TEXT NOT NULL,
           pid INTEGER,
           remote_ip TEXT NOT NULL,
           remote_port INTEGER NOT NULL,
           local_ip TEXT NOT NULL,
           local_port INTEGER NOT NULL,
           protocol TEXT NOT NULL,
           direction TEXT NOT NULL,
           state TEXT NOT NULL,
           domain TEXT,
           country_code TEXT,
           asn INTEGER,
           active INTEGER NOT NULL,
           connection_json TEXT NOT NULL
         );
         CREATE INDEX idx_connections_session_time ON connections(session_id, last_seen_ms DESC);
         CREATE INDEX idx_connections_process_time ON connections(process_name, last_seen_ms DESC);
         CREATE INDEX idx_connections_remote_ip ON connections(remote_ip);
         CREATE INDEX idx_connections_domain ON connections(domain);
         CREATE INDEX idx_connections_country ON connections(country_code);
         CREATE TABLE settings (key TEXT PRIMARY KEY, value_json TEXT NOT NULL);
         CREATE TABLE bookmarks (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           kind TEXT NOT NULL,
           value TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           UNIQUE(kind, value)
         );
         CREATE TABLE ignore_rules (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           kind TEXT NOT NULL,
           value TEXT NOT NULL,
           enabled INTEGER NOT NULL DEFAULT 1,
           created_at_ms INTEGER NOT NULL,
           UNIQUE(kind, value)
         );",
    )
}

fn migration_v2(transaction: &Transaction<'_>) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(
        "ALTER TABLE sessions ADD COLUMN destination_count INTEGER NOT NULL DEFAULT 0;
         ALTER TABLE sessions ADD COLUMN recording_count INTEGER NOT NULL DEFAULT 0;
         CREATE TABLE recordings (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
           path TEXT NOT NULL,
           started_at_ms INTEGER NOT NULL,
           ended_at_ms INTEGER,
           bytes_written INTEGER NOT NULL DEFAULT 0,
           status TEXT NOT NULL
         );
         CREATE INDEX idx_recordings_session_time ON recordings(session_id, started_at_ms DESC);",
    )
}

fn migration_v3(transaction: &Transaction<'_>) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(
        "CREATE TABLE traffic_samples (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
           timestamp_ms INTEGER NOT NULL,
           process_key TEXT NOT NULL DEFAULT '',
           source TEXT NOT NULL,
           bytes_sent INTEGER NOT NULL DEFAULT 0,
           bytes_received INTEGER NOT NULL DEFAULT 0,
           packets_sent INTEGER NOT NULL DEFAULT 0,
           packets_received INTEGER NOT NULL DEFAULT 0,
           down_bytes_per_second REAL NOT NULL DEFAULT 0,
           up_bytes_per_second REAL NOT NULL DEFAULT 0,
           packets_per_second REAL NOT NULL DEFAULT 0,
           UNIQUE(session_id, timestamp_ms, process_key, source)
         );
         CREATE INDEX idx_traffic_samples_time ON traffic_samples(timestamp_ms);
         CREATE INDEX idx_traffic_samples_session_process
           ON traffic_samples(session_id, process_key, timestamp_ms);",
    )
}

fn validate_export_path(value: &str, format: ExportFormat) -> Result<PathBuf, StorageError> {
    if value.trim().is_empty() || value.as_bytes().contains(&0) {
        return Err(StorageError::InvalidExportPath(
            "path is empty or contains NUL".into(),
        ));
    }
    let path = PathBuf::from(value);
    let expected = match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Json => "json",
    };
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case(expected))
    {
        return Err(StorageError::InvalidExportPath(format!(
            "expected .{expected} extension"
        )));
    }
    let Some(parent) = path.parent() else {
        return Err(StorageError::InvalidExportPath("path has no parent".into()));
    };
    if !parent.exists() || !parent.is_dir() {
        return Err(StorageError::InvalidExportPath(
            "parent directory does not exist".into(),
        ));
    }
    Ok(path)
}

fn normalize_rule_value(kind: RuleKind, value: &str) -> Result<String, StorageError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 512 || trimmed.as_bytes().contains(&0) {
        return Err(StorageError::InvalidRule(
            "value must contain 1 to 512 characters and no NUL".into(),
        ));
    }
    match kind {
        RuleKind::Ip => {
            let address = trimmed
                .parse::<std::net::IpAddr>()
                .map_err(|_| StorageError::InvalidRule("IP value is invalid".into()))?;
            Ok(address.to_string())
        }
        RuleKind::Country => {
            if trimmed.len() != 2
                || !trimmed
                    .chars()
                    .all(|character| character.is_ascii_alphabetic())
            {
                return Err(StorageError::InvalidRule(
                    "country must be a two-letter ISO code".into(),
                ));
            }
            Ok(trimmed.to_ascii_uppercase())
        }
        RuleKind::Localhost => Ok("localhost".into()),
        RuleKind::Domain => {
            if trimmed.chars().any(char::is_whitespace) {
                return Err(StorageError::InvalidRule(
                    "domain cannot contain whitespace".into(),
                ));
            }
            Ok(trimmed.trim_end_matches('.').to_ascii_lowercase())
        }
        RuleKind::Process => Ok(trimmed.to_owned()),
    }
}

fn export_csv(entries: &[HistoryEntry]) -> String {
    let mut output = String::from(
        "session_id,first_seen,last_seen,process,pid,protocol,direction,local_endpoint,remote_endpoint,domain,country,asn,bytes_down,bytes_up,packets_down,packets_up,state\n",
    );
    for entry in entries {
        let connection = &entry.connection;
        let values = [
            entry.session_id.clone(),
            connection.first_seen_ms.to_string(),
            connection.last_seen_ms.to_string(),
            connection.process.name.clone(),
            connection
                .process
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_default(),
            connection.protocol.as_str().into(),
            format!("{:?}", connection.direction).to_lowercase(),
            connection.key.local.label(),
            connection.key.remote.label(),
            connection
                .domain
                .clone()
                .or_else(|| connection.hostname.clone())
                .unwrap_or_default(),
            connection
                .geo
                .as_ref()
                .map(|geo| geo.country_code.clone())
                .unwrap_or_default(),
            connection
                .asn
                .as_ref()
                .map(|asn| asn.number.to_string())
                .unwrap_or_default(),
            connection.traffic.bytes_received.to_string(),
            connection.traffic.bytes_sent.to_string(),
            connection.traffic.packets_received.to_string(),
            connection.traffic.packets_sent.to_string(),
            format!("{:?}", connection.state).to_lowercase(),
        ];
        output.push_str(&values.map(|value| csv_escape(&value)).join(","));
        output.push('\n');
    }
    output
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn to_sql_u64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

fn from_sql_u64(value: i64) -> u64 {
    value.max(0) as u64
}

fn recording_status_name(status: &RecordingStatus) -> &'static str {
    match status {
        RecordingStatus::Idle => "idle",
        RecordingStatus::Starting => "starting",
        RecordingStatus::Recording => "recording",
        RecordingStatus::Stopping => "stopping",
        RecordingStatus::Error => "error",
    }
}

fn recording_status_from_name(value: &str) -> RecordingStatus {
    match value {
        "starting" => RecordingStatus::Starting,
        "recording" => RecordingStatus::Recording,
        "stopping" => RecordingStatus::Stopping,
        "error" => RecordingStatus::Error,
        _ => RecordingStatus::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sabicheiro_core::{ConnectionState, Direction, NetworkProtocol, TimelineResolution};

    fn sample_connection() -> Connection {
        let mut connection = Connection::placeholder(1_000);
        connection.id = "connection-1".into();
        connection.process.name = "A, Browser".into();
        connection.process.pid = Some(7);
        connection.protocol = NetworkProtocol::Tcp;
        connection.key.protocol = NetworkProtocol::Tcp;
        connection.key.remote.address = "203.0.113.10".parse().expect("remote IP");
        connection.key.remote.port = 443;
        connection.direction = Direction::Outbound;
        connection.state = ConnectionState::Established;
        connection.active = true;
        connection.last_seen_ms = 2_000;
        connection
    }

    #[test]
    fn migrations_and_history_roundtrip() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        storage
            .save_connections("s1", &[sample_connection()])
            .expect("save");
        let entries = storage
            .query_history(&HistoryQuery::default())
            .expect("history");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].connection.process.name, "A, Browser");
        let overlapping = storage
            .query_history(&HistoryQuery {
                from_ms: Some(1_500),
                to_ms: Some(1_600),
                ..HistoryQuery::default()
            })
            .expect("overlapping history");
        assert_eq!(overlapping.len(), 1);
    }

    #[test]
    fn traffic_timeline_roundtrips_resolution_and_counter_provenance() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        storage
            .save_traffic_samples(&[
                TrafficSample {
                    timestamp_ms: 1_100,
                    session_id: Some("s1".into()),
                    process_key: None,
                    counters: TrafficCounters {
                        bytes_sent: 100,
                        bytes_received: 200,
                        packets_sent: 1,
                        packets_received: 2,
                    },
                    rates: TransferRates {
                        down_bytes_per_second: 200.0,
                        up_bytes_per_second: 100.0,
                        packets_per_second: 3.0,
                    },
                    source: CounterSource::InterfaceAggregate,
                },
                TrafficSample {
                    timestamp_ms: 1_600,
                    session_id: Some("s1".into()),
                    process_key: None,
                    counters: TrafficCounters {
                        bytes_sent: 300,
                        bytes_received: 400,
                        packets_sent: 3,
                        packets_received: 4,
                    },
                    rates: TransferRates {
                        down_bytes_per_second: 400.0,
                        up_bytes_per_second: 300.0,
                        packets_per_second: 7.0,
                    },
                    source: CounterSource::InterfaceAggregate,
                },
                TrafficSample {
                    timestamp_ms: 1_600,
                    session_id: Some("s1".into()),
                    process_key: Some("browser:7".into()),
                    counters: TrafficCounters {
                        bytes_sent: 25,
                        bytes_received: 50,
                        packets_sent: 1,
                        packets_received: 1,
                    },
                    rates: TransferRates {
                        down_bytes_per_second: 50.0,
                        up_bytes_per_second: 25.0,
                        packets_per_second: 2.0,
                    },
                    source: CounterSource::EtwFlow,
                },
            ])
            .expect("save samples");

        let global = storage
            .query_timeline(&TimelineQuery {
                from_ms: 1_000,
                to_ms: 2_000,
                session_id: Some("s1".into()),
                process_key: None,
                resolution: TimelineResolution::OneSecond,
            })
            .expect("global timeline");
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].timestamp_ms, 1_000);
        assert_eq!(global[0].counters.bytes_sent, 400);
        assert_eq!(global[0].counters.bytes_received, 600);
        assert_eq!(global[0].rates.down_bytes_per_second, 300.0);
        assert_eq!(global[0].source, CounterSource::InterfaceAggregate);

        let process = storage
            .query_timeline(&TimelineQuery {
                from_ms: 1_000,
                to_ms: 2_000,
                session_id: Some("s1".into()),
                process_key: Some("browser:7".into()),
                resolution: TimelineResolution::OneMinute,
            })
            .expect("process timeline");
        assert_eq!(process.len(), 1);
        assert_eq!(process[0].process_key.as_deref(), Some("browser:7"));
        assert_eq!(process[0].counters.bytes_received, 50);
        assert_eq!(process[0].source, CounterSource::EtwFlow);
    }

    #[test]
    fn version_one_database_migrates_recording_metadata_without_data_loss() {
        let mut connection = SqliteConnection::open_in_memory().expect("SQLite");
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (
                   version INTEGER PRIMARY KEY,
                   applied_at_ms INTEGER NOT NULL
                 );",
            )
            .expect("migration table");
        let transaction = connection.transaction().expect("transaction");
        migration_v1(&transaction).expect("version one schema");
        transaction.commit().expect("commit version one");
        connection
            .execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (1, 1000)",
                [],
            )
            .expect("version marker");
        connection
            .execute(
                "INSERT INTO sessions(id, started_at_ms, connection_count) VALUES ('legacy', 1000, 7)",
                [],
            )
            .expect("legacy session");

        let mut storage = Storage {
            connection,
            path: None,
        };
        storage.configure().expect("configure");
        storage.migrate().expect("migrate to latest");

        let sessions = storage.list_sessions(10).expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].connection_count, 7);
        assert_eq!(sessions[0].destination_count, 0);
        assert_eq!(sessions[0].recording_count, 0);
        let version = storage
            .connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("schema version");
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn retention_removes_expired_rows() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        storage
            .save_connections("s1", &[sample_connection()])
            .expect("save");
        assert_eq!(
            storage
                .cleanup(RetentionPolicy::OneDay, 100_000_000)
                .expect("cleanup"),
            1
        );
    }

    #[test]
    fn csv_export_quotes_values() {
        let directory = tempfile::tempdir().expect("tempdir");
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        storage
            .save_connections("s1", &[sample_connection()])
            .expect("save");
        let path = directory.path().join("export.csv");
        let result = storage
            .export(&ExportRequest {
                format: ExportFormat::Csv,
                path: path.to_string_lossy().into_owned(),
                session_id: None,
                process_key: None,
                from_ms: None,
                to_ms: None,
                query: None,
            })
            .expect("export");
        assert_eq!(result.records, 1);
        let csv = fs::read_to_string(path).expect("read export");
        assert!(csv.contains("\"A, Browser\""));
    }

    #[test]
    fn local_focus_rules_roundtrip_and_toggle() {
        let storage = Storage::in_memory().expect("storage");
        let bookmark = storage
            .add_bookmark(RuleKind::Country, "es", 1_000)
            .expect("bookmark");
        let ignore = storage
            .add_ignore_rule(RuleKind::Domain, "Example.COM.", 2_000)
            .expect("ignore rule");

        let rules = storage.list_focus_rules().expect("list rules");
        assert_eq!(rules.bookmarks[0].value, "ES");
        assert_eq!(rules.ignore_rules[0].value, "example.com");
        assert!(
            storage
                .set_ignore_rule_enabled(ignore.id, false)
                .expect("disable")
        );
        assert!(!storage.list_focus_rules().expect("list").ignore_rules[0].enabled);
        assert!(
            storage
                .remove_bookmark(bookmark.id)
                .expect("remove bookmark")
        );
        assert!(
            storage
                .remove_ignore_rule(ignore.id)
                .expect("remove ignore")
        );
    }

    #[test]
    fn destination_history_reports_prior_connections_and_sessions() {
        let mut storage = Storage::in_memory().expect("storage");
        let first = sample_connection();
        storage.start_session("s1", 1_000).expect("session 1");
        storage
            .save_connections("s1", std::slice::from_ref(&first))
            .expect("save first");
        let mut second = first;
        second.id = "connection-2".into();
        second.first_seen_ms = 3_000;
        second.last_seen_ms = 4_000;
        storage.start_session("s2", 3_000).expect("session 2");
        storage
            .save_connections("s2", &[second])
            .expect("save second");

        let stats = storage
            .connection_history_stats("203.0.113.10")
            .expect("history stats");
        assert_eq!(stats.first_ever_seen_ms, Some(1_000));
        assert_eq!(stats.previous_connections, 1);
        assert_eq!(stats.sessions, 2);
    }

    #[test]
    fn statistical_baselines_persist_locally() {
        use sabicheiro_core::{BaselineObservation, Endpoint};

        let storage = Storage::in_memory().expect("storage");
        let connection = sabicheiro_core::ConnectionKey {
            pid: None,
            local: Endpoint::new("127.0.0.1".parse().expect("local"), 50_000),
            remote: Endpoint::new("1.1.1.1".parse().expect("remote"), 443),
            protocol: NetworkProtocol::Tcp,
        };
        let mut baselines = BaselineBook::default();
        baselines.observe(BaselineObservation {
            process_key: "browser",
            destination_key: "one.example",
            connection: &connection,
            geo: None,
            timestamp_ms: 1_000,
        });
        storage.save_baselines(&baselines).expect("save baseline");
        let mut loaded = storage.load_baselines().expect("load baseline");
        let signals = loaded.observe(BaselineObservation {
            process_key: "browser",
            destination_key: "two.example",
            connection: &connection,
            geo: None,
            timestamp_ms: 2_000,
        });
        assert!(
            signals
                .iter()
                .any(|signal| signal.kind == sabicheiro_core::SignalKind::NewDestination)
        );
    }

    #[test]
    fn packet_recordings_are_indexed_with_their_session() {
        let storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        let recording = RecordingState {
            status: RecordingStatus::Recording,
            output_path: Some("capture.pcapng".into()),
            started_at_ms: Some(1_100),
            bytes_written: 0,
            max_file_size_mb: 64,
            duration_limit_seconds: Some(60),
            message: None,
        };
        let id = storage
            .start_recording(Some("s1"), &recording)
            .expect("start recording");
        let finished = RecordingState {
            status: RecordingStatus::Idle,
            bytes_written: 4_096,
            ..recording
        };
        assert!(
            storage
                .finish_recording(id, &finished, 2_000)
                .expect("finish recording")
        );
        let recordings = storage.list_recordings(Some("s1"), 10).expect("recordings");
        assert_eq!(recordings.len(), 1);
        assert_eq!(recordings[0].bytes_written, 4_096);
        assert_eq!(
            storage.list_sessions(10).expect("sessions")[0].recording_count,
            1
        );
    }
}
