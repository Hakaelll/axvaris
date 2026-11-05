mod external_dns;
use rusqlite::{Connection as SqliteConnection, OpenFlags, OptionalExtension, Transaction, params};
use sabicheiro_core::{
    AppSettings, BaselineBook, Bookmark, ComparisonRow, Connection, ConnectionAnnotation,
    ConnectionFilter, ConnectionHistoryStats, CounterSource, DerivedGeoEnrichment,
    EvidenceJournalStats, EvidenceObservation, EvidenceQuery, ExportFormat, ExportRedaction,
    ExportRequest, ExportResult, FocusRules, HandshakeModeAudit, HistoryCursor, HistoryEntry,
    HistoryQuery, IgnoreRule, InvestigationQuery, ListenerExposure, ListenerSnapshot,
    LocalNotificationRecord, LocalRule, LocalRuleAction, NetworkActionAudit, NetworkActionIntent,
    NetworkDestinationClass, NetworkProfile, NotificationState, RecordingState, RecordingStatus,
    RecordingSummary, RecoveryReport, RetentionBatchResult, RetentionEstimate, RetentionPolicy,
    RetentionTier, RuleKind, RuleMatch, SavedInvestigation, SessionComparison, SessionSummary,
    StorageHealth, TimelineQuery, TodayCardState, TodayChangeExample, TodayChangeGroup,
    TodayChangeKind, TodayEvidenceTarget, TrafficCounters, TrafficSample, TransferRates,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

const SCHEMA_VERSION: i64 = 8;
const MAX_QUERY_LIMIT: usize = 100_000;
const V7_TODAY_INDEXES_SQL: &str = "
CREATE INDEX IF NOT EXISTS idx_connections_process_remote
  ON connections(process_name, remote_ip, session_id);
CREATE INDEX IF NOT EXISTS idx_connections_executable_revision
  ON connections(process_name, executable_revision, session_id);
";
const V6_TRAFFIC_ROLLUPS_SQL: &str = "
CREATE INDEX IF NOT EXISTS idx_connections_history_cursor
  ON connections(last_seen_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_connections_session_cursor
  ON connections(session_id, last_seen_ms DESC, id DESC);
CREATE TABLE IF NOT EXISTS traffic_rollups (
  resolution_ms INTEGER NOT NULL,
  bucket_ms INTEGER NOT NULL,
  session_id TEXT NOT NULL DEFAULT '',
  process_key TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL,
  sample_interval_ms INTEGER NOT NULL,
  sample_count INTEGER NOT NULL,
  bytes_sent INTEGER NOT NULL,
  bytes_received INTEGER NOT NULL,
  packets_sent INTEGER NOT NULL,
  packets_received INTEGER NOT NULL,
  down_rate_sum REAL NOT NULL,
  up_rate_sum REAL NOT NULL,
  packet_rate_sum REAL NOT NULL,
  PRIMARY KEY(resolution_ms, bucket_ms, session_id, process_key, source)
);
CREATE INDEX IF NOT EXISTS idx_traffic_rollups_time
  ON traffic_rollups(resolution_ms, bucket_ms);
CREATE INDEX IF NOT EXISTS idx_traffic_rollups_session_process
  ON traffic_rollups(session_id, process_key, resolution_ms, bucket_ms);
";
const V5_PRODUCT_TABLES_SQL: &str = "
CREATE TABLE IF NOT EXISTS local_rules (
  id TEXT PRIMARY KEY,
  version INTEGER NOT NULL,
  enabled INTEGER NOT NULL,
  rule_json TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS local_rule_versions (
  id TEXT NOT NULL,
  version INTEGER NOT NULL,
  rule_json TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  PRIMARY KEY(id, version)
);
CREATE TABLE IF NOT EXISTS rule_matches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  rule_id TEXT NOT NULL,
  rule_version INTEGER NOT NULL,
  connection_id TEXT NOT NULL,
  matched_at_ms INTEGER NOT NULL,
  match_json TEXT NOT NULL,
  UNIQUE(rule_id, rule_version, connection_id, matched_at_ms)
);
CREATE INDEX IF NOT EXISTS idx_rule_matches_time ON rule_matches(matched_at_ms DESC);
CREATE TABLE IF NOT EXISTS connection_annotations (
  connection_id TEXT PRIMARY KEY,
  annotation_json TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS local_notifications (
  id TEXT PRIMARY KEY,
  rule_id TEXT NOT NULL,
  state TEXT NOT NULL,
  last_at_ms INTEGER NOT NULL,
  notification_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_local_notifications_state_time
  ON local_notifications(state, last_at_ms DESC);
CREATE TABLE IF NOT EXISTS network_profiles (
  id TEXT PRIMARY KEY,
  display_name TEXT,
  profile_json TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS today_cards (
  id TEXT PRIMARY KEY,
  dismissed INTEGER NOT NULL DEFAULT 0,
  pinned INTEGER NOT NULL DEFAULT 0,
  updated_at_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS listener_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
  observed_at_ms INTEGER NOT NULL,
  snapshot_sha256 TEXT NOT NULL,
  listeners_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_listener_snapshots_time
  ON listener_snapshots(observed_at_ms DESC);
CREATE TABLE IF NOT EXISTS handshake_mode_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  occurred_at_ms INTEGER NOT NULL,
  enabled INTEGER NOT NULL,
  maximum_bytes INTEGER NOT NULL,
  maximum_packets INTEGER NOT NULL,
  duration_limit_ms INTEGER NOT NULL,
  detail TEXT NOT NULL
);
";

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
    #[error("invalid saved investigation: {0}")]
    InvalidInvestigation(String),
    #[error("invalid evidence observation: {0}")]
    InvalidEvidence(String),
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct InvestigationWorkspace {
    tags: Vec<String>,
    selected_connection_id: Option<String>,
    layout_json: String,
    autosave_revision: u64,
}

pub struct Storage {
    connection: SqliteConnection,
    path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub next_cursor: Option<HistoryCursor>,
    pub has_more: bool,
}

#[derive(Debug, Default)]
struct SessionComparisonCounts {
    processes: BTreeMap<String, usize>,
    destinations: BTreeMap<String, usize>,
    countries: BTreeMap<String, usize>,
    ports: BTreeMap<String, usize>,
}

fn quote_filter(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn today_connection_example(
    kind: TodayChangeKind,
    category: &str,
    primary: String,
    secondary: Option<String>,
    connection: &Connection,
    query: Option<String>,
) -> TodayChangeExample {
    TodayChangeExample {
        id: format!("{category}:{}", connection.id),
        kind,
        primary,
        secondary,
        connection_id: Some(connection.id.clone()),
        query,
        target: TodayEvidenceTarget::Investigate,
        observed_at_ms: connection.last_seen_ms,
    }
}

fn comparison_counts_for_connections(connections: &[Connection]) -> SessionComparisonCounts {
    let mut counts = SessionComparisonCounts::default();
    for connection in connections {
        let weight = connection.repeat_count.max(1) as usize;
        *counts
            .processes
            .entry(connection.process.name.clone())
            .or_default() += weight;
        *counts
            .destinations
            .entry(
                connection
                    .domain
                    .clone()
                    .or_else(|| connection.hostname.clone())
                    .unwrap_or_else(|| connection.key.remote.address.to_string()),
            )
            .or_default() += weight;
        *counts
            .countries
            .entry(
                connection
                    .geo
                    .as_ref()
                    .and_then(|geo| geo.country_name.clone())
                    .unwrap_or_else(|| "Unknown geography".into()),
            )
            .or_default() += weight;
        *counts
            .ports
            .entry(connection.key.remote.port.to_string())
            .or_default() += weight;
    }
    counts
}

fn comparison_from_counts(
    left_session_id: &str,
    right_session_id: &str,
    left: &SessionComparisonCounts,
    right: &SessionComparisonCounts,
) -> SessionComparison {
    SessionComparison {
        left_session_id: left_session_id.to_owned(),
        right_session_id: right_session_id.to_owned(),
        processes: comparison_rows(&left.processes, &right.processes),
        destinations: comparison_rows(&left.destinations, &right.destinations),
        countries: comparison_rows(&left.countries, &right.countries),
        ports: comparison_rows(&left.ports, &right.ports),
    }
}

fn comparison_rows(
    left: &BTreeMap<String, usize>,
    right: &BTreeMap<String, usize>,
) -> Vec<ComparisonRow> {
    let keys = left
        .keys()
        .chain(right.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut rows = keys
        .into_iter()
        .map(|key| {
            let left_count = left.get(&key).copied().unwrap_or(0);
            let right_count = right.get(&key).copied().unwrap_or(0);
            ComparisonRow {
                key,
                left_count,
                right_count,
                delta: right_count as i64 - left_count as i64,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .delta
            .unsigned_abs()
            .cmp(&left.delta.unsigned_abs())
            .then_with(|| left.key.cmp(&right.key))
    });
    rows
}

impl Storage {
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let connection = SqliteConnection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.busy_timeout(std::time::Duration::from_millis(3000))?;
        Ok(Self {
            connection,
            path: Some(path.to_path_buf()),
        })
    }

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
        storage.ensure_v5_extensions()?;
        storage.ensure_v6_extensions()?;
        storage.ensure_v7_extensions()?;
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
        storage.ensure_v5_extensions()?;
        storage.ensure_v6_extensions()?;
        storage.ensure_v7_extensions()?;
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
        if (1..4).contains(&current) {
            self.back_up_before_version(current, 4)?;
        }
        if (1..5).contains(&current) {
            self.back_up_before_version(current, 5)?;
        }
        if (1..6).contains(&current) {
            self.back_up_before_version(current, 6)?;
        }
        if (1..7).contains(&current) {
            self.back_up_before_version(current, 7)?;
        }
        if (1..SCHEMA_VERSION).contains(&current) {
            self.back_up_before_version(current, SCHEMA_VERSION)?;
        }
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
                params![3, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        if current < 4 {
            let transaction = self.connection.transaction()?;
            migration_v4(&transaction)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![4, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        if current < 5 {
            let transaction = self.connection.transaction()?;
            migration_v5(&transaction)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![5, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        if current < 6 {
            let transaction = self.connection.transaction()?;
            migration_v6(&transaction)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![6, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        if current < 7 {
            let transaction = self.connection.transaction()?;
            migration_v7(&transaction)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![7, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        if current < 8 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(external_dns::EXTERNAL_DNS_SQL)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                params![8, chrono::Utc::now().timestamp_millis()],
            )?;
            transaction.commit()?;
        }
        Ok(())
    }

    fn ensure_v5_extensions(&self) -> Result<(), StorageError> {
        if !table_has_column(&self.connection, "investigations", "workspace_json")? {
            self.connection.execute_batch(
                "ALTER TABLE investigations
                   ADD COLUMN workspace_json TEXT NOT NULL DEFAULT '{}';",
            )?;
        }
        self.connection.execute_batch(V5_PRODUCT_TABLES_SQL)?;
        Ok(())
    }

    fn ensure_v6_extensions(&self) -> Result<(), StorageError> {
        if !table_has_column(&self.connection, "traffic_samples", "sample_interval_ms")? {
            self.connection.execute_batch(
                "ALTER TABLE traffic_samples
                   ADD COLUMN sample_interval_ms INTEGER NOT NULL DEFAULT 1000;",
            )?;
        }
        self.connection.execute_batch(V6_TRAFFIC_ROLLUPS_SQL)?;
        Ok(())
    }

    fn ensure_v7_extensions(&self) -> Result<(), StorageError> {
        if !table_has_column(&self.connection, "connections", "executable_revision")? {
            self.connection
                .execute_batch("ALTER TABLE connections ADD COLUMN executable_revision TEXT;")?;
        }
        self.connection.execute_batch(V7_TODAY_INDEXES_SQL)?;
        Ok(())
    }

    fn back_up_before_version(&self, current: i64, target: i64) -> Result<(), StorageError> {
        if current == 0 {
            return Ok(());
        }
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(FULL);")?;
        let backup = path.with_extension(format!("pre-v{target}-from-v{current}.bak"));
        self.connection.backup(rusqlite::MAIN_DB, backup, None)?;
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
                   state, domain, country_code, asn, active, remote_scope,
                   endpoint_role, role_reason, observation_source, correlation_quality,
                   process_instance, interface_id, capture_generation, flow_generation,
                   city_generation, asn_generation, executable_revision, connection_json
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26,
                   ?27, ?28, ?29, ?30
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
                   remote_scope=excluded.remote_scope,
                   endpoint_role=excluded.endpoint_role,
                   role_reason=excluded.role_reason,
                   observation_source=excluded.observation_source,
                   correlation_quality=excluded.correlation_quality,
                   process_instance=excluded.process_instance,
                   interface_id=excluded.interface_id,
                   capture_generation=excluded.capture_generation,
                   flow_generation=excluded.flow_generation,
                   city_generation=excluded.city_generation,
                   asn_generation=excluded.asn_generation,
                   executable_revision=excluded.executable_revision,
                   connection_json=excluded.connection_json",
            )?;
            for connection in connections {
                let json = serde_json::to_string(connection)?;
                let remote_scope = serialized_name(&connection.key.remote.scope)?;
                let endpoint_role = serialized_name(&connection.evidence.endpoint_role)?;
                let observation_source = serialized_name(&connection.evidence.observation_source)?;
                let correlation_quality =
                    serialized_name(&connection.evidence.correlation_quality)?;
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
                    connection
                        .geo
                        .as_ref()
                        .and_then(|geo| geo.country_code.as_deref()),
                    connection.asn.as_ref().map(|asn| i64::from(asn.number)),
                    i64::from(connection.active),
                    remote_scope,
                    endpoint_role,
                    connection.evidence.role_reason,
                    observation_source,
                    correlation_quality,
                    connection.key.process_instance,
                    connection.key.interface_id,
                    to_sql_u64(connection.key.capture_generation),
                    to_sql_u64(connection.key.flow_generation),
                    connection
                        .geo
                        .as_ref()
                        .map_or(0, |geo| to_sql_u64(geo.city_generation)),
                    connection
                        .asn
                        .as_ref()
                        .map_or(0, |asn| to_sql_u64(asn.generation)),
                    connection.process.executable_revision_id,
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

    pub fn append_observations(
        &mut self,
        session_id: Option<&str>,
        observations: &[EvidenceObservation],
    ) -> Result<usize, StorageError> {
        if observations.is_empty() {
            return Ok(0);
        }
        let transaction = self.connection.transaction()?;
        let mut inserted = 0_usize;
        {
            let mut existing_statement = transaction.prepare_cached(
                "SELECT id FROM evidence_journal
                 WHERE provider_id=?1 AND provider_generation=?2 AND provider_sequence=?3",
            )?;
            let mut insert_statement = transaction.prepare_cached(
                "INSERT OR IGNORE INTO evidence_journal(
                   id, session_id, schema_version, provider_id, provider_generation,
                   provider_sequence, monotonic_ns, utc_ms, kind, process_instance,
                   remote_ip, domain, asn, route_id, listener_scope, network_profile_id,
                   observation_json, created_at_ms
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, ?17, ?18
                 )",
            )?;
            for observation in observations {
                observation
                    .validate()
                    .map_err(|error| StorageError::InvalidEvidence(error.to_string()))?;
                let existing = existing_statement
                    .query_row(
                        params![
                            observation.cursor.provider_id,
                            to_sql_u64(observation.cursor.generation),
                            to_sql_u64(observation.cursor.sequence),
                        ],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?;
                if existing.as_deref().is_some_and(|id| id != observation.id) {
                    return Err(StorageError::InvalidEvidence(format!(
                        "provider cursor {}:{}:{} already belongs to another observation",
                        observation.cursor.provider_id,
                        observation.cursor.generation,
                        observation.cursor.sequence
                    )));
                }
                let network = observation.network.as_ref();
                let process_instance =
                    network.and_then(|event| event.key.process_instance.as_deref());
                let remote_ip = network.map(|event| event.key.remote.address.to_string());
                let domain =
                    network.and_then(|event| event.domain.as_deref().or(event.hostname.as_deref()));
                let asn = network
                    .and_then(|event| event.asn.as_ref())
                    .map(|asn| i64::from(asn.number));
                let route_id = observation
                    .payload
                    .get("route_id")
                    .and_then(serde_json::Value::as_str);
                let listener_scope = observation
                    .payload
                    .get("listener_scope")
                    .and_then(serde_json::Value::as_str);
                let network_profile_id = observation
                    .payload
                    .get("network_profile_id")
                    .and_then(serde_json::Value::as_str);
                inserted = inserted.saturating_add(insert_statement.execute(params![
                    observation.id,
                    session_id,
                    i64::from(observation.schema_version),
                    observation.cursor.provider_id,
                    to_sql_u64(observation.cursor.generation),
                    to_sql_u64(observation.cursor.sequence),
                    to_sql_u64(observation.clock.monotonic_ns),
                    observation.clock.utc_ms,
                    serialized_name(&observation.kind)?,
                    process_instance,
                    remote_ip,
                    domain,
                    asn,
                    route_id,
                    listener_scope,
                    network_profile_id,
                    serde_json::to_string(observation)?,
                    chrono::Utc::now().timestamp_millis(),
                ])?);
            }
        }
        transaction.commit()?;
        Ok(inserted)
    }

    pub fn query_observations(
        &self,
        query: &EvidenceQuery,
    ) -> Result<Vec<EvidenceObservation>, StorageError> {
        let kind = query.kind.map(|kind| serialized_name(&kind)).transpose()?;
        let after_monotonic = query.after_monotonic_ns.map(to_sql_u64);
        let mut statement = self.connection.prepare(
            "SELECT observation_json FROM evidence_journal
             WHERE (?1 IS NULL OR utc_ms >= ?1)
               AND (?2 IS NULL OR utc_ms <= ?2)
               AND (?3 IS NULL OR session_id = ?3)
               AND (?4 IS NULL OR provider_id = ?4)
               AND (?5 IS NULL OR process_instance = ?5)
               AND (?6 IS NULL OR remote_ip = ?6)
               AND (?7 IS NULL OR kind = ?7)
               AND (?8 IS NULL OR monotonic_ns > ?8)
             ORDER BY monotonic_ns, provider_id, provider_generation, provider_sequence, id
             LIMIT ?9",
        )?;
        let rows = statement.query_map(
            params![
                query.from_utc_ms,
                query.to_utc_ms,
                query.session_id,
                query.provider_id,
                query.process_instance,
                query.remote_ip,
                kind,
                after_monotonic,
                query.bounded_limit() as i64,
            ],
            |row| row.get::<_, String>(0),
        )?;
        let mut observations = Vec::new();
        for row in rows {
            let observation: EvidenceObservation = serde_json::from_str(&row?)?;
            observation
                .validate()
                .map_err(|error| StorageError::InvalidEvidence(error.to_string()))?;
            observations.push(observation);
        }
        Ok(observations)
    }

    /// Loads a complete, deterministically ordered journal slice for an explicit
    /// reconstruction. Interactive queries remain bounded through `EvidenceQuery`.
    pub fn load_observations_for_replay(
        &self,
        session_id: Option<&str>,
    ) -> Result<Vec<EvidenceObservation>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT observation_json FROM evidence_journal
             WHERE (?1 IS NULL OR session_id = ?1)
             ORDER BY monotonic_ns, provider_id, provider_generation, provider_sequence, id",
        )?;
        let rows = statement.query_map(params![session_id], |row| row.get::<_, String>(0))?;
        let mut observations = Vec::new();
        for row in rows {
            let observation: EvidenceObservation = serde_json::from_str(&row?)?;
            observation
                .validate()
                .map_err(|error| StorageError::InvalidEvidence(error.to_string()))?;
            observations.push(observation);
        }
        Ok(observations)
    }

    pub fn evidence_stats(&self) -> Result<EvidenceJournalStats, StorageError> {
        let (observations, first_utc_ms, last_utc_ms, providers, gaps) =
            self.connection.query_row(
                "SELECT COUNT(*), MIN(utc_ms), MAX(utc_ms), COUNT(DISTINCT provider_id),
                    COALESCE(SUM(CASE WHEN kind='provider_gap' THEN 1 ELSE 0 END), 0)
             FROM evidence_journal",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )?;
        Ok(EvidenceJournalStats {
            observations: observations.max(0) as u64,
            first_utc_ms,
            last_utc_ms,
            providers: providers.max(0) as usize,
            gaps: gaps.max(0) as u64,
            bytes_on_disk: self.database_size_bytes(),
        })
    }

    pub fn retention_estimate(
        &self,
        cutoff_utc_ms: i64,
    ) -> Result<RetentionEstimate, StorageError> {
        let detailed = count_before(
            &self.connection,
            "evidence_journal",
            "utc_ms",
            cutoff_utc_ms,
        )?;
        let connections = count_before(
            &self.connection,
            "connections",
            "last_seen_ms",
            cutoff_utc_ms,
        )?;
        let raw_samples = count_before(
            &self.connection,
            "traffic_samples",
            "timestamp_ms",
            cutoff_utc_ms,
        )?;
        let compacted_samples = count_before(
            &self.connection,
            "traffic_rollups",
            "bucket_ms",
            cutoff_utc_ms,
        )?;
        let samples = raw_samples.saturating_add(compacted_samples);
        let rollups = self.connection.query_row(
            "SELECT COUNT(*) FROM evidence_rollups WHERE day_utc < ?1",
            params![cutoff_utc_ms.div_euclid(86_400_000)],
            |row| row.get::<_, i64>(0),
        )?;
        let total_rows = self.connection.query_row(
            "SELECT
               (SELECT COUNT(*) FROM evidence_journal) +
               (SELECT COUNT(*) FROM connections) +
               (SELECT COUNT(*) FROM traffic_samples) +
               (SELECT COUNT(*) FROM traffic_rollups) +
               (SELECT COUNT(*) FROM evidence_rollups)",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        let eligible = detailed
            .saturating_add(connections)
            .saturating_add(samples)
            .saturating_add(rollups);
        let estimated_bytes = if total_rows <= 0 {
            0
        } else {
            self.database_size_bytes()
                .saturating_mul(eligible.max(0) as u64)
                / total_rows as u64
        };
        Ok(RetentionEstimate {
            cutoff_utc_ms,
            rows_by_tier: [
                (RetentionTier::DetailedEvidence, detailed.max(0) as u64),
                (RetentionTier::Connections, connections.max(0) as u64),
                (RetentionTier::TrafficSamples, samples.max(0) as u64),
                (RetentionTier::DailyRollups, rollups.max(0) as u64),
            ]
            .into_iter()
            .collect(),
            estimated_bytes,
        })
    }

    pub fn apply_retention_batch(
        &mut self,
        tier: RetentionTier,
        cutoff_utc_ms: i64,
        batch_size: usize,
        cancelled: bool,
    ) -> Result<RetentionBatchResult, StorageError> {
        if cancelled {
            return Ok(RetentionBatchResult {
                removed_by_tier: Default::default(),
                remaining_eligible_rows: 0,
                cancelled: true,
                compact_recommended: false,
            });
        }
        let limit = batch_size.clamp(1, 10_000) as i64;
        let transaction = self.connection.transaction()?;
        if tier == RetentionTier::TrafficSamples {
            let removed_raw = transaction.execute(
                "DELETE FROM traffic_samples WHERE rowid IN (
                   SELECT rowid FROM traffic_samples WHERE timestamp_ms < ?1
                   ORDER BY timestamp_ms LIMIT ?2
                 )",
                params![cutoff_utc_ms, limit],
            )?;
            let remaining_limit = limit.saturating_sub(removed_raw as i64);
            let removed_rollups = if remaining_limit > 0 {
                transaction.execute(
                    "DELETE FROM traffic_rollups WHERE rowid IN (
                       SELECT rowid FROM traffic_rollups WHERE bucket_ms < ?1
                       ORDER BY bucket_ms LIMIT ?2
                     )",
                    params![cutoff_utc_ms, remaining_limit],
                )?
            } else {
                0
            };
            transaction.commit()?;
            let remaining = count_before(
                &self.connection,
                "traffic_samples",
                "timestamp_ms",
                cutoff_utc_ms,
            )?
            .saturating_add(count_before(
                &self.connection,
                "traffic_rollups",
                "bucket_ms",
                cutoff_utc_ms,
            )?);
            let removed = removed_raw.saturating_add(removed_rollups);
            return Ok(RetentionBatchResult {
                removed_by_tier: [(tier, removed as u64)].into_iter().collect(),
                remaining_eligible_rows: remaining.max(0) as u64,
                cancelled: false,
                compact_recommended: removed > 0 && remaining == 0,
            });
        }
        if tier == RetentionTier::Connections {
            transaction.execute(
                "INSERT INTO evidence_rollups(
                   day_utc, process_key, remote_ip, connections, first_seen_ms, last_seen_ms
                 )
                 SELECT last_seen_ms / 86400000,
                        COALESCE(process_instance, process_name), remote_ip,
                        COUNT(*), MIN(first_seen_ms), MAX(last_seen_ms)
                 FROM connections
                 WHERE id IN (
                   SELECT id FROM connections WHERE last_seen_ms < ?1
                   ORDER BY last_seen_ms LIMIT ?2
                 )
                 GROUP BY last_seen_ms / 86400000,
                          COALESCE(process_instance, process_name), remote_ip
                 ON CONFLICT(day_utc, process_key, remote_ip) DO UPDATE SET
                   connections=evidence_rollups.connections + excluded.connections,
                   first_seen_ms=MIN(evidence_rollups.first_seen_ms, excluded.first_seen_ms),
                   last_seen_ms=MAX(evidence_rollups.last_seen_ms, excluded.last_seen_ms)",
                params![cutoff_utc_ms, limit],
            )?;
        }
        let (table, time_column) = match tier {
            RetentionTier::DetailedEvidence => ("evidence_journal", "utc_ms"),
            RetentionTier::Connections => ("connections", "last_seen_ms"),
            RetentionTier::TrafficSamples => ("traffic_samples", "timestamp_ms"),
            RetentionTier::DailyRollups => ("evidence_rollups", "day_utc"),
        };
        let cutoff = if tier == RetentionTier::DailyRollups {
            cutoff_utc_ms.div_euclid(86_400_000)
        } else {
            cutoff_utc_ms
        };
        let delete_sql = format!(
            "DELETE FROM {table} WHERE rowid IN (
               SELECT rowid FROM {table} WHERE {time_column} < ?1
               ORDER BY {time_column} LIMIT ?2
             )"
        );
        let removed = transaction.execute(&delete_sql, params![cutoff, limit])?;
        if tier == RetentionTier::Connections {
            transaction.execute(
                "UPDATE sessions SET connection_count = (
                   SELECT COUNT(*) FROM connections WHERE connections.session_id=sessions.id
                 )",
                [],
            )?;
        }
        transaction.commit()?;
        let remaining = count_before(&self.connection, table, time_column, cutoff)?;
        Ok(RetentionBatchResult {
            removed_by_tier: [(tier, removed as u64)].into_iter().collect(),
            remaining_eligible_rows: remaining.max(0) as u64,
            cancelled: false,
            compact_recommended: removed > 0 && remaining == 0,
        })
    }

    pub fn index_search_document(
        &mut self,
        reference_id: &str,
        kind: &str,
        name: &str,
        notes: &str,
        tags: &str,
        detail: &str,
    ) -> Result<(), StorageError> {
        if reference_id.trim().is_empty() || kind.trim().is_empty() {
            return Err(StorageError::InvalidEvidence(
                "search documents require a reference and kind".into(),
            ));
        }
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM search_fts WHERE reference_id=?1",
            params![reference_id],
        )?;
        transaction.execute(
            "INSERT INTO search_fts(reference_id, kind, name, notes, tags, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![reference_id, kind, name, notes, tags, detail],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn search(&self, text: &str, limit: usize) -> Result<Vec<String>, StorageError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let phrase = format!("\"{}\"", text.replace('"', "\"\""));
        let mut statement = self.connection.prepare(
            "SELECT reference_id FROM search_fts
             WHERE search_fts MATCH ?1
             ORDER BY rank LIMIT ?2",
        )?;
        let rows = statement.query_map(params![phrase, limit.clamp(1, 1_000) as i64], |row| {
            row.get::<_, String>(0)
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_network_action(
        &self,
        occurred_at_ms: i64,
        intent: NetworkActionIntent,
        destination_class: NetworkDestinationClass,
        consent_id: Option<&str>,
        allowed: bool,
        bytes_sent: u64,
        bytes_received: u64,
        detail: &str,
    ) -> Result<i64, StorageError> {
        if detail.len() > 4_096 || consent_id.is_some_and(|value| value.len() > 128) {
            return Err(StorageError::InvalidEvidence(
                "network action audit fields exceed their local limits".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO network_action_audit(
               occurred_at_ms, intent, destination_class, consent_id, allowed,
               bytes_sent, bytes_received, detail
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                occurred_at_ms,
                serialized_name(&intent)?,
                serialized_name(&destination_class)?,
                consent_id,
                i64::from(allowed),
                to_sql_u64(bytes_sent),
                to_sql_u64(bytes_received),
                detail,
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn list_network_actions(
        &self,
        limit: usize,
    ) -> Result<Vec<NetworkActionAudit>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, occurred_at_ms, intent, destination_class, consent_id,
                    allowed, bytes_sent, bytes_received, detail
             FROM network_action_audit ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, 10_000) as i64], |row| {
            let intent: String = row.get(2)?;
            let destination: String = row.get(3)?;
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                intent,
                destination,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)? != 0,
                row.get::<_, i64>(6)?.max(0) as u64,
                row.get::<_, i64>(7)?.max(0) as u64,
                row.get::<_, String>(8)?,
            ))
        })?;
        rows.map(|row| {
            let (
                id,
                occurred_at_ms,
                intent,
                destination,
                consent_id,
                allowed,
                bytes_sent,
                bytes_received,
                detail,
            ) = row?;
            Ok(NetworkActionAudit {
                id,
                occurred_at_ms,
                intent: serde_json::from_str(&format!("\"{intent}\""))?,
                destination_class: serde_json::from_str(&format!("\"{destination}\""))?,
                consent_id,
                allowed,
                bytes_sent,
                bytes_received,
                detail,
            })
        })
        .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_projection_run(
        &self,
        projection_name: &str,
        projection_version: u32,
        first_observation_id: Option<&str>,
        last_observation_id: Option<&str>,
        observation_count: usize,
        result_sha256: &str,
        created_at_ms: i64,
    ) -> Result<(), StorageError> {
        if projection_name.trim().is_empty()
            || result_sha256.len() != 64
            || !result_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(StorageError::InvalidEvidence(
                "projection identity or SHA-256 is invalid".into(),
            ));
        }
        self.connection.execute(
            "INSERT OR IGNORE INTO projection_runs(
               projection_name, projection_version, first_observation_id,
               last_observation_id, observation_count, result_sha256, created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                projection_name,
                i64::from(projection_version),
                first_observation_id,
                last_observation_id,
                observation_count as i64,
                result_sha256.to_ascii_lowercase(),
                created_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn query_history(&self, query: &HistoryQuery) -> Result<Vec<HistoryEntry>, StorageError> {
        self.query_history_page(query).map(|page| page.entries)
    }

    /// Classifies the current connection set against every retained connection row.
    /// Only the compact candidate keys cross into SQLite; historical connection JSON is
    /// never materialized in Rust or sent over IPC.
    pub fn today_novelty(
        &self,
        current_session_id: Option<&str>,
        connections: &[Connection],
        example_limit: usize,
    ) -> Result<Vec<TodayChangeGroup>, StorageError> {
        let example_limit = example_limit.clamp(1, 10);
        let mut processes = BTreeMap::<String, &Connection>::new();
        let mut destinations = BTreeMap::<(String, String), &Connection>::new();
        let mut domains = BTreeMap::<String, &Connection>::new();
        let mut asns = BTreeMap::<String, &Connection>::new();
        let mut countries = BTreeMap::<String, &Connection>::new();
        let mut revisions = BTreeMap::<(String, String), &Connection>::new();

        for connection in connections {
            let process = connection.process.name.trim();
            if !process.is_empty() {
                processes.entry(process.to_owned()).or_insert(connection);
                destinations
                    .entry((
                        process.to_owned(),
                        connection.key.remote.address.to_string(),
                    ))
                    .or_insert(connection);
            }
            if let Some(domain) = connection
                .domain
                .as_deref()
                .or(connection.hostname.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                domains
                    .entry(domain.to_ascii_lowercase())
                    .or_insert(connection);
            }
            if let Some(asn) = connection.asn.as_ref().filter(|asn| asn.number > 0) {
                asns.entry(asn.number.to_string()).or_insert(connection);
            }
            if let Some(country) = connection
                .geo
                .as_ref()
                .and_then(|geo| geo.country_code.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                countries
                    .entry(country.to_ascii_uppercase())
                    .or_insert(connection);
            }
            if let Some(revision) = connection
                .process
                .executable_revision_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                revisions
                    .entry((process.to_owned(), revision.to_owned()))
                    .or_insert(connection);
            }
        }

        let unseen_processes = self.unseen_scalar_candidates(
            processes.keys().cloned().collect(),
            current_session_id,
            "process",
        )?;
        let unseen_destinations = self.unseen_pair_candidates(
            destinations.keys().cloned().collect(),
            current_session_id,
            "destination",
        )?;
        let unseen_domains = self.unseen_scalar_candidates(
            domains.keys().cloned().collect(),
            current_session_id,
            "domain",
        )?;
        let unseen_asns = self.unseen_scalar_candidates(
            asns.keys().cloned().collect(),
            current_session_id,
            "asn",
        )?;
        let unseen_countries = self.unseen_scalar_candidates(
            countries.keys().cloned().collect(),
            current_session_id,
            "country",
        )?;
        let unseen_revisions = self.unseen_pair_candidates(
            revisions.keys().cloned().collect(),
            current_session_id,
            "revision",
        )?;

        let process_examples = unseen_processes
            .iter()
            .take(example_limit)
            .filter_map(|value| {
                processes.get(value).map(|connection| {
                    today_connection_example(
                        TodayChangeKind::Process,
                        "process",
                        value.clone(),
                        None,
                        connection,
                        Some(format!("process:{}", quote_filter(value))),
                    )
                })
            })
            .collect();
        let destination_examples = unseen_destinations
            .iter()
            .take(example_limit)
            .filter_map(|key| {
                destinations.get(key).map(|connection| {
                    let display = connection
                        .domain
                        .clone()
                        .or_else(|| connection.hostname.clone())
                        .unwrap_or_else(|| key.1.clone());
                    let destination_filter =
                        if connection.domain.is_some() || connection.hostname.is_some() {
                            format!("domain:{}", quote_filter(&display))
                        } else {
                            format!("ip:{}", key.1)
                        };
                    today_connection_example(
                        TodayChangeKind::ApplicationDestination,
                        "destination",
                        display,
                        Some(key.0.clone()),
                        connection,
                        Some(format!(
                            "process:{} AND {destination_filter}",
                            quote_filter(&key.0)
                        )),
                    )
                })
            })
            .collect();
        let domain_examples = unseen_domains
            .iter()
            .take(example_limit)
            .filter_map(|value| {
                domains.get(value).map(|connection| {
                    today_connection_example(
                        TodayChangeKind::Domain,
                        "domain",
                        value.clone(),
                        Some(connection.process.name.clone()),
                        connection,
                        Some(format!("domain:{}", quote_filter(value))),
                    )
                })
            })
            .collect();
        let asn_examples = unseen_asns
            .iter()
            .take(example_limit)
            .filter_map(|value| {
                asns.get(value).map(|connection| {
                    today_connection_example(
                        TodayChangeKind::Asn,
                        "asn",
                        format!("AS{value}"),
                        connection
                            .asn
                            .as_ref()
                            .and_then(|asn| asn.organization.clone()),
                        connection,
                        Some(format!("asn:{value}")),
                    )
                })
            })
            .collect();
        let country_examples = unseen_countries
            .iter()
            .take(example_limit)
            .filter_map(|value| {
                countries.get(value).map(|connection| {
                    today_connection_example(
                        TodayChangeKind::Country,
                        "country",
                        connection
                            .geo
                            .as_ref()
                            .and_then(|geo| geo.country_name.clone())
                            .unwrap_or_else(|| value.clone()),
                        Some(value.clone()),
                        connection,
                        Some(format!("country:{value}")),
                    )
                })
            })
            .collect();
        let revision_examples = unseen_revisions
            .iter()
            .take(example_limit)
            .filter_map(|key| {
                revisions.get(key).map(|connection| {
                    today_connection_example(
                        TodayChangeKind::ExecutableRevision,
                        "revision",
                        key.1.clone(),
                        Some(key.0.clone()),
                        connection,
                        Some(format!("process:{}", quote_filter(&key.0))),
                    )
                })
            })
            .collect();

        Ok(vec![
            TodayChangeGroup {
                kind: TodayChangeKind::Process,
                count: unseen_processes.len(),
                examples: process_examples,
            },
            TodayChangeGroup {
                kind: TodayChangeKind::ApplicationDestination,
                count: unseen_destinations.len(),
                examples: destination_examples,
            },
            TodayChangeGroup {
                kind: TodayChangeKind::Domain,
                count: unseen_domains.len(),
                examples: domain_examples,
            },
            TodayChangeGroup {
                kind: TodayChangeKind::Asn,
                count: unseen_asns.len(),
                examples: asn_examples,
            },
            TodayChangeGroup {
                kind: TodayChangeKind::Country,
                count: unseen_countries.len(),
                examples: country_examples,
            },
            TodayChangeGroup {
                kind: TodayChangeKind::ExecutableRevision,
                count: unseen_revisions.len(),
                examples: revision_examples,
            },
        ])
    }

    fn unseen_scalar_candidates(
        &self,
        candidates: Vec<String>,
        current_session_id: Option<&str>,
        category: &str,
    ) -> Result<Vec<String>, StorageError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let historical_expression = match category {
            "process" => "history.process_name",
            "domain" => "LOWER(history.domain)",
            "asn" => "CAST(history.asn AS TEXT)",
            "country" => "UPPER(history.country_code)",
            _ => {
                return Err(StorageError::InvalidEvidence(
                    "unknown novelty category".into(),
                ));
            }
        };
        let candidates_json = serde_json::to_string(&candidates)?;
        let sql = format!(
            "WITH candidates(value) AS (
               SELECT DISTINCT CAST(value AS TEXT) FROM json_each(?1)
             )
             SELECT candidate.value FROM candidates candidate
             WHERE NOT EXISTS (
               SELECT 1 FROM connections history
               WHERE (?2 IS NULL OR history.session_id <> ?2)
                 AND {historical_expression} = candidate.value
             )
             ORDER BY candidate.value"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params![candidates_json, current_session_id], |row| {
            row.get::<_, String>(0)
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn unseen_pair_candidates(
        &self,
        candidates: Vec<(String, String)>,
        current_session_id: Option<&str>,
        category: &str,
    ) -> Result<Vec<(String, String)>, StorageError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let second_expression = match category {
            "destination" => "history.remote_ip",
            "revision" => "history.executable_revision",
            _ => {
                return Err(StorageError::InvalidEvidence(
                    "unknown novelty category".into(),
                ));
            }
        };
        let candidates_json = serde_json::to_string(&candidates)?;
        let sql = format!(
            "WITH candidates(first_value, second_value) AS (
               SELECT DISTINCT
                 CAST(json_extract(value, '$[0]') AS TEXT),
                 CAST(json_extract(value, '$[1]') AS TEXT)
               FROM json_each(?1)
             )
             SELECT candidate.first_value, candidate.second_value
             FROM candidates candidate
             WHERE NOT EXISTS (
               SELECT 1 FROM connections history
               WHERE (?2 IS NULL OR history.session_id <> ?2)
                 AND history.process_name = candidate.first_value
                 AND {second_expression} = candidate.second_value
             )
             ORDER BY candidate.first_value, candidate.second_value"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params![candidates_json, current_session_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn compare_sessions(
        &self,
        left_session_id: &str,
        right_session_id: &str,
    ) -> Result<SessionComparison, StorageError> {
        let left = self.session_comparison_counts(left_session_id)?;
        let right = self.session_comparison_counts(right_session_id)?;
        Ok(comparison_from_counts(
            left_session_id,
            right_session_id,
            &left,
            &right,
        ))
    }

    pub fn compare_session_to_connections(
        &self,
        left_session_id: &str,
        right_session_id: &str,
        right_connections: &[Connection],
    ) -> Result<SessionComparison, StorageError> {
        let left = self.session_comparison_counts(left_session_id)?;
        let right = comparison_counts_for_connections(right_connections);
        Ok(comparison_from_counts(
            left_session_id,
            right_session_id,
            &left,
            &right,
        ))
    }

    pub fn session_connection_count(&self, session_id: &str) -> Result<usize, StorageError> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM connections WHERE session_id=?1",
                params![session_id],
                |row| Ok(row.get::<_, i64>(0)?.max(0) as usize),
            )
            .map_err(Into::into)
    }

    pub fn session_counter_sources(&self, session_id: &str) -> Result<Vec<String>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT COALESCE(json_extract(connection_json, '$.counter_source'), 'unavailable')
             FROM connections WHERE session_id=?1 ORDER BY 1",
        )?;
        let rows = statement.query_map(params![session_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn session_comparison_counts(
        &self,
        session_id: &str,
    ) -> Result<SessionComparisonCounts, StorageError> {
        let load = |key_expression: &str| -> Result<BTreeMap<String, usize>, StorageError> {
            let sql = format!(
                "SELECT {key_expression} AS comparison_key,
                        SUM(COALESCE(NULLIF(CAST(json_extract(connection_json, '$.repeat_count') AS INTEGER), 0), 1))
                 FROM connections
                 WHERE session_id=?1
                 GROUP BY comparison_key"
            );
            let mut statement = self.connection.prepare(&sql)?;
            let rows = statement.query_map(params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?.max(0) as usize,
                ))
            })?;
            rows.collect::<Result<BTreeMap<_, _>, _>>()
                .map_err(Into::into)
        };
        Ok(SessionComparisonCounts {
            processes: load("process_name")?,
            destinations: load("COALESCE(NULLIF(domain, ''), remote_ip)")?,
            countries: load(
                "COALESCE(NULLIF(json_extract(connection_json, '$.geo.country_name'), ''), 'Unknown geography')",
            )?,
            ports: load("CAST(remote_port AS TEXT)")?,
        })
    }

    pub fn query_history_page(&self, query: &HistoryQuery) -> Result<HistoryPage, StorageError> {
        let limit = query.limit.clamp(1, MAX_QUERY_LIMIT);
        let filter = query
            .query
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::parse::<ConnectionFilter>)
            .transpose()
            .map_err(|error| StorageError::InvalidFilter(error.to_string()))?;

        let mut cursor = query.cursor.clone();
        let mut entries = Vec::with_capacity(limit.min(5_000));
        loop {
            let fetch_limit = limit.saturating_sub(entries.len()).clamp(64, 5_000);
            let mut sql = String::from(
                "SELECT id, last_seen_ms, session_id, connection_json FROM connections WHERE 1=1",
            );
            let mut parameters = Vec::<rusqlite::types::Value>::with_capacity(7);
            if let Some(from_ms) = query.from_ms {
                parameters.push(from_ms.into());
                sql.push_str(&format!(" AND last_seen_ms >= ?{}", parameters.len()));
            }
            if let Some(to_ms) = query.to_ms {
                parameters.push(to_ms.into());
                sql.push_str(&format!(" AND first_seen_ms <= ?{}", parameters.len()));
            }
            if let Some(session_id) = &query.session_id {
                parameters.push(session_id.clone().into());
                sql.push_str(&format!(" AND session_id = ?{}", parameters.len()));
            }
            if let Some(cursor) = &cursor {
                parameters.push(cursor.last_seen_ms.into());
                let timestamp_parameter = parameters.len();
                parameters.push(cursor.connection_id.clone().into());
                let id_parameter = parameters.len();
                sql.push_str(&format!(
                    " AND (last_seen_ms, id) < (?{timestamp_parameter}, ?{id_parameter})"
                ));
            }
            parameters.push((fetch_limit as i64).into());
            sql.push_str(&format!(
                " ORDER BY last_seen_ms DESC, id DESC LIMIT ?{}",
                parameters.len()
            ));
            let mut statement = self.connection.prepare_cached(&sql)?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(parameters.iter()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            if rows.is_empty() {
                return Ok(HistoryPage {
                    entries,
                    next_cursor: None,
                    has_more: false,
                });
            }
            let row_count = rows.len();
            for (index, (id, last_seen_ms, session_id, json)) in rows.into_iter().enumerate() {
                cursor = Some(HistoryCursor {
                    last_seen_ms,
                    connection_id: id,
                });
                let connection: Connection = serde_json::from_str(&json)?;
                if filter
                    .as_ref()
                    .is_none_or(|candidate| candidate.matches(&connection))
                {
                    entries.push(HistoryEntry {
                        session_id,
                        connection,
                    });
                    if entries.len() == limit {
                        return Ok(HistoryPage {
                            entries,
                            next_cursor: cursor,
                            has_more: index + 1 < row_count || row_count == fetch_limit,
                        });
                    }
                }
            }
            if row_count < fetch_limit {
                return Ok(HistoryPage {
                    entries,
                    next_cursor: None,
                    has_more: false,
                });
            }
        }
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
                ephemeral: false,
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
            self.connection
                .execute("DELETE FROM external_dns_queries", [])?;
            self.connection.execute("DELETE FROM traffic_samples", [])?;
            self.connection.execute("DELETE FROM traffic_rollups", [])?;
            self.connection.execute("DELETE FROM recordings", [])?;
            self.connection.execute("DELETE FROM sessions", [])?;
            return Ok(removed);
        }
        let cutoff = now_ms.saturating_sub(days.saturating_mul(86_400_000));
        self.connection.execute(
            "DELETE FROM external_dns_queries WHERE timestamp_ms < ?1",
            params![cutoff],
        )?;
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
        self.connection.execute(
            "DELETE FROM traffic_rollups WHERE bucket_ms < ?1",
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
                   sample_interval_ms,
                   bytes_sent, bytes_received, packets_sent, packets_received,
                   down_bytes_per_second, up_bytes_per_second, packets_per_second
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(session_id, timestamp_ms, process_key, source) DO UPDATE SET
                   sample_interval_ms = excluded.sample_interval_ms,
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
                    sample.sample_interval_ms.min(i64::MAX as u64) as i64,
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
            "WITH combined AS (
               SELECT timestamp_ms, COALESCE(session_id, '') AS session_id,
                      process_key, source, sample_interval_ms, 1 AS sample_count,
                      bytes_sent, bytes_received, packets_sent, packets_received,
                      down_bytes_per_second AS down_rate_sum,
                      up_bytes_per_second AS up_rate_sum,
                      packets_per_second AS packet_rate_sum
               FROM traffic_samples
               WHERE timestamp_ms BETWEEN ?2 AND ?3
                 AND (?4 IS NULL OR session_id = ?4)
                 AND ((?5 IS NULL AND process_key = '') OR process_key = ?5)
               UNION ALL
               SELECT bucket_ms AS timestamp_ms, session_id, process_key, source,
                      sample_interval_ms, sample_count,
                      bytes_sent, bytes_received, packets_sent, packets_received,
                      down_rate_sum, up_rate_sum, packet_rate_sum
               FROM traffic_rollups
               WHERE bucket_ms <= ?3
                 AND bucket_ms + resolution_ms > ?2
                 AND (?4 IS NULL OR session_id = ?4)
                 AND ((?5 IS NULL AND process_key = '') OR process_key = ?5)
             )
             SELECT (timestamp_ms / ?1) * ?1 AS output_bucket_ms,
                    NULLIF(session_id, ''), NULLIF(process_key, ''), source,
                    MAX(sample_interval_ms),
                    SUM(bytes_sent), SUM(bytes_received),
                    SUM(packets_sent), SUM(packets_received),
                    SUM(down_rate_sum) / MAX(1, SUM(sample_count)),
                    SUM(up_rate_sum) / MAX(1, SUM(sample_count)),
                    SUM(packet_rate_sum) / MAX(1, SUM(sample_count))
             FROM combined
             GROUP BY output_bucket_ms, session_id, process_key, source
             ORDER BY output_bucket_ms ASC
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
                    sample_interval_ms: row.get::<_, i64>(4)?.max(1) as u64,
                    source: match source.as_str() {
                        "interface_aggregate" => CounterSource::InterfaceAggregate,
                        "etw_flow" => CounterSource::EtwFlow,
                        "demo" => CounterSource::Demo,
                        _ => CounterSource::Unavailable,
                    },
                    counters: TrafficCounters {
                        bytes_sent: row.get::<_, i64>(5)?.max(0) as u64,
                        bytes_received: row.get::<_, i64>(6)?.max(0) as u64,
                        packets_sent: row.get::<_, i64>(7)?.max(0) as u64,
                        packets_received: row.get::<_, i64>(8)?.max(0) as u64,
                    },
                    rates: TransferRates {
                        down_bytes_per_second: row.get(9)?,
                        up_bytes_per_second: row.get(10)?,
                        packets_per_second: row.get(11)?,
                    },
                })
            },
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn compact_traffic_samples(&mut self, now_ms: i64) -> Result<usize, StorageError> {
        const RAW_WINDOW_MS: i64 = 60 * 60 * 1_000;
        const MINUTE_WINDOW_MS: i64 = 24 * 60 * 60 * 1_000;
        let raw_cutoff = now_ms.saturating_sub(RAW_WINDOW_MS);
        let minute_cutoff = now_ms.saturating_sub(MINUTE_WINDOW_MS);
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO traffic_rollups(
               resolution_ms, bucket_ms, session_id, process_key, source,
               sample_interval_ms, sample_count,
               bytes_sent, bytes_received, packets_sent, packets_received,
               down_rate_sum, up_rate_sum, packet_rate_sum
             )
             SELECT 60000, (timestamp_ms / 60000) * 60000,
                    COALESCE(session_id, ''), process_key, source,
                    60000, COUNT(*),
                    SUM(bytes_sent), SUM(bytes_received),
                    SUM(packets_sent), SUM(packets_received),
                    SUM(down_bytes_per_second), SUM(up_bytes_per_second),
                    SUM(packets_per_second)
             FROM traffic_samples
             WHERE timestamp_ms < ?1 AND sample_interval_ms > 1000
             GROUP BY (timestamp_ms / 60000), session_id, process_key, source
             ON CONFLICT(resolution_ms, bucket_ms, session_id, process_key, source)
             DO UPDATE SET
               sample_count = sample_count + excluded.sample_count,
               bytes_sent = bytes_sent + excluded.bytes_sent,
               bytes_received = bytes_received + excluded.bytes_received,
               packets_sent = packets_sent + excluded.packets_sent,
               packets_received = packets_received + excluded.packets_received,
               down_rate_sum = down_rate_sum + excluded.down_rate_sum,
               up_rate_sum = up_rate_sum + excluded.up_rate_sum,
               packet_rate_sum = packet_rate_sum + excluded.packet_rate_sum",
            params![raw_cutoff],
        )?;
        let removed_raw = transaction.execute(
            "DELETE FROM traffic_samples
             WHERE timestamp_ms < ?1 AND sample_interval_ms > 1000",
            params![raw_cutoff],
        )?;
        transaction.execute(
            "INSERT INTO traffic_rollups(
               resolution_ms, bucket_ms, session_id, process_key, source,
               sample_interval_ms, sample_count,
               bytes_sent, bytes_received, packets_sent, packets_received,
               down_rate_sum, up_rate_sum, packet_rate_sum
             )
             SELECT 3600000, (bucket_ms / 3600000) * 3600000,
                    session_id, process_key, source, 3600000, SUM(sample_count),
                    SUM(bytes_sent), SUM(bytes_received),
                    SUM(packets_sent), SUM(packets_received),
                    SUM(down_rate_sum), SUM(up_rate_sum), SUM(packet_rate_sum)
             FROM traffic_rollups
             WHERE resolution_ms = 60000 AND bucket_ms < ?1
             GROUP BY (bucket_ms / 3600000), session_id, process_key, source
             ON CONFLICT(resolution_ms, bucket_ms, session_id, process_key, source)
             DO UPDATE SET
               sample_count = sample_count + excluded.sample_count,
               bytes_sent = bytes_sent + excluded.bytes_sent,
               bytes_received = bytes_received + excluded.bytes_received,
               packets_sent = packets_sent + excluded.packets_sent,
               packets_received = packets_received + excluded.packets_received,
               down_rate_sum = down_rate_sum + excluded.down_rate_sum,
               up_rate_sum = up_rate_sum + excluded.up_rate_sum,
               packet_rate_sum = packet_rate_sum + excluded.packet_rate_sum",
            params![minute_cutoff],
        )?;
        let removed_minutes = transaction.execute(
            "DELETE FROM traffic_rollups
             WHERE resolution_ms = 60000 AND bucket_ms < ?1",
            params![minute_cutoff],
        )?;
        transaction.commit()?;
        Ok(removed_raw.saturating_add(removed_minutes))
    }

    pub fn save_investigation(
        &self,
        investigation: &SavedInvestigation,
    ) -> Result<(), StorageError> {
        validate_investigation(investigation)?;
        let query_json = serde_json::to_string(&investigation.query)?;
        let workspace_json = serde_json::to_string(&InvestigationWorkspace {
            tags: investigation.tags.clone(),
            selected_connection_id: investigation.selected_connection_id.clone(),
            layout_json: investigation.layout_json.clone(),
            autosave_revision: investigation.autosave_revision,
        })?;
        self.connection.execute(
            "INSERT INTO investigations(
               id, name, query_json, notes, workspace_json, created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name,
               query_json=excluded.query_json,
               notes=excluded.notes,
               workspace_json=excluded.workspace_json,
               updated_at_ms=excluded.updated_at_ms",
            params![
                investigation.id,
                investigation.name,
                query_json,
                investigation.notes,
                workspace_json,
                investigation.created_at_ms,
                investigation.updated_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn list_investigations(&self) -> Result<Vec<SavedInvestigation>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, query_json, notes, workspace_json, created_at_ms, updated_at_ms
             FROM investigations ORDER BY updated_at_ms DESC, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;
        let mut investigations = Vec::new();
        for row in rows {
            let (id, name, query_json, notes, workspace_json, created_at_ms, updated_at_ms) = row?;
            let workspace: InvestigationWorkspace = serde_json::from_str(&workspace_json)?;
            investigations.push(SavedInvestigation {
                id,
                name,
                query: serde_json::from_str(&query_json)?,
                notes,
                tags: workspace.tags,
                selected_connection_id: workspace.selected_connection_id,
                layout_json: workspace.layout_json,
                autosave_revision: workspace.autosave_revision,
                created_at_ms,
                updated_at_ms,
            });
        }
        Ok(investigations)
    }

    pub fn delete_investigation(&self, id: &str) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM investigations WHERE id=?1", params![id])?
            > 0)
    }

    pub fn save_local_rule(&self, rule: &LocalRule) -> Result<(), StorageError> {
        rule.validate().map_err(StorageError::InvalidRule)?;
        let rule_json = serde_json::to_string(rule)?;
        let existing = self
            .connection
            .query_row(
                "SELECT version, rule_json FROM local_rules WHERE id=?1",
                params![rule.id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((version, existing_json)) = existing {
            let incoming = i64::from(rule.version);
            if incoming < version {
                return Err(StorageError::InvalidRule(
                    "a rule cannot be replaced by an older version".into(),
                ));
            }
            if incoming == version && existing_json != rule_json {
                return Err(StorageError::InvalidRule(
                    "editing a rule requires incrementing its version".into(),
                ));
            }
        }
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT OR IGNORE INTO local_rule_versions(id, version, rule_json, created_at_ms)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                rule.id,
                i64::from(rule.version),
                rule_json,
                rule.updated_at_ms
            ],
        )?;
        transaction.execute(
            "INSERT INTO local_rules(id, version, enabled, rule_json, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               version=excluded.version,
               enabled=excluded.enabled,
               rule_json=excluded.rule_json,
               updated_at_ms=excluded.updated_at_ms",
            params![
                rule.id,
                i64::from(rule.version),
                rule.enabled,
                rule_json,
                rule.created_at_ms,
                rule.updated_at_ms,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_local_rules(&self) -> Result<Vec<LocalRule>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT rule_json FROM local_rules ORDER BY updated_at_ms DESC, id")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(StorageError::from))
            .collect()
    }

    pub fn list_local_rule_versions(&self, id: &str) -> Result<Vec<LocalRule>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT rule_json FROM local_rule_versions WHERE id=?1 ORDER BY version DESC",
        )?;
        let rows = statement.query_map(params![id], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(StorageError::from))
            .collect()
    }

    pub fn delete_local_rule(&self, id: &str) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM local_rules WHERE id=?1", params![id])?
            > 0)
    }

    pub fn record_rule_match(
        &self,
        rule_match: &RuleMatch,
        matched_at_ms: i64,
    ) -> Result<bool, StorageError> {
        if !rule_match.matched {
            return Ok(false);
        }
        Ok(self.connection.execute(
            "INSERT OR IGNORE INTO rule_matches(
               rule_id, rule_version, connection_id, matched_at_ms, match_json
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                rule_match.rule_id,
                i64::from(rule_match.rule_version),
                rule_match.connection_id,
                matched_at_ms,
                serde_json::to_string(rule_match)?,
            ],
        )? > 0)
    }

    pub fn list_rule_matches(&self, limit: usize) -> Result<Vec<RuleMatch>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT match_json FROM rule_matches
             ORDER BY matched_at_ms DESC, id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, MAX_QUERY_LIMIT) as i64], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| serde_json::from_str(&row?).map_err(StorageError::from))
            .collect()
    }

    pub fn apply_rule_match_actions(
        &self,
        rule_match: &RuleMatch,
        updated_at_ms: i64,
    ) -> Result<Option<ConnectionAnnotation>, StorageError> {
        if !rule_match.matched {
            return Ok(None);
        }
        let transaction = self.connection.unchecked_transaction()?;
        let existing = transaction
            .query_row(
                "SELECT annotation_json FROM connection_annotations WHERE connection_id=?1",
                params![rule_match.connection_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let mut annotation = existing
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_else(|| ConnectionAnnotation {
                connection_id: rule_match.connection_id.clone(),
                ..ConnectionAnnotation::default()
            });
        for action in &rule_match.actions {
            match action {
                LocalRuleAction::Tag(tag) => {
                    annotation.tags.insert(tag.clone());
                }
                LocalRuleAction::Pin => annotation.pinned = true,
                LocalRuleAction::AddToInvestigation(investigation_id) => {
                    annotation
                        .investigation_ids
                        .insert(investigation_id.clone());
                    let query_json = transaction
                        .query_row(
                            "SELECT query_json FROM investigations WHERE id=?1",
                            params![investigation_id],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()?;
                    if let Some(query_json) = query_json {
                        let mut query: InvestigationQuery = serde_json::from_str(&query_json)?;
                        if !query
                            .pinned_connection_ids
                            .contains(&rule_match.connection_id)
                        {
                            query
                                .pinned_connection_ids
                                .push(rule_match.connection_id.clone());
                            transaction.execute(
                                "UPDATE investigations SET query_json=?1, updated_at_ms=?2 WHERE id=?3",
                                params![
                                    serde_json::to_string(&query)?,
                                    updated_at_ms,
                                    investigation_id
                                ],
                            )?;
                        }
                    }
                }
                LocalRuleAction::Notify => {}
            }
        }
        annotation.updated_at_ms = updated_at_ms;
        transaction.execute(
            "INSERT INTO connection_annotations(connection_id, annotation_json, updated_at_ms)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(connection_id) DO UPDATE SET
               annotation_json=excluded.annotation_json,
               updated_at_ms=excluded.updated_at_ms",
            params![
                annotation.connection_id,
                serde_json::to_string(&annotation)?,
                annotation.updated_at_ms,
            ],
        )?;
        transaction.commit()?;
        Ok(Some(annotation))
    }

    pub fn list_connection_annotations(
        &self,
        limit: usize,
    ) -> Result<Vec<ConnectionAnnotation>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT annotation_json FROM connection_annotations
             ORDER BY updated_at_ms DESC, connection_id LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, MAX_QUERY_LIMIT) as i64], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| serde_json::from_str(&row?).map_err(StorageError::from))
            .collect()
    }

    pub fn save_notification(
        &self,
        notification: &LocalNotificationRecord,
    ) -> Result<(), StorageError> {
        if notification.id.is_empty() || notification.rule_id.is_empty() {
            return Err(StorageError::InvalidRule(
                "notification and rule ids cannot be empty".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO local_notifications(
               id, rule_id, state, last_at_ms, notification_json
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               state=excluded.state,
               last_at_ms=excluded.last_at_ms,
               notification_json=excluded.notification_json",
            params![
                notification.id,
                notification.rule_id,
                notification_state_name(notification.state),
                notification.last_at_ms,
                serde_json::to_string(notification)?,
            ],
        )?;
        Ok(())
    }

    pub fn list_notifications(
        &self,
        limit: usize,
    ) -> Result<Vec<LocalNotificationRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT notification_json FROM local_notifications
             ORDER BY last_at_ms DESC, id LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, MAX_QUERY_LIMIT) as i64], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| serde_json::from_str(&row?).map_err(StorageError::from))
            .collect()
    }

    pub fn update_notification_state(
        &self,
        id: &str,
        state: NotificationState,
        snoozed_until_ms: Option<i64>,
    ) -> Result<bool, StorageError> {
        let Some(json) = self
            .connection
            .query_row(
                "SELECT notification_json FROM local_notifications WHERE id=?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        else {
            return Ok(false);
        };
        let mut notification: LocalNotificationRecord = serde_json::from_str(&json)?;
        notification.state = state;
        notification.snoozed_until_ms = snoozed_until_ms;
        self.save_notification(&notification)?;
        Ok(true)
    }

    pub fn save_network_profile(
        &self,
        profile: &NetworkProfile,
        updated_at_ms: i64,
    ) -> Result<(), StorageError> {
        if profile.id.is_empty() || !profile.sensitive_fields_redacted {
            return Err(StorageError::InvalidEvidence(
                "network profiles require a pseudonymous id and redacted signature".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO network_profiles(
               id, display_name, profile_json, created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               display_name=excluded.display_name,
               profile_json=excluded.profile_json,
               updated_at_ms=excluded.updated_at_ms",
            params![
                profile.id,
                profile.display_name,
                serde_json::to_string(profile)?,
                profile.created_at_ms,
                updated_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn list_network_profiles(&self) -> Result<Vec<NetworkProfile>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT profile_json FROM network_profiles ORDER BY updated_at_ms DESC, id")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(StorageError::from))
            .collect()
    }

    pub fn save_today_card_state(&self, state: &TodayCardState) -> Result<(), StorageError> {
        if state.id.is_empty() || state.id.len() > 256 {
            return Err(StorageError::InvalidEvidence(
                "today card id must contain 1 to 256 characters".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO today_cards(id, dismissed, pinned, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
               dismissed=excluded.dismissed,
               pinned=excluded.pinned,
               updated_at_ms=excluded.updated_at_ms",
            params![state.id, state.dismissed, state.pinned, state.updated_at_ms],
        )?;
        Ok(())
    }

    pub fn list_today_card_states(&self) -> Result<Vec<TodayCardState>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, dismissed, pinned, updated_at_ms
             FROM today_cards ORDER BY pinned DESC, updated_at_ms DESC, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TodayCardState {
                id: row.get(0)?,
                dismissed: row.get(1)?,
                pinned: row.get(2)?,
                updated_at_ms: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_listener_snapshot(
        &self,
        session_id: Option<&str>,
        observed_at_ms: i64,
        listeners: &[ListenerExposure],
    ) -> Result<ListenerSnapshot, StorageError> {
        let mut listeners = listeners.to_vec();
        listeners
            .sort_by_cached_key(|listener| serde_json::to_string(listener).unwrap_or_default());
        let snapshot_sha256 = projection_sha256(&listeners)?;
        let listeners_json = serde_json::to_string(&listeners)?;
        self.connection.execute(
            "INSERT INTO listener_snapshots(
               session_id, observed_at_ms, snapshot_sha256, listeners_json
             ) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, observed_at_ms, snapshot_sha256, listeners_json],
        )?;
        Ok(ListenerSnapshot {
            id: Some(self.connection.last_insert_rowid()),
            session_id: session_id.map(str::to_owned),
            observed_at_ms,
            snapshot_sha256,
            listeners,
        })
    }

    pub fn list_listener_snapshots(
        &self,
        limit: usize,
    ) -> Result<Vec<ListenerSnapshot>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, session_id, observed_at_ms, snapshot_sha256, listeners_json
             FROM listener_snapshots ORDER BY observed_at_ms DESC, id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, 10_000) as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut snapshots = Vec::new();
        for row in rows {
            let (id, session_id, observed_at_ms, snapshot_sha256, listeners_json) = row?;
            snapshots.push(ListenerSnapshot {
                id: Some(id),
                session_id,
                observed_at_ms,
                snapshot_sha256,
                listeners: serde_json::from_str(&listeners_json)?,
            });
        }
        Ok(snapshots)
    }

    pub fn record_handshake_mode_audit(
        &self,
        audit: &HandshakeModeAudit,
    ) -> Result<(), StorageError> {
        if audit.maximum_bytes > 65_536 || audit.maximum_packets > 64 {
            return Err(StorageError::InvalidEvidence(
                "handshake metadata budget exceeds the local safety limit".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO handshake_mode_audit(
               occurred_at_ms, enabled, maximum_bytes, maximum_packets,
               duration_limit_ms, detail
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                audit.occurred_at_ms,
                audit.enabled,
                i64::from(audit.maximum_bytes),
                i64::from(audit.maximum_packets),
                to_sql_u64(audit.duration_limit_ms),
                audit.detail,
            ],
        )?;
        Ok(())
    }

    pub fn list_handshake_mode_audit(
        &self,
        limit: usize,
    ) -> Result<Vec<HandshakeModeAudit>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT occurred_at_ms, enabled, maximum_bytes, maximum_packets,
                    duration_limit_ms, detail
             FROM handshake_mode_audit ORDER BY occurred_at_ms DESC, id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, 10_000) as i64], |row| {
            Ok(HandshakeModeAudit {
                occurred_at_ms: row.get(0)?,
                enabled: row.get(1)?,
                maximum_bytes: row.get::<_, i64>(2)?.max(0) as u32,
                maximum_packets: row.get::<_, i64>(3)?.max(0) as u32,
                duration_limit_ms: row.get::<_, i64>(4)?.max(0) as u64,
                detail: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn query_investigation(
        &self,
        query: &InvestigationQuery,
    ) -> Result<Vec<HistoryEntry>, StorageError> {
        let mut entries = self.query_history(&HistoryQuery {
            from_ms: query.from_ms,
            to_ms: query.to_ms,
            session_id: query.session_id.clone(),
            query: query.filter.clone(),
            limit: MAX_QUERY_LIMIT,
            cursor: None,
        })?;
        if query.pinned_connection_ids.is_empty()
            && query.pinned_destination_ips.is_empty()
            && query.pinned_process_keys.is_empty()
            && query.pinned_asns.is_empty()
        {
            return Ok(entries);
        }
        entries.retain(|entry| {
            query.pinned_connection_ids.contains(&entry.connection.id)
                || query
                    .pinned_destination_ips
                    .contains(&entry.connection.key.remote.address)
                || query.pinned_process_keys.iter().any(|key| {
                    key.eq_ignore_ascii_case(&entry.connection.process.name)
                        || entry.connection.process.pid.is_some_and(|pid| {
                            key.eq_ignore_ascii_case(&format!(
                                "{}:{pid}",
                                entry.connection.process.name
                            ))
                        })
                })
                || entry
                    .connection
                    .asn
                    .as_ref()
                    .is_some_and(|asn| query.pinned_asns.contains(&asn.number))
        });
        Ok(entries)
    }

    pub fn query_investigation_redacted(
        &self,
        query: &InvestigationQuery,
        redaction: &ExportRedaction,
    ) -> Result<Vec<HistoryEntry>, StorageError> {
        let mut entries = self.query_investigation(query)?;
        apply_export_redaction(&mut entries, redaction);
        Ok(entries)
    }

    pub fn replace_geo_views(
        &mut self,
        session_id: &str,
        view_kind: &str,
        enrichments: &[DerivedGeoEnrichment],
        city_generation: u64,
        asn_generation: u64,
        created_at_ms: i64,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM geo_views
             WHERE view_kind=?1 AND connection_id IN (
               SELECT id FROM connections WHERE session_id=?2
             )",
            params![view_kind, session_id],
        )?;
        {
            let mut statement = transaction.prepare_cached(
                "INSERT INTO geo_views(
                   connection_id, view_kind, city_generation, asn_generation,
                   enrichment_json, created_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for enrichment in enrichments {
                statement.execute(params![
                    enrichment.connection_id,
                    view_kind,
                    to_sql_u64(city_generation),
                    to_sql_u64(asn_generation),
                    serde_json::to_string(enrichment)?,
                    created_at_ms,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn load_geo_views(
        &self,
        session_id: &str,
        view_kind: &str,
    ) -> Result<Vec<DerivedGeoEnrichment>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT geo_views.enrichment_json
             FROM geo_views
             JOIN connections ON connections.id=geo_views.connection_id
             WHERE connections.session_id=?1 AND geo_views.view_kind=?2
             ORDER BY connections.last_seen_ms DESC",
        )?;
        let rows = statement.query_map(params![session_id, view_kind], |row| {
            row.get::<_, Option<String>>(0)
        })?;
        let mut enrichments = Vec::new();
        for row in rows {
            if let Some(json) = row? {
                enrichments.push(serde_json::from_str(&json)?);
            }
        }
        Ok(enrichments)
    }

    pub fn clear_sessions(&self) -> Result<usize, StorageError> {
        let dns = self
            .connection
            .execute("DELETE FROM external_dns_queries", [])?;
        let rollups = self.connection.execute("DELETE FROM traffic_rollups", [])?;
        Ok(dns
            .saturating_add(rollups)
            .saturating_add(self.connection.execute("DELETE FROM sessions", [])?))
    }

    pub fn clear_all_user_data(&self) -> Result<usize, StorageError> {
        let mut removed = 0;
        for table in [
            "sessions",
            "investigations",
            "bookmarks",
            "ignore_rules",
            "settings",
            "local_rules",
            "local_rule_versions",
            "rule_matches",
            "connection_annotations",
            "local_notifications",
            "network_profiles",
            "today_cards",
            "listener_snapshots",
            "handshake_mode_audit",
            "traffic_rollups",
            "external_dns_queries",
        ] {
            removed += self
                .connection
                .execute(&format!("DELETE FROM {table}"), [])?;
        }
        self.connection.execute_batch(
            "PRAGMA wal_checkpoint(TRUNCATE);
             PRAGMA optimize;",
        )?;
        Ok(removed)
    }

    pub fn export(&self, request: &ExportRequest) -> Result<ExportResult, StorageError> {
        let path = validate_export_path(&request.path, request.format)?;
        let mut query = HistoryQuery {
            from_ms: request.from_ms,
            to_ms: request.to_ms,
            session_id: request.session_id.clone(),
            query: request.query.clone(),
            limit: 5_000,
            cursor: None,
        };
        static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let (temp_path, file) = loop {
            let mut name = path.file_name().unwrap().to_os_string();
            name.push(format!(
                ".{}.{}.tmp",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            let candidate = path.with_file_name(name);
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => break (candidate, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        };
        let result = (|| -> Result<ExportResult, StorageError> {
            let transaction = self.connection.unchecked_transaction()?;
            let mut output = io::BufWriter::new(file);
            match request.format {
                ExportFormat::Json => output.write_all(b"[")?,
                ExportFormat::Csv => output.write_all(export_csv(&[]).as_bytes())?,
            }
            let process_key = request.process_key.as_ref().map(|key| key.to_lowercase());
            let mut records = 0;
            loop {
                let page = self.query_history_page(&query)?;
                let mut entries = page.entries;
                if let Some(process_key) = &process_key {
                    entries.retain(|entry| {
                        entry.connection.process.name.to_lowercase() == *process_key
                            || entry.connection.process.pid.is_some_and(|pid| {
                                format!("{}:{pid}", entry.connection.process.name).to_lowercase()
                                    == *process_key
                            })
                    });
                }
                apply_export_redaction(&mut entries, &request.redaction);
                match request.format {
                    ExportFormat::Json => {
                        for entry in &entries {
                            if records > 0 {
                                output.write_all(b",")?;
                            }
                            serde_json::to_writer(&mut output, entry)?;
                            records += 1;
                        }
                    }
                    ExportFormat::Csv => {
                        let csv = export_csv(&entries);
                        output.write_all(csv.split_once('\n').unwrap().1.as_bytes())?;
                        records += entries.len();
                    }
                }
                if !page.has_more {
                    break;
                }
                query.cursor = page.next_cursor;
            }
            if matches!(request.format, ExportFormat::Json) {
                output.write_all(b"]")?;
            }
            output.flush()?;
            let bytes_written = output.get_ref().metadata()?.len();
            drop(output);
            transaction.commit()?;
            fs::rename(&temp_path, &path)?;
            Ok(ExportResult {
                path: path.to_string_lossy().into_owned(),
                records,
                bytes_written,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        result
    }

    pub fn backup_to(&self, destination: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
        let destination = resolve_new_path(destination.as_ref())?;
        if self
            .path
            .as_ref()
            .and_then(|path| fs::canonicalize(path).ok())
            .is_some_and(|source| source == destination)
        {
            return Err(StorageError::InvalidEvidence(
                "backup destination must differ from the active database".into(),
            ));
        }
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(FULL);")?;
        self.connection
            .backup(rusqlite::MAIN_DB, &destination, None)?;
        Ok(destination)
    }

    pub fn read_only_quick_check(path: impl AsRef<Path>) -> Result<String, StorageError> {
        let connection = SqliteConnection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection
            .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
            .map_err(Into::into)
    }

    pub fn salvage_database(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        backup: impl AsRef<Path>,
    ) -> Result<RecoveryReport, StorageError> {
        let source = fs::canonicalize(source)?;
        let destination = resolve_new_path(destination.as_ref())?;
        let backup = resolve_new_path(backup.as_ref())?;
        if source == destination || source == backup || destination == backup {
            return Err(StorageError::InvalidEvidence(
                "source, recovery destination and backup must be distinct".into(),
            ));
        }
        fs::copy(&source, &backup)?;
        let recovered = Storage::open(&destination)?;
        recovered
            .connection
            .execute_batch("PRAGMA foreign_keys=OFF;")?;
        let source_uri = sqlite_read_only_uri(&source);
        recovered
            .connection
            .execute("ATTACH DATABASE ?1 AS damaged", params![source_uri])?;

        let tables = [
            "sessions",
            "settings",
            "bookmarks",
            "ignore_rules",
            "connections",
            "recordings",
            "traffic_samples",
            "investigations",
            "geo_views",
            "evidence_journal",
            "projection_runs",
            "evidence_rollups",
            "search_fts",
            "network_action_audit",
            "local_rules",
            "local_rule_versions",
            "rule_matches",
            "connection_annotations",
            "local_notifications",
            "network_profiles",
            "today_cards",
            "listener_snapshots",
            "handshake_mode_audit",
        ];
        let mut recovered_rows = 0_u64;
        let mut lost_rows = 0_u64;
        let mut recovered_tables = Vec::new();
        let mut skipped_tables = Vec::new();
        for table in tables {
            let source_exists = recovered
                .connection
                .query_row(
                    "SELECT 1 FROM damaged.sqlite_master WHERE name=?1 LIMIT 1",
                    params![table],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !source_exists {
                skipped_tables.push(table.into());
                continue;
            }
            let source_count = table_count(&recovered.connection, "damaged", table).unwrap_or(0);
            let source_columns = table_columns(&recovered.connection, "damaged", table)?;
            let destination_columns = table_columns(&recovered.connection, "main", table)?;
            let columns = destination_columns
                .into_iter()
                .filter(|column| source_columns.contains(column))
                .collect::<Vec<_>>();
            if columns.is_empty() {
                lost_rows = lost_rows.saturating_add(source_count);
                skipped_tables.push(table.into());
                continue;
            }
            let column_sql = columns
                .iter()
                .map(|column| quote_identifier(column))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "INSERT OR IGNORE INTO main.{}({column_sql})
                 SELECT {column_sql} FROM damaged.{}",
                quote_identifier(table),
                quote_identifier(table)
            );
            match recovered.connection.execute(&sql, []) {
                Ok(inserted) => {
                    recovered_rows = recovered_rows.saturating_add(inserted as u64);
                    lost_rows =
                        lost_rows.saturating_add(source_count.saturating_sub(inserted as u64));
                    recovered_tables.push(table.into());
                }
                Err(_) => {
                    lost_rows = lost_rows.saturating_add(source_count);
                    skipped_tables.push(table.into());
                }
            }
        }
        recovered
            .connection
            .execute_batch("DETACH DATABASE damaged;")?;
        recovered
            .connection
            .execute_batch("PRAGMA foreign_keys=ON;")?;
        let gaps_marked = recovered
            .connection
            .prepare("PRAGMA foreign_key_check")?
            .query_map([], |_| Ok(()))?
            .filter_map(Result::ok)
            .count() as u64;
        let detail = recovered
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))?;
        let quick_check_ok = detail.eq_ignore_ascii_case("ok");
        let report = RecoveryReport {
            source_path: source.to_string_lossy().into_owned(),
            destination_path: destination.to_string_lossy().into_owned(),
            backup_path: backup.to_string_lossy().into_owned(),
            recovered_rows,
            lost_rows,
            recovered_tables,
            skipped_tables,
            gaps_marked,
            quick_check_ok,
            detail,
        };
        recovered.connection.execute(
            "INSERT INTO recovery_events(
               occurred_at_ms, operation, source_path, destination_path, backup_path,
               recovered_rows, lost_rows, detail
             ) VALUES (?1, 'salvage', ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                chrono::Utc::now().timestamp_millis(),
                report.source_path,
                report.destination_path,
                report.backup_path,
                to_sql_u64(report.recovered_rows),
                to_sql_u64(report.lost_rows),
                report.detail,
            ],
        )?;
        Ok(report)
    }

    pub fn health_check(&self) -> Result<StorageHealth, StorageError> {
        let detail = self
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))?;
        let (wal_busy_pages, wal_log_pages, wal_checkpointed_pages) =
            self.connection
                .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?;
        Ok(StorageHealth {
            quick_check_ok: detail.eq_ignore_ascii_case("ok"),
            detail,
            wal_busy_pages,
            wal_log_pages,
            wal_checkpointed_pages,
            database_size_bytes: self.database_size_bytes(),
            checked_at_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    pub fn optimize(&self) -> Result<(), StorageError> {
        self.connection.execute_batch("PRAGMA optimize;")?;
        Ok(())
    }

    pub fn compact(&self) -> Result<StorageHealth, StorageError> {
        self.connection.execute_batch(
            "PRAGMA wal_checkpoint(TRUNCATE);
             VACUUM;
             PRAGMA optimize;",
        )?;
        self.health_check()
    }

    pub fn database_size_bytes(&self) -> u64 {
        self.path
            .as_ref()
            .and_then(|path| fs::metadata(path).ok())
            .map_or(0, |metadata| metadata.len())
    }
}

pub fn projection_sha256(value: &impl serde::Serialize) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(value)?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
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

fn count_before(
    connection: &SqliteConnection,
    table: &str,
    time_column: &str,
    cutoff: i64,
) -> Result<i64, rusqlite::Error> {
    debug_assert!(
        [
            "evidence_journal",
            "connections",
            "traffic_samples",
            "traffic_rollups",
            "evidence_rollups"
        ]
        .contains(&table)
    );
    debug_assert!(
        [
            "utc_ms",
            "last_seen_ms",
            "timestamp_ms",
            "bucket_ms",
            "day_utc"
        ]
        .contains(&time_column)
    );
    connection.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE {time_column} < ?1"),
        params![cutoff],
        |row| row.get(0),
    )
}

fn resolve_new_path(path: &Path) -> Result<PathBuf, StorageError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if absolute.exists() {
        return Err(StorageError::InvalidEvidence(format!(
            "refusing to overwrite existing recovery output {}",
            absolute.display()
        )));
    }
    let file_name = absolute
        .file_name()
        .ok_or_else(|| StorageError::InvalidEvidence("recovery output needs a file name".into()))?;
    let parent = absolute.parent().ok_or_else(|| {
        StorageError::InvalidEvidence("recovery output needs a parent directory".into())
    })?;
    fs::create_dir_all(parent)?;
    Ok(fs::canonicalize(parent)?.join(file_name))
}

fn table_count(
    connection: &SqliteConnection,
    schema: &str,
    table: &str,
) -> Result<u64, rusqlite::Error> {
    debug_assert!(matches!(schema, "main" | "damaged"));
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM {}.{}",
                quote_identifier(schema),
                quote_identifier(table)
            ),
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count.max(0) as u64)
}

fn table_columns(
    connection: &SqliteConnection,
    schema: &str,
    table: &str,
) -> Result<Vec<String>, rusqlite::Error> {
    debug_assert!(matches!(schema, "main" | "damaged"));
    let mut statement = connection.prepare(&format!(
        "PRAGMA {}.table_info({})",
        quote_identifier(schema),
        quote_sql_string(table)
    ))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    columns.collect()
}

fn table_has_column(
    connection: &SqliteConnection,
    table: &str,
    column: &str,
) -> Result<bool, rusqlite::Error> {
    let mut statement =
        connection.prepare(&format!("PRAGMA table_info({})", quote_sql_string(table)))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in columns {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn quote_sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sqlite_read_only_uri(path: &Path) -> String {
    let path = path.to_string_lossy();
    #[cfg(target_os = "windows")]
    {
        let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
        if let Some(unc) = path.strip_prefix("UNC\\") {
            return format!("file://{}?mode=ro&immutable=1", unc.replace('\\', "/"));
        }
        format!("file:///{}?mode=ro&immutable=1", path.replace('\\', "/"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!("file:{}?mode=ro&immutable=1", path)
    }
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

fn migration_v4(transaction: &Transaction<'_>) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(
        "ALTER TABLE connections ADD COLUMN remote_scope TEXT NOT NULL DEFAULT 'unknown';
         ALTER TABLE connections ADD COLUMN endpoint_role TEXT NOT NULL DEFAULT 'unknown';
         ALTER TABLE connections ADD COLUMN role_reason TEXT NOT NULL DEFAULT 'historical_record';
         ALTER TABLE connections ADD COLUMN observation_source TEXT NOT NULL DEFAULT 'historical';
         ALTER TABLE connections ADD COLUMN correlation_quality TEXT NOT NULL DEFAULT 'unavailable';
         ALTER TABLE connections ADD COLUMN process_instance TEXT;
         ALTER TABLE connections ADD COLUMN interface_id TEXT;
         ALTER TABLE connections ADD COLUMN capture_generation INTEGER NOT NULL DEFAULT 0;
         ALTER TABLE connections ADD COLUMN flow_generation INTEGER NOT NULL DEFAULT 0;
         ALTER TABLE connections ADD COLUMN city_generation INTEGER NOT NULL DEFAULT 0;
         ALTER TABLE connections ADD COLUMN asn_generation INTEGER NOT NULL DEFAULT 0;
         CREATE INDEX idx_connections_scope_time
           ON connections(remote_scope, last_seen_ms DESC);
         CREATE INDEX idx_connections_asn_time
           ON connections(asn, last_seen_ms DESC);
         CREATE TABLE investigations (
           id TEXT PRIMARY KEY,
           name TEXT NOT NULL,
           query_json TEXT NOT NULL,
           notes TEXT NOT NULL DEFAULT '',
           created_at_ms INTEGER NOT NULL,
           updated_at_ms INTEGER NOT NULL
         );
         CREATE TABLE geo_views (
           connection_id TEXT NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
           view_kind TEXT NOT NULL,
           city_generation INTEGER NOT NULL DEFAULT 0,
           asn_generation INTEGER NOT NULL DEFAULT 0,
           enrichment_json TEXT,
           created_at_ms INTEGER NOT NULL,
           PRIMARY KEY(connection_id, view_kind, city_generation, asn_generation)
         );",
    )
}

fn migration_v5(transaction: &Transaction<'_>) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(
        "ALTER TABLE investigations
           ADD COLUMN workspace_json TEXT NOT NULL DEFAULT '{}';
         CREATE TABLE evidence_journal (
           id TEXT PRIMARY KEY,
           session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
           schema_version INTEGER NOT NULL,
           provider_id TEXT NOT NULL,
           provider_generation INTEGER NOT NULL,
           provider_sequence INTEGER NOT NULL,
           monotonic_ns INTEGER NOT NULL,
           utc_ms INTEGER NOT NULL,
           kind TEXT NOT NULL,
           process_instance TEXT,
           remote_ip TEXT,
           domain TEXT,
           asn INTEGER,
           route_id TEXT,
           listener_scope TEXT,
           network_profile_id TEXT,
           observation_json TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           UNIQUE(provider_id, provider_generation, provider_sequence)
         );
         CREATE INDEX idx_evidence_time
           ON evidence_journal(monotonic_ns, provider_id, provider_generation, provider_sequence);
         CREATE INDEX idx_evidence_utc ON evidence_journal(utc_ms);
         CREATE INDEX idx_evidence_provider
           ON evidence_journal(provider_id, provider_generation, provider_sequence);
         CREATE INDEX idx_evidence_process_time
           ON evidence_journal(process_instance, utc_ms DESC);
         CREATE INDEX idx_evidence_remote_time ON evidence_journal(remote_ip, utc_ms DESC);
         CREATE INDEX idx_evidence_domain_time ON evidence_journal(domain, utc_ms DESC);
         CREATE INDEX idx_evidence_asn_time ON evidence_journal(asn, utc_ms DESC);
         CREATE INDEX idx_evidence_route_time ON evidence_journal(route_id, utc_ms DESC);
         CREATE INDEX idx_evidence_listener_time ON evidence_journal(listener_scope, utc_ms DESC);
         CREATE INDEX idx_evidence_profile_time
           ON evidence_journal(network_profile_id, utc_ms DESC);
         CREATE TRIGGER evidence_journal_immutable_update
           BEFORE UPDATE ON evidence_journal
           BEGIN
             SELECT RAISE(ABORT, 'evidence journal rows are immutable');
           END;
         CREATE TABLE projection_runs (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           projection_name TEXT NOT NULL,
           projection_version INTEGER NOT NULL,
           first_observation_id TEXT,
           last_observation_id TEXT,
           observation_count INTEGER NOT NULL,
           result_sha256 TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           UNIQUE(projection_name, projection_version, result_sha256)
         );
         CREATE TABLE evidence_rollups (
           day_utc INTEGER NOT NULL,
           process_key TEXT NOT NULL,
           remote_ip TEXT NOT NULL,
           connections INTEGER NOT NULL DEFAULT 0,
           first_seen_ms INTEGER NOT NULL,
           last_seen_ms INTEGER NOT NULL,
           PRIMARY KEY(day_utc, process_key, remote_ip)
         );
         CREATE VIRTUAL TABLE search_fts USING fts5(
           reference_id UNINDEXED,
           kind,
           name,
           notes,
           tags,
           detail,
           tokenize='unicode61 remove_diacritics 2'
         );
         CREATE TABLE recovery_events (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           occurred_at_ms INTEGER NOT NULL,
           operation TEXT NOT NULL,
           source_path TEXT,
           destination_path TEXT,
           backup_path TEXT,
           recovered_rows INTEGER NOT NULL DEFAULT 0,
           lost_rows INTEGER NOT NULL DEFAULT 0,
           detail TEXT NOT NULL
         );
         CREATE TABLE network_action_audit (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           occurred_at_ms INTEGER NOT NULL,
           intent TEXT NOT NULL,
           destination_class TEXT NOT NULL,
           consent_id TEXT,
           allowed INTEGER NOT NULL,
           bytes_sent INTEGER NOT NULL DEFAULT 0,
           bytes_received INTEGER NOT NULL DEFAULT 0,
           detail TEXT NOT NULL
         );",
    )?;
    transaction.execute_batch(V5_PRODUCT_TABLES_SQL)
}

fn migration_v6(transaction: &Transaction<'_>) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(
        "ALTER TABLE traffic_samples
           ADD COLUMN sample_interval_ms INTEGER NOT NULL DEFAULT 1000;",
    )?;
    transaction.execute_batch(V6_TRAFFIC_ROLLUPS_SQL)
}

fn migration_v7(transaction: &Transaction<'_>) -> Result<(), rusqlite::Error> {
    transaction.execute_batch("ALTER TABLE connections ADD COLUMN executable_revision TEXT;")?;
    transaction.execute_batch(V7_TODAY_INDEXES_SQL)
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

fn validate_investigation(investigation: &SavedInvestigation) -> Result<(), StorageError> {
    if investigation.id.is_empty()
        || investigation.id.len() > 128
        || !investigation
            .id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
    {
        return Err(StorageError::InvalidInvestigation(
            "id must contain 1 to 128 ASCII letters, numbers, '-' or '_'".into(),
        ));
    }
    let name = investigation.name.trim();
    if name.is_empty()
        || name.len() > 120
        || investigation.notes.len() > 65_536
        || investigation.tags.len() > 256
        || investigation.tags.iter().any(|tag| tag.len() > 128)
        || investigation.layout_json.len() > 262_144
    {
        return Err(StorageError::InvalidInvestigation(
            "name, notes, tags or workspace layout exceeds its local safety limit".into(),
        ));
    }
    if let Some(filter) = investigation
        .query
        .filter
        .as_deref()
        .filter(|filter| !filter.trim().is_empty())
    {
        filter
            .parse::<ConnectionFilter>()
            .map_err(|error| StorageError::InvalidInvestigation(error.to_string()))?;
    }
    Ok(())
}

fn notification_state_name(state: NotificationState) -> &'static str {
    match state {
        NotificationState::Unread => "unread",
        NotificationState::Read => "read",
        NotificationState::Pinned => "pinned",
        NotificationState::Resolved => "resolved",
    }
}

fn apply_export_redaction(entries: &mut [HistoryEntry], redaction: &ExportRedaction) {
    for entry in entries {
        let connection = &mut entry.connection;
        if redaction.local_ip {
            connection.key.local.address = match connection.key.local.observed_family {
                sabicheiro_core::AddressFamily::Ipv4 => std::net::Ipv4Addr::UNSPECIFIED.into(),
                sabicheiro_core::AddressFamily::Ipv6 => std::net::Ipv6Addr::UNSPECIFIED.into(),
            };
            connection.key.local.scope = sabicheiro_core::AddressScope::Unspecified;
            connection.key.local.zone = None;
        }
        if redaction.executable_path {
            connection.process.executable_path = None;
        } else if redaction.user_names {
            connection.process.executable_path = connection
                .process
                .executable_path
                .as_deref()
                .map(redact_user_path);
        }
        if redaction.user_names {
            connection.process.executable = connection
                .process
                .executable
                .as_deref()
                .map(redact_user_path);
        }
    }
}

fn redact_user_path(value: &str) -> String {
    let normalized = value.replace('/', "\\");
    let lower = normalized.to_ascii_lowercase();
    for marker in ["\\users\\", "\\home\\"] {
        if let Some(start) = lower.find(marker) {
            let user_start = start + marker.len();
            let user_end = normalized[user_start..]
                .find('\\')
                .map_or(normalized.len(), |offset| user_start + offset);
            let mut redacted = normalized.clone();
            redacted.replace_range(user_start..user_end, "[redacted]");
            return if value.contains('/') {
                redacted.replace('\\', "/")
            } else {
                redacted
            };
        }
    }
    normalized
}

fn export_csv(entries: &[HistoryEntry]) -> String {
    let mut output = String::from(
        "session_id,first_seen,last_seen,process,pid,process_instance,protocol,direction,state,local_endpoint,remote_endpoint,remote_scope,endpoint_role,role_reason,observation_source,correlation_quality,interface_id,capture_generation,flow_generation,domain,country,region,city,geo_precision,placement,geo_source,geo_dataset_edition,geo_dataset_build_epoch_ms,geo_dataset_sha256,geo_matched_prefix,city_generation,asn,asn_organization,asn_dataset_edition,asn_dataset_build_epoch_ms,asn_dataset_sha256,asn_matched_prefix,asn_generation,bytes_down,bytes_up,packets_down,packets_up\n",
    );
    for entry in entries {
        let connection = &entry.connection;
        let geo = connection.geo.as_ref();
        let asn = connection.asn.as_ref();
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
            connection.key.process_instance.clone().unwrap_or_default(),
            connection.protocol.as_str().into(),
            format!("{:?}", connection.direction).to_lowercase(),
            format!("{:?}", connection.state).to_lowercase(),
            connection.key.local.label(),
            connection.key.remote.label(),
            format!("{:?}", connection.key.remote.scope).to_lowercase(),
            format!("{:?}", connection.evidence.endpoint_role).to_lowercase(),
            connection.evidence.role_reason.clone(),
            format!("{:?}", connection.evidence.observation_source).to_lowercase(),
            format!("{:?}", connection.evidence.correlation_quality).to_lowercase(),
            connection.key.interface_id.clone().unwrap_or_default(),
            connection.key.capture_generation.to_string(),
            connection.key.flow_generation.to_string(),
            connection
                .domain
                .clone()
                .or_else(|| connection.hostname.clone())
                .unwrap_or_default(),
            geo.and_then(|value| value.country_code.clone())
                .unwrap_or_default(),
            geo.and_then(|value| value.region.clone())
                .unwrap_or_default(),
            geo.and_then(|value| value.city.clone()).unwrap_or_default(),
            geo.map(|value| format!("{:?}", value.precision).to_lowercase())
                .unwrap_or_default(),
            geo.map(|value| format!("{:?}", value.placement).to_lowercase())
                .unwrap_or_default(),
            geo.map(|value| value.source.clone()).unwrap_or_default(),
            geo.and_then(|value| value.dataset_edition.clone())
                .unwrap_or_default(),
            geo.and_then(|value| value.dataset_build_epoch_ms)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            geo.and_then(|value| value.dataset_sha256.clone())
                .unwrap_or_default(),
            geo.and_then(|value| value.matched_prefix)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            geo.map(|value| value.city_generation.to_string())
                .unwrap_or_default(),
            asn.map(|value| value.number.to_string())
                .unwrap_or_default(),
            asn.and_then(|value| value.organization.clone())
                .unwrap_or_default(),
            asn.and_then(|value| value.dataset_edition.clone())
                .unwrap_or_default(),
            asn.and_then(|value| value.dataset_build_epoch_ms)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            asn.and_then(|value| value.dataset_sha256.clone())
                .unwrap_or_default(),
            asn.and_then(|value| value.matched_prefix)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            asn.map(|value| value.generation.to_string())
                .unwrap_or_default(),
            connection.traffic.bytes_received.to_string(),
            connection.traffic.bytes_sent.to_string(),
            connection.traffic.packets_received.to_string(),
            connection.traffic.packets_sent.to_string(),
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

fn serialized_name(value: &impl serde::Serialize) -> Result<String, serde_json::Error> {
    serde_json::to_string(value).map(|encoded| encoded.trim_matches('"').to_owned())
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
    use sabicheiro_core::{
        ClockQuality, ConnectionEvidence, ConnectionKey, ConnectionState, Direction, EvidenceClock,
        EvidenceObservation, LocalRuleAction, LocalRuleCondition, NetworkEventKind,
        NetworkProtocol, NormalizedNetworkEvent, ObservationSource, ProviderCursor, RuleContext,
        TimelineResolution,
    };

    fn sample_connection() -> Connection {
        let mut connection = Connection::placeholder(1_000);
        connection.id = "connection-1".into();
        connection.process.name = "A, Browser".into();
        connection.process.pid = Some(7);
        connection.protocol = NetworkProtocol::Tcp;
        connection.key.protocol = NetworkProtocol::Tcp;
        connection.key.remote =
            sabicheiro_core::Endpoint::new("203.0.113.10".parse().expect("remote IP"), 443);
        connection.direction = Direction::Outbound;
        connection.state = ConnectionState::Established;
        connection.active = true;
        connection.last_seen_ms = 2_000;
        connection
    }

    fn sample_observation(sequence: u64, utc_ms: i64) -> EvidenceObservation {
        let connection = sample_connection();
        EvidenceObservation::network(
            ProviderCursor {
                provider_id: "fixture.events".into(),
                generation: 1,
                sequence,
                watermark_monotonic_ns: Some(sequence.saturating_mul(1_000)),
                dropped_before: 0,
            },
            EvidenceClock {
                clock_id: "fixture-clock".into(),
                monotonic_ns: sequence.saturating_mul(1_000),
                utc_ms,
                quality: ClockQuality::Anchored,
            },
            NormalizedNetworkEvent {
                kind: NetworkEventKind::Opened,
                key: ConnectionKey {
                    pid: connection.process.pid,
                    process_instance: Some("pid:7:start:1".into()),
                    local: connection.key.local,
                    remote: connection.key.remote.clone(),
                    protocol: NetworkProtocol::Tcp,
                    interface_id: Some("fixture-route".into()),
                    capture_generation: 1,
                    flow_generation: sequence,
                },
                evidence: ConnectionEvidence::for_peer(
                    &connection.key.remote,
                    ConnectionState::Established,
                    ObservationSource::Demo,
                    utc_ms,
                ),
                timestamp_ms: utc_ms,
                process: connection.process,
                direction: Direction::Outbound,
                state: ConnectionState::Established,
                interface: Some("fixture-route".into()),
                hostname: Some("fixture.example".into()),
                domain: Some("fixture.example".into()),
                geo: None,
                asn: None,
                traffic_delta: TrafficCounters::default(),
                counter_source: CounterSource::Unavailable,
            },
            [],
        )
    }

    #[test]
    fn today_novelty_uses_all_retained_history_beyond_latest_five_hundred() {
        let mut storage = Storage::in_memory().expect("storage");
        storage
            .start_session("historical", 1_000)
            .expect("historical session");

        let mut known = sample_connection();
        known.id = "known-outside-page".into();
        known.process.name = "Known Browser".into();
        known.domain = Some("known.example".into());
        known.process.executable_revision_id = Some("revision-known".into());
        known.asn = Some(sabicheiro_core::AsnInfo {
            number: 64_500,
            ..sabicheiro_core::AsnInfo::default()
        });
        known.geo = Some(sabicheiro_core::GeoLocation {
            country_code: Some("ES".into()),
            country_name: Some("Spain".into()),
            ..sabicheiro_core::GeoLocation::default()
        });
        storage
            .save_connections("historical", std::slice::from_ref(&known))
            .expect("known history");

        let fillers = (0..501)
            .map(|index| {
                let mut connection = sample_connection();
                connection.id = format!("newer-{index:03}");
                connection.process.name = format!("Filler {index:03}");
                connection.last_seen_ms = 10_000 + index;
                connection
            })
            .collect::<Vec<_>>();
        storage
            .save_connections("historical", &fillers)
            .expect("newer history");

        let mut current_known = known.clone();
        current_known.id = "current-known".into();
        current_known.last_seen_ms = 50_000;
        let mut genuinely_new = sample_connection();
        genuinely_new.id = "current-new".into();
        genuinely_new.process.name = "Genuinely New".into();
        genuinely_new.key.remote.address = "198.51.100.77".parse().expect("new IP");
        genuinely_new.domain = Some("new.example".into());
        genuinely_new.process.executable_revision_id = Some("revision-new".into());
        genuinely_new.asn = Some(sabicheiro_core::AsnInfo {
            number: 64_501,
            ..sabicheiro_core::AsnInfo::default()
        });
        genuinely_new.geo = Some(sabicheiro_core::GeoLocation {
            country_code: Some("PT".into()),
            country_name: Some("Portugal".into()),
            ..sabicheiro_core::GeoLocation::default()
        });

        let groups = storage
            .today_novelty(Some("current"), &[current_known, genuinely_new], 3)
            .expect("today novelty");
        for kind in [
            TodayChangeKind::Process,
            TodayChangeKind::ApplicationDestination,
            TodayChangeKind::Domain,
            TodayChangeKind::Asn,
            TodayChangeKind::Country,
            TodayChangeKind::ExecutableRevision,
        ] {
            let group = groups
                .iter()
                .find(|group| group.kind == kind)
                .expect("novelty category");
            assert_eq!(group.count, 1, "unexpected count for {kind:?}");
            assert_eq!(group.examples.len(), 1);
            assert_eq!(
                group.examples[0].connection_id.as_deref(),
                Some("current-new")
            );
        }
    }

    #[test]
    fn immutable_evidence_journal_is_idempotent_and_cursor_safe() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        let observation = sample_observation(1, 1_000);
        assert_eq!(
            storage
                .append_observations(Some("s1"), std::slice::from_ref(&observation))
                .expect("append"),
            1
        );
        assert_eq!(
            storage
                .append_observations(Some("s1"), std::slice::from_ref(&observation))
                .expect("idempotent retry"),
            0
        );
        let loaded = storage
            .query_observations(&EvidenceQuery::default())
            .expect("journal");
        assert_eq!(loaded, vec![observation.clone()]);
        assert!(
            storage
                .connection
                .execute(
                    "UPDATE evidence_journal SET utc_ms=2000 WHERE id=?1",
                    params![observation.id],
                )
                .is_err()
        );
        let mut conflict = observation;
        conflict.id = "conflicting-id".into();
        assert!(matches!(
            storage.append_observations(Some("s1"), &[conflict]),
            Err(StorageError::InvalidEvidence(_))
        ));
        let stats = storage.evidence_stats().expect("stats");
        assert_eq!(stats.observations, 1);
        assert_eq!(stats.providers, 1);
    }

    #[test]
    fn tiered_retention_estimates_rolls_up_and_deletes_in_batches() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        storage
            .append_observations(Some("s1"), &[sample_observation(1, 1_000)])
            .expect("journal");
        storage
            .save_connections("s1", &[sample_connection()])
            .expect("connection");
        let estimate = storage.retention_estimate(3_000).expect("estimate");
        assert_eq!(estimate.rows_by_tier[&RetentionTier::DetailedEvidence], 1);
        assert_eq!(estimate.rows_by_tier[&RetentionTier::Connections], 1);

        let cancelled = storage
            .apply_retention_batch(RetentionTier::DetailedEvidence, 3_000, 100, true)
            .expect("cancel");
        assert!(cancelled.cancelled);
        assert_eq!(storage.evidence_stats().expect("stats").observations, 1);

        let evidence = storage
            .apply_retention_batch(RetentionTier::DetailedEvidence, 3_000, 100, false)
            .expect("delete evidence");
        assert_eq!(
            evidence.removed_by_tier[&RetentionTier::DetailedEvidence],
            1
        );
        let connections = storage
            .apply_retention_batch(RetentionTier::Connections, 3_000, 100, false)
            .expect("roll up connections");
        assert_eq!(connections.removed_by_tier[&RetentionTier::Connections], 1);
        let rollups = storage
            .connection
            .query_row("SELECT SUM(connections) FROM evidence_rollups", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("rollup count");
        assert_eq!(rollups, 1);
    }

    #[test]
    fn fts_search_keeps_text_local_and_returns_stable_references() {
        let mut storage = Storage::in_memory().expect("storage");
        storage
            .index_search_document(
                "investigation:1",
                "investigation",
                "Browser update",
                "A new destination appeared after update",
                "review pinned",
                "fixture.example 203.0.113.10",
            )
            .expect("index");
        assert_eq!(
            storage.search("new destination", 10).expect("search"),
            vec!["investigation:1"]
        );
        storage
            .index_search_document(
                "investigation:1",
                "investigation",
                "Browser update",
                "Resolved locally",
                "done",
                "fixture.example",
            )
            .expect("replace index");
        assert!(
            storage
                .search("new destination", 10)
                .expect("search")
                .is_empty()
        );
    }

    #[test]
    fn projection_hash_is_stable_and_recorded_with_version() {
        let storage = Storage::in_memory().expect("storage");
        let value = vec![sample_observation(1, 1_000), sample_observation(2, 2_000)];
        let first = projection_sha256(&value).expect("hash");
        let second = projection_sha256(&value).expect("hash again");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        storage
            .record_projection_run(
                "connections",
                1,
                Some(&value[0].id),
                Some(&value[1].id),
                value.len(),
                &first,
                3_000,
            )
            .expect("record projection");
        let count = storage
            .connection
            .query_row("SELECT COUNT(*) FROM projection_runs", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("projection count");
        assert_eq!(count, 1);
    }

    #[test]
    fn backup_and_salvage_never_overwrite_the_original() {
        let directory = tempfile::tempdir().expect("tempdir");
        let source = directory.path().join("active.sqlite3");
        let backup = directory.path().join("active-manual.bak");
        let salvage_backup = directory.path().join("damaged-original.bak");
        let recovered_path = directory.path().join("recovered.sqlite3");
        {
            let mut storage = Storage::open(&source).expect("storage");
            storage.start_session("s1", 1_000).expect("session");
            storage
                .append_observations(Some("s1"), &[sample_observation(1, 1_000)])
                .expect("journal");
            let written = storage.backup_to(&backup).expect("backup");
            assert_eq!(
                fs::canonicalize(written).expect("canonical written backup"),
                fs::canonicalize(&backup).expect("canonical requested backup")
            );
        }
        assert_eq!(
            Storage::read_only_quick_check(&backup).expect("check"),
            "ok"
        );
        let damaged = SqliteConnection::open(&source).expect("open source");
        damaged
            .execute("DROP TABLE traffic_samples", [])
            .expect("simulate partial schema damage");
        drop(damaged);

        let report =
            Storage::salvage_database(&source, &recovered_path, &salvage_backup).expect("salvage");
        assert!(report.quick_check_ok);
        assert!(report.recovered_tables.contains(&"sessions".into()));
        assert!(report.recovered_tables.contains(&"evidence_journal".into()));
        assert!(report.skipped_tables.contains(&"traffic_samples".into()));
        assert!(source.is_file());
        assert!(salvage_backup.is_file());
        let recovered = Storage::open(&recovered_path).expect("open recovered");
        assert_eq!(recovered.evidence_stats().expect("stats").observations, 1);
        assert!(Storage::salvage_database(&source, &recovered_path, &backup).is_err());
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
    fn new_database_contains_no_example_or_demo_rows() {
        let storage = Storage::in_memory().expect("storage");
        assert!(storage.list_sessions(100).expect("sessions").is_empty());
        assert!(
            storage
                .query_history(&HistoryQuery::default())
                .expect("connections")
                .is_empty()
        );
        assert_eq!(storage.evidence_stats().expect("evidence").observations, 0);
        assert!(
            storage
                .list_investigations()
                .expect("investigations")
                .is_empty()
        );
        assert!(storage.list_local_rules().expect("rules").is_empty());
        assert!(
            storage
                .list_notifications(100)
                .expect("notifications")
                .is_empty()
        );
    }

    #[test]
    fn history_cursor_pages_are_stable_bounded_and_complete() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("paged", 1_000).expect("session");
        let connections = (0..1_205)
            .map(|index| {
                let mut connection = sample_connection();
                connection.id = format!("paged-{index:06}");
                connection.first_seen_ms = 1_000 + index;
                connection.last_seen_ms = 2_000 + index;
                connection.process.name = if index % 2 == 0 {
                    "Browser".into()
                } else {
                    "Updater".into()
                };
                connection
            })
            .collect::<Vec<_>>();
        for chunk in connections.chunks(250) {
            storage
                .save_connections("paged", chunk)
                .expect("save history chunk");
        }

        let mut cursor = None;
        let mut ids = std::collections::HashSet::new();
        let mut pages = 0;
        loop {
            let page = storage
                .query_history_page(&HistoryQuery {
                    session_id: Some("paged".into()),
                    limit: 500,
                    cursor,
                    ..HistoryQuery::default()
                })
                .expect("history page");
            if pages == 0 {
                assert!(serde_json::to_vec(&page.entries).expect("page JSON").len() < 1_048_576);
            }
            for entry in &page.entries {
                assert!(
                    ids.insert(entry.connection.id.clone()),
                    "duplicate cursor row"
                );
            }
            pages += 1;
            if !page.has_more {
                assert!(page.next_cursor.is_none());
                break;
            }
            cursor = page.next_cursor;
        }
        assert_eq!(pages, 3);
        assert_eq!(ids.len(), connections.len());
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
                    sample_interval_ms: 1_000,
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
                    sample_interval_ms: 1_000,
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
                    sample_interval_ms: 1_000,
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
    fn normal_eight_hour_samples_compact_while_high_fidelity_stays_raw() {
        const HOUR_MS: i64 = 3_600_000;
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("normal", 0).expect("normal session");
        storage
            .start_session("precise", 0)
            .expect("precise session");
        let normal = (0..(8 * 60 * 6))
            .map(|index| TrafficSample {
                timestamp_ms: i64::from(index) * 10_000,
                session_id: Some("normal".into()),
                process_key: None,
                sample_interval_ms: 10_000,
                counters: TrafficCounters {
                    bytes_sent: 10,
                    bytes_received: 20,
                    packets_sent: 1,
                    packets_received: 2,
                },
                rates: TransferRates {
                    down_bytes_per_second: 2.0,
                    up_bytes_per_second: 1.0,
                    packets_per_second: 0.3,
                },
                source: CounterSource::InterfaceAggregate,
            })
            .collect::<Vec<_>>();
        for chunk in normal.chunks(500) {
            storage.save_traffic_samples(chunk).expect("normal samples");
        }
        let precise = (0..120)
            .map(|index| TrafficSample {
                timestamp_ms: i64::from(index) * 1_000,
                session_id: Some("precise".into()),
                process_key: None,
                sample_interval_ms: 1_000,
                counters: TrafficCounters::default(),
                rates: TransferRates::default(),
                source: CounterSource::InterfaceAggregate,
            })
            .collect::<Vec<_>>();
        storage
            .save_traffic_samples(&precise)
            .expect("precise samples");

        storage
            .compact_traffic_samples(48 * HOUR_MS)
            .expect("compact samples");
        let normal_rows = storage
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM traffic_samples WHERE session_id='normal') +
                   (SELECT COUNT(*) FROM traffic_rollups WHERE session_id='normal')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("normal row count");
        let precise_rows = storage
            .connection
            .query_row(
                "SELECT COUNT(*) FROM traffic_samples WHERE session_id='precise'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("precise row count");
        assert!(normal_rows <= 24, "normal row count was {normal_rows}");
        assert_eq!(precise_rows, 120);

        let timeline = storage
            .query_timeline(&TimelineQuery {
                from_ms: 0,
                to_ms: 8 * HOUR_MS,
                session_id: Some("normal".into()),
                process_key: None,
                resolution: TimelineResolution::OneHour,
            })
            .expect("compacted timeline");
        assert_eq!(timeline.len(), 8);
        assert_eq!(
            timeline
                .iter()
                .map(|sample| sample.counters.bytes_received)
                .sum::<u64>(),
            normal.len() as u64 * 20
        );
        assert!(timeline.iter().all(|sample| {
            (sample.rates.down_bytes_per_second - 2.0).abs() < f64::EPSILON
                && sample.sample_interval_ms == HOUR_MS as u64
        }));
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
                redaction: ExportRedaction::default(),
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
    fn product_state_roundtrips_with_rule_versions_and_bounded_audits() {
        use std::collections::BTreeSet;

        let storage = Storage::in_memory().expect("storage");
        let mut rule = LocalRule {
            id: "new-https".into(),
            version: 1,
            name: "Review new HTTPS destinations".into(),
            enabled: true,
            match_all: true,
            conditions: vec![LocalRuleCondition::Port(443)],
            actions: BTreeSet::from([
                LocalRuleAction::Notify,
                LocalRuleAction::Pin,
                LocalRuleAction::Tag("review".into()),
            ]),
            cooldown_seconds: 60,
            created_at_ms: 1_000,
            updated_at_ms: 1_000,
        };
        storage.save_local_rule(&rule).expect("save rule");
        let mut invalid_edit = rule.clone();
        invalid_edit.name = "Changed without a version".into();
        assert!(storage.save_local_rule(&invalid_edit).is_err());
        rule.version = 2;
        rule.name = "Review HTTPS destinations".into();
        rule.updated_at_ms = 2_000;
        storage.save_local_rule(&rule).expect("save version two");
        assert_eq!(
            storage.list_local_rules().expect("rules"),
            vec![rule.clone()]
        );
        assert_eq!(
            storage
                .list_local_rule_versions(&rule.id)
                .expect("versions")
                .len(),
            2
        );

        let matched = rule.evaluate(&sample_connection(), &RuleContext::default());
        assert!(
            storage
                .record_rule_match(&matched, 2_100)
                .expect("record match")
        );
        assert_eq!(
            storage.list_rule_matches(10).expect("matches"),
            vec![matched]
        );
        let annotation = storage
            .apply_rule_match_actions(&storage.list_rule_matches(1).expect("match")[0], 2_100)
            .expect("apply safe actions")
            .expect("annotation");
        assert!(annotation.pinned);
        assert!(annotation.tags.contains("review"));

        let notification = LocalNotificationRecord {
            id: "notification-1".into(),
            rule_id: rule.id.clone(),
            rule_version: rule.version,
            evidence_ids: vec!["connection-1".into()],
            network_profile_id: Some("home".into()),
            title: "Review".into(),
            detail: "A local rule matched".into(),
            group_key: "rule:home:connection-1".into(),
            count: 1,
            first_at_ms: 2_100,
            last_at_ms: 2_100,
            snoozed_until_ms: None,
            state: NotificationState::Unread,
        };
        storage
            .save_notification(&notification)
            .expect("save notification");
        assert!(storage
            .update_notification_state(
                &notification.id,
                NotificationState::Pinned,
                Some(4_000),
            )
            .expect("update notification"));
        let notifications = storage.list_notifications(10).expect("notifications");
        assert_eq!(notifications[0].state, NotificationState::Pinned);
        assert_eq!(notifications[0].snoozed_until_ms, Some(4_000));

        let profile = NetworkProfile::pseudonymous(
            "sensitive Wi-Fi and gateway signature",
            Some("Home".into()),
            false,
            1_000,
        );
        storage
            .save_network_profile(&profile, 2_000)
            .expect("save profile");
        assert_eq!(
            storage.list_network_profiles().expect("profiles"),
            vec![profile]
        );

        let card = TodayCardState {
            id: "new-destination:connection-1".into(),
            dismissed: false,
            pinned: true,
            updated_at_ms: 2_200,
        };
        storage
            .save_today_card_state(&card)
            .expect("save card state");
        assert_eq!(storage.list_today_card_states().expect("cards"), vec![card]);

        storage.start_session("s1", 1_000).expect("session");
        let listener = ListenerExposure::classify(
            sabicheiro_core::Endpoint::new("127.0.0.1".parse().expect("IP"), 8080),
            NetworkProtocol::Tcp,
            Some("fixture:1".into()),
            false,
            2_300,
        );
        let snapshot = storage
            .save_listener_snapshot(Some("s1"), 2_300, &[listener])
            .expect("listener snapshot");
        assert_eq!(
            storage
                .list_listener_snapshots(10)
                .expect("listener snapshots"),
            vec![snapshot]
        );

        let audit = HandshakeModeAudit {
            occurred_at_ms: 2_400,
            enabled: true,
            maximum_bytes: 16_384,
            maximum_packets: 8,
            duration_limit_ms: 5_000,
            detail: "explicit local opt-in".into(),
        };
        storage
            .record_handshake_mode_audit(&audit)
            .expect("audit handshake mode");
        assert_eq!(
            storage
                .list_handshake_mode_audit(10)
                .expect("handshake audit"),
            vec![audit]
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
            process_instance: None,
            local: Endpoint::new("127.0.0.1".parse().expect("local"), 50_000),
            remote: Endpoint::new("1.1.1.1".parse().expect("remote"), 443),
            protocol: NetworkProtocol::Tcp,
            interface_id: Some("test".into()),
            capture_generation: 1,
            flow_generation: 1,
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

    fn create_legacy_database(path: &Path, version: i64) {
        let mut connection = SqliteConnection::open(path).expect("legacy database");
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (
                   version INTEGER PRIMARY KEY,
                   applied_at_ms INTEGER NOT NULL
                 );",
            )
            .expect("migration table");
        for candidate in 1..=version {
            let transaction = connection.transaction().expect("legacy transaction");
            match candidate {
                1 => migration_v1(&transaction).expect("v1"),
                2 => migration_v2(&transaction).expect("v2"),
                3 => migration_v3(&transaction).expect("v3"),
                4 => migration_v4(&transaction).expect("v4"),
                5 => migration_v5(&transaction).expect("v5"),
                6 => migration_v6(&transaction).expect("v6"),
                _ => unreachable!("supported legacy schema"),
            }
            transaction
                .execute(
                    "INSERT INTO schema_migrations(version, applied_at_ms) VALUES (?1, ?2)",
                    params![candidate, candidate * 1_000],
                )
                .expect("version marker");
            transaction.commit().expect("legacy commit");
        }
    }

    #[test]
    fn every_file_backed_legacy_version_creates_a_readable_pre_v4_backup() {
        for version in 1..=3 {
            let directory = tempfile::tempdir().expect("tempdir");
            let path = directory.path().join(format!("legacy-v{version}.sqlite3"));
            create_legacy_database(&path, version);
            let storage = Storage::open(&path).expect("migrate legacy database");
            let migrated_version = storage
                .connection
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("migrated version");
            assert_eq!(migrated_version, SCHEMA_VERSION);
            drop(storage);

            let backup = path.with_extension(format!("pre-v4-from-v{version}.bak"));
            assert!(backup.is_file());
            let backup_connection = SqliteConnection::open(backup).expect("open backup");
            let backup_version = backup_connection
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("backup version");
            assert_eq!(backup_version, version);
        }
    }

    #[test]
    fn every_legacy_schema_creates_a_readable_pre_v5_backup() {
        for version in 1..=4 {
            let directory = tempfile::tempdir().expect("tempdir");
            let path = directory.path().join(format!("legacy-v{version}.sqlite3"));
            create_legacy_database(&path, version);
            let storage = Storage::open(&path).expect("migrate legacy database");
            drop(storage);
            let backup = path.with_extension(format!("pre-v5-from-v{version}.bak"));
            assert!(backup.is_file());
            let backup_connection = SqliteConnection::open(backup).expect("open backup");
            let backup_version = backup_connection
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("backup version");
            assert_eq!(backup_version, version);
        }
    }

    #[test]
    fn every_legacy_schema_creates_a_readable_pre_v6_backup() {
        for version in 1..=5 {
            let directory = tempfile::tempdir().expect("tempdir");
            let path = directory.path().join(format!("legacy-v{version}.sqlite3"));
            create_legacy_database(&path, version);
            let storage = Storage::open(&path).expect("migrate legacy database");
            assert_eq!(
                storage
                    .connection
                    .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .expect("schema version"),
                SCHEMA_VERSION
            );
            drop(storage);
            let backup = path.with_extension(format!("pre-v6-from-v{version}.bak"));
            assert!(backup.is_file());
            let backup_connection = SqliteConnection::open(backup).expect("open backup");
            assert_eq!(
                backup_connection
                    .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .expect("backup version"),
                version
            );
        }
    }

    #[test]
    fn every_legacy_schema_creates_a_readable_pre_v7_backup() {
        for version in 1..=6 {
            let directory = tempfile::tempdir().expect("tempdir");
            let path = directory.path().join(format!("legacy-v{version}.sqlite3"));
            create_legacy_database(&path, version);
            let storage = Storage::open(&path).expect("migrate legacy database");
            assert_eq!(
                storage
                    .connection
                    .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .expect("schema version"),
                SCHEMA_VERSION
            );
            drop(storage);
            let backup = path.with_extension(format!("pre-v7-from-v{version}.bak"));
            assert!(backup.is_file());
            let backup_connection = SqliteConnection::open(backup).expect("open backup");
            assert_eq!(
                backup_connection
                    .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .expect("backup version"),
                version
            );
        }
    }

    #[test]
    fn failed_v5_migration_rolls_back_and_preserves_the_v4_backup() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("migration-v5-failure.sqlite3");
        create_legacy_database(&path, 4);
        let connection = SqliteConnection::open(&path).expect("legacy database");
        connection
            .execute("CREATE TABLE evidence_journal(id TEXT PRIMARY KEY)", [])
            .expect("inject migration conflict");
        drop(connection);
        assert!(Storage::open(&path).is_err());
        assert!(path.with_extension("pre-v5-from-v4.bak").is_file());
        let connection = SqliteConnection::open(&path).expect("database remains readable");
        let version = connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("schema version");
        assert_eq!(version, 4);
        assert!(
            connection
                .prepare("SELECT utc_ms FROM evidence_journal")
                .is_err()
        );
    }

    #[test]
    fn failed_v4_migration_rolls_back_schema_and_keeps_the_backup() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("migration-failure.sqlite3");
        create_legacy_database(&path, 3);
        let connection = SqliteConnection::open(&path).expect("legacy database");
        connection
            .execute_batch(
                "ALTER TABLE connections ADD COLUMN endpoint_role TEXT NOT NULL DEFAULT 'unknown';",
            )
            .expect("inject migration conflict");
        drop(connection);

        assert!(Storage::open(&path).is_err());
        assert!(path.with_extension("pre-v4-from-v3.bak").is_file());
        let connection = SqliteConnection::open(&path).expect("failed database remains readable");
        let version = connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("schema version");
        assert_eq!(version, 3);
        let has_remote_scope = connection
            .prepare("SELECT remote_scope FROM connections")
            .is_ok();
        assert!(!has_remote_scope, "partial migration must be rolled back");
    }

    #[test]
    fn investigations_roundtrip_and_reconstruct_the_same_connection_set() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        storage
            .save_connections("s1", &[sample_connection()])
            .expect("save connection");
        let investigation = SavedInvestigation {
            id: "investigation-1".into(),
            name: "Documentation destination".into(),
            query: InvestigationQuery {
                filter: Some("cidr:203.0.113.0/24 source:unknown".into()),
                session_id: Some("s1".into()),
                ..InvestigationQuery::default()
            },
            notes: "Local-only note".into(),
            tags: vec!["review".into()],
            selected_connection_id: Some("connection-1".into()),
            layout_json: r#"{"inspector":"open"}"#.into(),
            autosave_revision: 3,
            created_at_ms: 2_000,
            updated_at_ms: 2_000,
        };
        storage
            .save_investigation(&investigation)
            .expect("save investigation");
        assert_eq!(
            storage.list_investigations().expect("list"),
            vec![investigation.clone()]
        );
        let reconstructed = storage
            .query_investigation(&investigation.query)
            .expect("reconstruct investigation");
        assert_eq!(reconstructed.len(), 1);
        assert_eq!(reconstructed[0].connection.id, "connection-1");
        assert!(
            storage
                .delete_investigation(&investigation.id)
                .expect("delete")
        );
    }

    #[test]
    fn derived_geo_views_replace_atomically_without_overwriting_observed_fields() {
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        let original = sample_connection();
        storage
            .save_connections("s1", std::slice::from_ref(&original))
            .expect("save original");
        let first = DerivedGeoEnrichment {
            connection_id: original.id.clone(),
            observed_geo: None,
            observed_asn: None,
            current_geo: Some(sabicheiro_core::GeoLocation {
                country_code: Some("AA".into()),
                city_generation: 2,
                ..Default::default()
            }),
            current_asn: None,
        };
        storage
            .replace_geo_views("s1", "current", std::slice::from_ref(&first), 2, 1, 3_000)
            .expect("first view");
        assert_eq!(
            storage.load_geo_views("s1", "current").expect("first load"),
            vec![first]
        );
        let second = DerivedGeoEnrichment {
            connection_id: original.id.clone(),
            observed_geo: None,
            observed_asn: None,
            current_geo: Some(sabicheiro_core::GeoLocation {
                country_code: Some("BB".into()),
                city_generation: 3,
                ..Default::default()
            }),
            current_asn: None,
        };
        storage
            .replace_geo_views("s1", "current", std::slice::from_ref(&second), 3, 1, 4_000)
            .expect("replacement view");
        assert_eq!(
            storage
                .load_geo_views("s1", "current")
                .expect("second load"),
            vec![second]
        );
        assert!(
            storage
                .query_history(&HistoryQuery::default())
                .expect("observed history")[0]
                .connection
                .geo
                .is_none()
        );
    }

    #[test]
    fn read_only_open_does_not_create_or_upgrade() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing").join("database.sqlite3");
        assert!(Storage::open_read_only(&missing).is_err());
        assert!(!missing.parent().unwrap().exists());
        let legacy = directory.path().join("legacy.sqlite3");
        create_legacy_database(&legacy, 3);
        let storage = Storage::open_read_only(&legacy).unwrap();
        assert!(
            storage
                .connection
                .execute("CREATE TABLE unwanted(id INTEGER)", [])
                .is_err()
        );
        assert_eq!(
            storage
                .connection
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            3
        );
        drop(storage);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        let current = directory.path().join("current.sqlite3");
        let mut writer = Storage::open(&current).unwrap();
        writer.start_session("s1", 0).unwrap();
        writer
            .save_connections("s1", &[sample_connection()])
            .unwrap();
        let reader = Storage::open_read_only(&current).unwrap();
        assert_eq!(
            reader
                .query_history(&HistoryQuery::default())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn export_pages_all_rows_and_finds_old_process() {
        let directory = tempfile::tempdir().unwrap();
        let mut storage = Storage::in_memory().unwrap();
        storage.start_session("s1", 0).unwrap();
        let mut old = sample_connection();
        old.process.name = "old-only".into();
        old.key.local = sabicheiro_core::Endpoint::new("192.168.1.44".parse().unwrap(), 50_000);
        old.last_seen_ms = 1;
        storage.save_connections("s1", &[old]).unwrap();
        for batch in 0..21 {
            let rows: Vec<_> = (0..5_000)
                .map(|i| {
                    let mut row = sample_connection();
                    row.id = format!("row-{}", batch * 5_000 + i);
                    row.last_seen_ms = 2;
                    row
                })
                .collect();
            storage.save_connections("s1", &rows).unwrap();
        }
        for format in [ExportFormat::Json, ExportFormat::Csv] {
            let extension = match format {
                ExportFormat::Json => "json",
                ExportFormat::Csv => "csv",
            };
            let path = directory.path().join(format!("all.{extension}"));
            let mut request = ExportRequest {
                format,
                path: path.to_string_lossy().into_owned(),
                session_id: None,
                process_key: None,
                from_ms: None,
                to_ms: None,
                query: None,
                redaction: ExportRedaction {
                    local_ip: true,
                    ..Default::default()
                },
            };
            assert_eq!(storage.export(&request).unwrap().records, 105_001);
            let output = fs::read_to_string(&path).unwrap();
            assert!(!output.contains("192.168.1.44"));
            match format {
                ExportFormat::Json => assert_eq!(
                    serde_json::from_str::<Vec<HistoryEntry>>(&output)
                        .unwrap()
                        .len(),
                    105_001
                ),
                ExportFormat::Csv => assert_eq!(output.lines().count(), 105_002),
            }
            request.process_key = Some("old-only".into());
            assert_eq!(storage.export(&request).unwrap().records, 1);
            let output = fs::read_to_string(&path).unwrap();
            assert!(output.contains("old-only"));
            assert!(!output.contains("192.168.1.44"));
        }
    }

    #[test]
    fn export_failure_preserves_existing_output() {
        let directory = tempfile::tempdir().unwrap();
        let mut storage = Storage::in_memory().unwrap();
        storage.start_session("s1", 0).unwrap();
        let rows: Vec<_> = (0..5_001)
            .map(|i| {
                let mut row = sample_connection();
                row.id = format!("row-{i}");
                row.last_seen_ms = i;
                row
            })
            .collect();
        storage.save_connections("s1", &rows).unwrap();
        storage
            .connection
            .execute(
                "UPDATE connections SET connection_json = 'invalid' WHERE id = 'row-0'",
                [],
            )
            .unwrap();
        let path = directory.path().join("export.json");
        fs::write(&path, "keep me").unwrap();
        let request = ExportRequest {
            format: ExportFormat::Json,
            path: path.to_string_lossy().into_owned(),
            session_id: None,
            process_key: None,
            from_ms: None,
            to_ms: None,
            query: None,
            redaction: ExportRedaction::default(),
        };
        assert!(storage.export(&request).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "keep me");
    }

    #[test]
    fn exports_include_provenance_and_apply_each_local_redaction() {
        let directory = tempfile::tempdir().expect("tempdir");
        let mut storage = Storage::in_memory().expect("storage");
        storage.start_session("s1", 1_000).expect("session");
        let mut connection = sample_connection();
        connection.key.local =
            sabicheiro_core::Endpoint::new("192.168.1.44".parse().expect("local IP"), 50_000);
        connection.process.executable_path = Some("C:\\Users\\alice\\Apps\\browser.exe".into());
        connection.geo = Some(sabicheiro_core::GeoLocation {
            country_code: Some("AA".into()),
            precision: sabicheiro_core::GeoPrecision::Country,
            source: "Fixture MMDB".into(),
            dataset_edition: Some("Sabicheiro-City-Test".into()),
            dataset_sha256: Some("abc123".into()),
            city_generation: 7,
            ..Default::default()
        });
        storage
            .save_connections("s1", &[connection])
            .expect("save connection");
        let path = directory.path().join("redacted.json");
        storage
            .export(&ExportRequest {
                format: ExportFormat::Json,
                path: path.to_string_lossy().into_owned(),
                session_id: None,
                process_key: None,
                from_ms: None,
                to_ms: None,
                query: None,
                redaction: ExportRedaction {
                    local_ip: true,
                    executable_path: false,
                    user_names: true,
                },
            })
            .expect("redacted export");
        let exported = fs::read_to_string(path).expect("read export");
        assert!(!exported.contains("192.168.1.44"));
        assert!(!exported.contains("alice"));
        assert!(exported.contains("[redacted]"));
        assert!(exported.contains("Fixture MMDB"));
        assert!(exported.contains("abc123"));
    }
}
