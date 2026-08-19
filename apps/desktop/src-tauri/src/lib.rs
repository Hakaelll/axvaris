use chrono::Timelike;
#[cfg(target_os = "windows")]
use sabicheiro_capture::WindowsEventCapture;
use sabicheiro_capture::{
    BoundedEvidenceQueue, EvidenceFusion, EvidenceSequencer, FlowCorrelator,
    NetworkCaptureProvider, PipelineQueueClass, PipelineStage, ProviderAction, ProviderSupervisor,
    RecordingController, SnapshotDiffer, inspect_executable, lookup_network_path, system_provider,
};
use sabicheiro_core::{
    AddressFamily, AppSettings, Bookmark, CapabilityAvailability, CaptureCapability,
    CaptureCapabilityKind, CaptureMode, CaptureSelfTest, CaptureStatus, CaptureSummary,
    ComparisonRow, Connection, ConnectionAggregator, ConnectionFilter, ConnectionHistoryStats,
    ConnectionKey, CorrelationQuality, CounterSource, DashboardSnapshot, DashboardUpdate,
    DerivedGeoEnrichment, Diagnostics, DnsAnswerEvidence, DnsCausalGraph, DnsCausalRelation,
    ExecutableInspectionState, ExecutableInspectionStatus, ExportFormat, ExportRequest,
    ExportResult, FocusRules, GeoDatabaseStatus, HandshakeModeAudit, HistoryCursor, HistoryEntry,
    HistoryQuery, IdentityEventKind, IdentityTimelineEvent, IgnoreRule, ImportedSabiBundle,
    ListenerExposure, ListenerScope, ListenerSnapshot, LocalDataCategory, LocalDataRemovalResult,
    LocalNotificationRecord, LocalRule, LocalRuleAction, NetworkActionBroker, NetworkActionGrant,
    NetworkActionIntent, NetworkActionPolicy, NetworkEventKind, NetworkPathEvidence,
    NetworkPathKind, NetworkProfile, NetworkProtocol, NormalizedNetworkEvent,
    NotificationCenterEngine, NotificationState, ObservationSource, OfflineServiceHint,
    OperationProfile, ProviderCapability, ProviderDescriptor, ProviderMode, QuietHours,
    RecordingRequest, RecordingState, RecordingStatus, RecordingSummary, ReenrichmentJobState,
    ReenrichmentJobStatus, RetentionPolicy, RuleContext, RuleKind, RuleMatch, SabiExportRequest,
    SabiManifest, SavedInvestigation, SelfTestCheck, SelfTestLevel, SessionComparison,
    SessionSummary, StorageHealth, SubsystemDiagnostic, SubsystemStatus, SupportBundleRequest,
    TimelineQuery, TimelineResolution, TodayCardState, TrafficCounters, TrafficSample,
    TransferRates, reconstruct_at,
};
use sabicheiro_geo::{
    AddressScope, DnsResolver, GeoDatabaseKind, GeoLookup, LocalGeoResolver, import_mmdb,
    remove_mmdb, rollback_mmdb, update_dbip_lite,
};
use sabicheiro_storage::Storage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{self, Seek, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Networks, ProcessesToUpdate, System};
use tauri::{Manager, State};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

const LIVE_POLL_INTERVAL: Duration = Duration::from_millis(750);
const DEMO_EVENT_INTERVAL: Duration = Duration::from_secs(1);
const PERSIST_INTERVAL: Duration = Duration::from_secs(1);
const DIAGNOSTICS_INTERVAL: Duration = Duration::from_secs(2);
const INTERFACE_REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const RETENTION_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const DNS_QUEUE_CAPACITY: usize = 1_024;
const GEO_QUEUE_CAPACITY: usize = 2_048;
const EVIDENCE_STORAGE_BATCH_SIZE: usize = 4_096;

fn evidence_provider_descriptor(source: ObservationSource, generation: u64) -> ProviderDescriptor {
    let (provider_id, display_name, mode, kinds): (
        &str,
        &str,
        ProviderMode,
        &[CaptureCapabilityKind],
    ) = match source {
        ObservationSource::WindowsIpHelper => (
            "windows.ip_helper",
            "Windows IP Helper",
            ProviderMode::Snapshot,
            &[
                CaptureCapabilityKind::SocketSnapshot,
                CaptureCapabilityKind::ProcessIdentity,
                CaptureCapabilityKind::Interface,
            ],
        ),
        ObservationSource::WindowsEtw => (
            "windows.etw",
            "Windows ETW",
            ProviderMode::EventStream,
            &[
                CaptureCapabilityKind::LifecycleEvents,
                CaptureCapabilityKind::FlowBytes,
                CaptureCapabilityKind::FlowPackets,
                CaptureCapabilityKind::Dns,
                CaptureCapabilityKind::ProcessIdentity,
            ],
        ),
        ObservationSource::WindowsHelper => (
            "windows.helper",
            "Sabicheiro capture helper",
            ProviderMode::EventStream,
            &[
                CaptureCapabilityKind::LifecycleEvents,
                CaptureCapabilityKind::FlowBytes,
                CaptureCapabilityKind::FlowPackets,
                CaptureCapabilityKind::Dns,
                CaptureCapabilityKind::ProcessIdentity,
            ],
        ),
        ObservationSource::LinuxInetDiag => (
            "linux.inet_diag",
            "Linux INET_DIAG",
            ProviderMode::Snapshot,
            &[
                CaptureCapabilityKind::SocketSnapshot,
                CaptureCapabilityKind::ProcessIdentity,
                CaptureCapabilityKind::NetworkNamespace,
                CaptureCapabilityKind::Interface,
            ],
        ),
        ObservationSource::LinuxEbpf => (
            "linux.ebpf",
            "Linux eBPF",
            ProviderMode::EventStream,
            &[
                CaptureCapabilityKind::LifecycleEvents,
                CaptureCapabilityKind::FlowBytes,
                CaptureCapabilityKind::FlowPackets,
                CaptureCapabilityKind::ProcessIdentity,
                CaptureCapabilityKind::NetworkNamespace,
                CaptureCapabilityKind::Interface,
            ],
        ),
        ObservationSource::Demo => (
            "fixture.demo",
            "Deterministic demo",
            ProviderMode::Fixture,
            &[
                CaptureCapabilityKind::LifecycleEvents,
                CaptureCapabilityKind::FlowBytes,
                CaptureCapabilityKind::FlowPackets,
                CaptureCapabilityKind::ProcessIdentity,
                CaptureCapabilityKind::Interface,
            ],
        ),
        _ => (
            "sabicheiro.normalized",
            "Normalized local observation",
            ProviderMode::Unknown,
            &[CaptureCapabilityKind::LifecycleEvents],
        ),
    };
    ProviderDescriptor {
        protocol_version: 1,
        provider_id: provider_id.into(),
        display_name: display_name.into(),
        mode,
        generation: generation.max(1),
        capabilities: kinds
            .iter()
            .copied()
            .map(|kind| ProviderCapability {
                kind,
                availability: CapabilityAvailability::Available,
                source,
                limitation: (mode == ProviderMode::Snapshot).then(|| {
                    "lifecycle timing is bounded by the provider snapshot interval".into()
                }),
            })
            .collect(),
    }
}

fn evidence_queue_class(observation: &sabicheiro_core::EvidenceObservation) -> PipelineQueueClass {
    match observation.network.as_ref().map(|event| event.kind) {
        Some(NetworkEventKind::Opened | NetworkEventKind::Closed) => PipelineQueueClass::Lifecycle,
        Some(NetworkEventKind::Updated)
            if observation
                .network
                .as_ref()
                .is_some_and(|event| event.traffic_delta != TrafficCounters::default()) =>
        {
            PipelineQueueClass::RepeatedMetric
        }
        _ => PipelineQueueClass::Context,
    }
}

fn transfer_rates_from_deltas(
    received_bytes: u64,
    transmitted_bytes: u64,
    packets: u64,
    elapsed: Duration,
) -> TransferRates {
    let elapsed_seconds = elapsed.as_secs_f64().max(0.001);
    TransferRates {
        down_bytes_per_second: received_bytes as f64 / elapsed_seconds,
        up_bytes_per_second: transmitted_bytes as f64 / elapsed_seconds,
        packets_per_second: packets as f64 / elapsed_seconds,
    }
}

fn counters_from_rates(rates: TransferRates) -> TrafficCounters {
    TrafficCounters {
        bytes_sent: rates.up_bytes_per_second.max(0.0).round() as u64,
        bytes_received: rates.down_bytes_per_second.max(0.0).round() as u64,
        packets_sent: 0,
        packets_received: rates.packets_per_second.max(0.0).round() as u64,
    }
}

#[derive(Clone)]
struct AppState(Arc<Runtime>);

struct TrayUi {
    _icon: TrayIcon<tauri::Wry>,
    _status: MenuItem<tauri::Wry>,
    _health: MenuItem<tauri::Wry>,
    _pause: MenuItem<tauri::Wry>,
}

struct Runtime {
    engine: Mutex<Engine>,
    reenrichment_jobs: Mutex<HashMap<String, ReenrichmentJobControl>>,
    executable_jobs: Mutex<HashMap<String, ExecutableInspectionJobControl>>,
    shutdown: AtomicBool,
    worker: Mutex<Option<thread::Thread>>,
    data_directory: PathBuf,
    logging: Option<LocalLogging>,
}

#[derive(Clone)]
struct ReenrichmentJobControl {
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<ReenrichmentJobStatus>>,
}

#[derive(Clone)]
struct ExecutableInspectionJobControl {
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<ExecutableInspectionStatus>>,
}

struct LocalLogging {
    directory: PathBuf,
    write_lock: Mutex<()>,
}

impl LocalLogging {
    fn info(&self, message: &'static str) {
        self.write("INFO", message);
    }

    fn write(&self, level: &'static str, message: &'static str) {
        let Ok(_guard) = self.write_lock.lock() else {
            return;
        };
        let date = chrono::Utc::now().format("%Y-%m-%d");
        let path = self.directory.join(format!("sabicheiro.log.{date}"));
        let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
            return;
        };
        let timestamp = chrono::Utc::now().to_rfc3339();
        let _ = writeln!(file, "{timestamp}\t{level}\tsabicheiro_desktop\t{message}");
        let _ = file.flush();
    }
}

#[derive(Debug, Clone, Serialize)]
struct CommandError {
    code: &'static str,
    message: String,
    recoverable: bool,
}

impl CommandError {
    fn recoverable(code: &'static str, error: impl std::fmt::Display) -> Self {
        Self {
            code,
            message: error.to_string(),
            recoverable: true,
        }
    }

    fn fatal(code: &'static str, error: impl std::fmt::Display) -> Self {
        Self {
            code,
            message: error.to_string(),
            recoverable: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetCaptureRequest {
    mode: CaptureMode,
    selected_interface: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryResponse {
    entries: Vec<HistoryEntry>,
    sessions: Vec<SessionSummary>,
    recordings: Vec<RecordingSummary>,
    #[serde(rename = "next_cursor")]
    next_cursor: Option<HistoryCursor>,
    #[serde(rename = "has_more")]
    has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionContextResponse {
    connection_id: String,
    dns: Option<DnsCausalRelation>,
    path: NetworkPathEvidence,
    listener: Option<ListenerExposure>,
    service_hint: Option<OfflineServiceHint>,
    registrable_domain: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComparisonExportRequest {
    left_session_id: String,
    right_session_id: String,
    format: ExportFormat,
    path: String,
}

#[derive(Debug)]
struct DnsJob {
    key: ConnectionKey,
    address: IpAddr,
    grant: NetworkActionGrant,
}

#[derive(Debug)]
struct DnsResult {
    key: ConnectionKey,
    address: IpAddr,
    hostname: Option<String>,
    observed_at_ms: i64,
}

struct DnsService {
    sender: SyncSender<DnsJob>,
    receiver: Receiver<DnsResult>,
    pending: Arc<AtomicUsize>,
    dropped: Arc<AtomicU64>,
}

impl DnsService {
    fn start() -> Self {
        let (job_sender, job_receiver) = mpsc::sync_channel::<DnsJob>(DNS_QUEUE_CAPACITY);
        let (result_sender, result_receiver) = mpsc::channel::<DnsResult>();
        let pending = Arc::new(AtomicUsize::new(0));
        let worker_pending = pending.clone();
        let _ = thread::Builder::new()
            .name("sabicheiro-dns".into())
            .spawn(move || {
                let mut resolver = DnsResolver::default();
                while let Ok(job) = job_receiver.recv() {
                    let observed_at_ms = sabicheiro_core::now_ms();
                    let hostname = resolver.resolve(job.address, observed_at_ms, &job.grant);
                    worker_pending.fetch_sub(1, Ordering::Relaxed);
                    if result_sender
                        .send(DnsResult {
                            key: job.key,
                            address: job.address,
                            hostname,
                            observed_at_ms,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        Self {
            sender: job_sender,
            receiver: result_receiver,
            pending,
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    fn enqueue(&self, job: DnsJob) -> bool {
        match self.sender.try_send(job) {
            Ok(()) => {
                self.pending.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    fn pending(&self) -> usize {
        self.pending.load(Ordering::Relaxed)
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
struct GeoJob {
    address: IpAddr,
}

#[derive(Debug)]
struct GeoResult {
    address: IpAddr,
    result: Result<GeoLookup, String>,
}

struct GeoService {
    resolver: Arc<Mutex<LocalGeoResolver>>,
    sender: SyncSender<GeoJob>,
    receiver: Receiver<GeoResult>,
    waiters: Arc<Mutex<HashMap<IpAddr, Vec<ConnectionKey>>>>,
    pending: Arc<AtomicUsize>,
    dropped: Arc<AtomicU64>,
}

impl GeoService {
    fn start(resolver: LocalGeoResolver) -> Self {
        let resolver = Arc::new(Mutex::new(resolver));
        let (job_sender, job_receiver) = mpsc::sync_channel::<GeoJob>(GEO_QUEUE_CAPACITY);
        let (result_sender, result_receiver) = mpsc::channel::<GeoResult>();
        let pending = Arc::new(AtomicUsize::new(0));
        let worker_pending = pending.clone();
        let worker_resolver = resolver.clone();
        let _ = thread::Builder::new()
            .name("sabicheiro-geoip".into())
            .spawn(move || {
                while let Ok(job) = job_receiver.recv() {
                    let result = worker_resolver
                        .lock()
                        .map_err(|_| "GeoIP resolver state is unavailable".to_owned())
                        .and_then(|mut resolver| {
                            resolver
                                .lookup(job.address)
                                .map_err(|error| error.to_string())
                        });
                    worker_pending.fetch_sub(1, Ordering::Relaxed);
                    if result_sender
                        .send(GeoResult {
                            address: job.address,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        Self {
            resolver,
            sender: job_sender,
            receiver: result_receiver,
            waiters: Arc::new(Mutex::new(HashMap::new())),
            pending,
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    fn enqueue(&self, key: ConnectionKey, address: IpAddr) -> bool {
        let Ok(mut waiters) = self.waiters.lock() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        };
        let entry = waiters.entry(address).or_default();
        if !entry.contains(&key) {
            entry.push(key);
        }
        if entry.len() > 1 {
            return true;
        }
        match self.sender.try_send(GeoJob { address }) {
            Ok(()) => {
                self.pending.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                waiters.remove(&address);
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    fn take_waiters(&self, address: IpAddr) -> Vec<ConnectionKey> {
        self.waiters
            .lock()
            .ok()
            .and_then(|mut waiters| waiters.remove(&address))
            .unwrap_or_default()
    }

    fn status(&self) -> Result<GeoDatabaseStatus, String> {
        self.resolver
            .lock()
            .map(|resolver| resolver.status())
            .map_err(|_| "GeoIP resolver state is unavailable".into())
    }

    fn generations(&self) -> Option<(u64, u64)> {
        self.resolver
            .lock()
            .ok()
            .map(|resolver| resolver.generations())
    }

    fn lookup(&self, address: IpAddr) -> Result<GeoLookup, String> {
        self.resolver
            .lock()
            .map_err(|_| "GeoIP resolver state is unavailable".to_owned())?
            .lookup(address)
            .map_err(|error| error.to_string())
    }

    fn reload(
        &self,
        city_path: impl AsRef<Path>,
        asn_path: impl AsRef<Path>,
    ) -> Result<(), String> {
        self.resolver
            .lock()
            .map_err(|_| "GeoIP resolver state is unavailable".to_owned())?
            .reload(city_path, asn_path)
            .map_err(|error| error.to_string())
    }

    fn reload_kind(&self, kind: GeoDatabaseKind, path: impl AsRef<Path>) -> Result<(), String> {
        self.resolver
            .lock()
            .map_err(|_| "GeoIP resolver state is unavailable".to_owned())?
            .reload_kind(kind, path)
            .map_err(|error| error.to_string())
    }

    fn replace(&self, resolver: LocalGeoResolver) -> Result<(), String> {
        let mut current = self
            .resolver
            .lock()
            .map_err(|_| "GeoIP resolver state is unavailable".to_owned())?;
        *current = resolver;
        Ok(())
    }

    fn pending(&self) -> usize {
        self.pending.load(Ordering::Relaxed)
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

struct Engine {
    aggregator: ConnectionAggregator,
    capture_mode: CaptureMode,
    capture_status: CaptureStatus,
    selected_interface: Option<String>,
    selected_interface_addresses: HashSet<IpAddr>,
    interface_by_address: HashMap<IpAddr, String>,
    vpn_active: bool,
    provider: Option<Box<dyn NetworkCaptureProvider>>,
    #[cfg(target_os = "windows")]
    event_capture: Option<WindowsEventCapture>,
    differ: SnapshotDiffer,
    correlator: FlowCorrelator,
    capture_generation: u64,
    evidence_clock_origin: Instant,
    evidence_sequencers: HashMap<(ObservationSource, u64), EvidenceSequencer>,
    evidence_fusion: EvidenceFusion,
    evidence_storage_queue: BoundedEvidenceQueue,
    last_event_capture_drops: u64,
    provider_supervisor: ProviderSupervisor,
    demo: sabicheiro_core::DemoGenerator,
    storage: Storage,
    geo: GeoService,
    dns: DnsService,
    dns_observations: HashMap<IpAddr, (String, i64)>,
    dns_causal_graph: DnsCausalGraph,
    dns_relations: HashMap<ConnectionKey, sabicheiro_core::DnsCausalRelation>,
    settings: AppSettings,
    session: Option<SessionSummary>,
    recording: RecordingController,
    active_recording_id: Option<i64>,
    diagnostics: Diagnostics,
    process_metrics: System,
    network_metrics: Networks,
    network_rates: TransferRates,
    last_capture_poll: Instant,
    last_demo_event: Instant,
    last_persist: Instant,
    last_persisted_revision: u64,
    persisted_connections: HashMap<String, Connection>,
    last_baseline_persist: Instant,
    last_traffic_sample: Instant,
    last_traffic_write: Instant,
    last_traffic_signature: Vec<TrafficSample>,
    last_diagnostics: Instant,
    last_interface_refresh: Instant,
    last_retention_cleanup: Instant,
    last_event_rate: Instant,
    events_since_rate: u64,
    paused_view_events: u64,
    view_paused: bool,
    paused_at_revision: u64,
    ipc_sequence: u64,
    ipc_baseline: Option<DashboardSnapshot>,
    ipc_metrics: IpcMetrics,
    subsystem_states: HashMap<String, (SubsystemStatus, i64)>,
    last_tick_at: Instant,
    network_signature: String,
    active_network_profile_id: Option<String>,
    local_rules: Vec<LocalRule>,
    notification_center: NotificationCenterEngine,
}

#[derive(Debug, Default)]
struct IpcMetrics {
    requests: u64,
    bytes_serialized: u64,
    empty_deltas: u64,
    overlaps: u64,
}

impl Runtime {
    fn new(data_directory: PathBuf, logging: Option<LocalLogging>) -> Self {
        let database_path = data_directory.join("sabicheiro.sqlite3");
        let (storage, storage_error) = match Storage::open(&database_path) {
            Ok(storage) => (storage, None),
            Err(error) => {
                let fallback = Storage::in_memory().expect("in-memory SQLite must initialize");
                (fallback, Some(error.to_string()))
            }
        };
        let settings = storage.load_settings().unwrap_or_default();
        let local_rules = storage.list_local_rules().unwrap_or_default();
        let notification_center = NotificationCenterEngine::from_records(
            10_000,
            storage.list_notifications(10_000).unwrap_or_default(),
        );
        let retention_error = storage
            .cleanup(settings.retention, sabicheiro_core::now_ms())
            .err()
            .map(|error| error.to_string());
        let storage_health = storage.health_check();
        let storage_health_error = storage_health
            .as_ref()
            .err()
            .map(ToString::to_string)
            .or_else(|| {
                storage_health
                    .as_ref()
                    .ok()
                    .filter(|health| !health.quick_check_ok)
                    .map(|health| format!("SQLite quick_check: {}", health.detail))
            });
        let (baselines, baseline_error) = match storage.load_baselines() {
            Ok(baselines) => (baselines, None),
            Err(error) => (Default::default(), Some(error.to_string())),
        };
        let geo_directory = data_directory.join("geoip");
        let city_path = geo_directory.join("dbip-city-lite.mmdb");
        let asn_path = geo_directory.join("dbip-asn-lite.mmdb");
        let (geo, geo_error) = match LocalGeoResolver::open(Some(&city_path), Some(&asn_path)) {
            Ok(geo) => (geo, None),
            Err(error) => (LocalGeoResolver::empty(), Some(error.to_string())),
        };
        let initial_error = storage_error
            .or(geo_error)
            .or(baseline_error)
            .or(retention_error)
            .or(storage_health_error);
        let mut diagnostics = Diagnostics {
            backend: "stopped".into(),
            capture_event_source: "none".into(),
            capture_queue_capacity: DNS_QUEUE_CAPACITY + GEO_QUEUE_CAPACITY,
            last_error: initial_error,
            ..Diagnostics::default()
        };
        diagnostics.database_size_bytes = storage.database_size_bytes();
        if let Ok(health) = storage_health {
            diagnostics.storage_quick_check_ok = health.quick_check_ok;
            diagnostics.storage_last_check_ms = Some(health.checked_at_ms);
            diagnostics.storage_wal_pages = health.wal_log_pages;
        }
        Self {
            engine: Mutex::new(Engine {
                aggregator: ConnectionAggregator::from_baselines(baselines),
                capture_mode: CaptureMode::Off,
                capture_status: CaptureStatus::Stopped,
                selected_interface: settings.selected_interface.clone(),
                selected_interface_addresses: HashSet::new(),
                interface_by_address: HashMap::new(),
                vpn_active: false,
                provider: None,
                #[cfg(target_os = "windows")]
                event_capture: None,
                differ: SnapshotDiffer::default(),
                correlator: FlowCorrelator::default(),
                capture_generation: 0,
                evidence_clock_origin: Instant::now(),
                evidence_sequencers: HashMap::new(),
                evidence_fusion: EvidenceFusion::default(),
                evidence_storage_queue: BoundedEvidenceQueue::new(
                    PipelineStage::Storage,
                    settings.operation_profile.evidence_queue_capacity(),
                ),
                last_event_capture_drops: 0,
                provider_supervisor: ProviderSupervisor::default(),
                demo: sabicheiro_core::DemoGenerator::new(settings.demo_seed),
                storage,
                geo: GeoService::start(geo),
                dns: DnsService::start(),
                dns_observations: HashMap::new(),
                dns_causal_graph: DnsCausalGraph::default(),
                dns_relations: HashMap::new(),
                settings,
                session: None,
                recording: RecordingController::default(),
                active_recording_id: None,
                diagnostics,
                process_metrics: System::new(),
                network_metrics: Networks::new_with_refreshed_list(),
                network_rates: TransferRates::default(),
                last_capture_poll: Instant::now() - LIVE_POLL_INTERVAL,
                last_demo_event: Instant::now() - DEMO_EVENT_INTERVAL,
                last_persist: Instant::now(),
                last_persisted_revision: 0,
                persisted_connections: HashMap::new(),
                last_baseline_persist: Instant::now(),
                last_traffic_sample: Instant::now(),
                last_traffic_write: Instant::now(),
                last_traffic_signature: Vec::new(),
                last_diagnostics: Instant::now() - DIAGNOSTICS_INTERVAL,
                last_interface_refresh: Instant::now() - INTERFACE_REFRESH_INTERVAL,
                last_retention_cleanup: Instant::now(),
                last_event_rate: Instant::now(),
                events_since_rate: 0,
                paused_view_events: 0,
                view_paused: false,
                paused_at_revision: 0,
                ipc_sequence: 0,
                ipc_baseline: None,
                ipc_metrics: IpcMetrics::default(),
                subsystem_states: HashMap::new(),
                last_tick_at: Instant::now(),
                network_signature: String::new(),
                active_network_profile_id: None,
                local_rules,
                notification_center,
            }),
            reenrichment_jobs: Mutex::new(HashMap::new()),
            executable_jobs: Mutex::new(HashMap::new()),
            shutdown: AtomicBool::new(false),
            worker: Mutex::new(None),
            data_directory,
            logging,
        }
    }

    fn spawn(self: &Arc<Self>) {
        let runtime = self.clone();
        let _ = thread::Builder::new()
            .name("sabicheiro-capture".into())
            .spawn(move || {
                if let Ok(mut worker) = runtime.worker.lock() {
                    *worker = Some(thread::current());
                }
                while !runtime.shutdown.load(Ordering::Relaxed) {
                    let wait = if let Ok(mut engine) = runtime.engine.lock() {
                        engine.tick();
                        engine.next_wakeup()
                    } else {
                        Duration::from_secs(1)
                    };
                    thread::park_timeout(wait);
                }
            });
    }

    fn wake(&self) {
        if let Ok(worker) = self.worker.lock()
            && let Some(worker) = worker.as_ref()
        {
            worker.unpark();
        }
    }

    fn shutdown(&self) {
        if let Some(logging) = &self.logging {
            logging.info("Sabicheiro desktop runtime stopping");
        }
        self.shutdown.store(true, Ordering::Relaxed);
        self.wake();
        if let Ok(mut engine) = self.engine.lock() {
            if engine.recording.state().status == RecordingStatus::Recording
                || (engine.recording.state().status == RecordingStatus::Error
                    && engine.active_recording_id.is_some())
            {
                let _ = engine.stop_recording();
            }
            engine.stop_current_session(sabicheiro_core::now_ms());
        }
    }
}

impl Engine {
    fn next_wakeup(&self) -> Duration {
        fn remaining(last: Instant, interval: Duration) -> Duration {
            interval.saturating_sub(last.elapsed())
        }

        let capture = match self.capture_mode {
            CaptureMode::Off => Duration::from_secs(15),
            CaptureMode::Demo => remaining(self.last_demo_event, DEMO_EVENT_INTERVAL),
            CaptureMode::Live => remaining(
                self.last_capture_poll,
                Duration::from_millis(self.settings.operation_profile.poll_interval_ms()),
            ),
        };
        let diagnostics_interval = if self.capture_mode == CaptureMode::Off {
            Duration::from_secs(15)
        } else {
            DIAGNOSTICS_INTERVAL
        };
        let mut wait = capture
            .min(remaining(self.last_diagnostics, diagnostics_interval))
            .min(remaining(
                self.last_retention_cleanup,
                RETENTION_CLEANUP_INTERVAL,
            ));
        if self.capture_mode != CaptureMode::Off {
            wait = wait.min(remaining(self.last_persist, PERSIST_INTERVAL));
        }
        wait.clamp(Duration::from_millis(10), Duration::from_secs(15))
    }

    fn tick(&mut self) {
        let now_ms = sabicheiro_core::now_ms();
        self.provider_supervisor.tick(now_ms);
        self.apply_provider_actions(now_ms);
        let tick_gap = self.last_tick_at.elapsed();
        self.last_tick_at = Instant::now();
        if self.capture_mode == CaptureMode::Live && tick_gap > Duration::from_secs(5) {
            self.reconcile_after_environment_change(
                now_ms,
                "Capture resumed after a scheduling or sleep gap; socket baselines were reset.",
            );
        }
        self.drain_geo_results();
        self.drain_dns_results();
        self.refresh_selected_interface();
        match self.capture_mode {
            CaptureMode::Off => {}
            CaptureMode::Demo if self.last_demo_event.elapsed() >= DEMO_EVENT_INTERVAL => {
                self.last_demo_event = Instant::now();
                let events = self.demo.next_batch(now_ms);
                self.process_events(events, true);
            }
            CaptureMode::Demo => {}
            CaptureMode::Live
                if self.last_capture_poll.elapsed()
                    >= Duration::from_millis(
                        self.settings.operation_profile.poll_interval_ms(),
                    ) =>
            {
                self.last_capture_poll = Instant::now();
                let result = self.provider.as_mut().map(|provider| provider.snapshot());
                match result {
                    Some(Ok(mut sockets)) => {
                        let snapshot_provider_id = self.provider.as_ref().map(|provider| {
                            provider.descriptor(self.capture_generation).provider_id
                        });
                        if !self.selected_interface_addresses.is_empty() {
                            sockets.retain(|socket| {
                                socket.identity.local.address.is_unspecified()
                                    || self
                                        .selected_interface_addresses
                                        .contains(&socket.identity.local.address)
                            });
                            for socket in &mut sockets {
                                socket.interface = self.selected_interface.clone();
                            }
                        } else {
                            for socket in &mut sockets {
                                socket.interface = socket.interface.clone().or_else(|| {
                                    self.interface_by_address
                                        .get(&socket.identity.local.address)
                                        .cloned()
                                });
                            }
                        }
                        let events = self.differ.diff(sockets, now_ms);
                        self.correlator.observe_snapshots(events.iter(), now_ms);
                        self.process_events(events, false);
                        self.capture_status = CaptureStatus::Capturing;
                        if let Some(provider_id) = snapshot_provider_id {
                            self.provider_supervisor.mark_healthy(&provider_id, now_ms);
                        }
                    }
                    Some(Err(error)) => {
                        let message = error.to_string();
                        if let Some(provider_id) = self.provider.as_ref().map(|provider| {
                            provider.descriptor(self.capture_generation).provider_id
                        }) {
                            self.provider_supervisor.mark_failure(
                                &provider_id,
                                message.clone(),
                                now_ms,
                            );
                        }
                        self.capture_status = CaptureStatus::Error(message.clone());
                        self.diagnostics.last_error = Some(message);
                    }
                    None => {
                        self.capture_status =
                            CaptureStatus::Error("capture provider is unavailable".into());
                    }
                }
            }
            CaptureMode::Live => {}
        }
        // Socket snapshots establish identity before ETW byte events are attached.
        self.drain_platform_events(now_ms);
        self.aggregator.prune(now_ms);

        if self.capture_mode != CaptureMode::Off && self.last_persist.elapsed() >= PERSIST_INTERVAL
        {
            self.persist(now_ms, false);
            self.last_persist = Instant::now();
        }
        let evidence_stats = self.evidence_storage_queue.stats(
            self.evidence_clock_origin
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64,
        );
        if self.capture_mode == CaptureMode::Live && evidence_stats.resync_required {
            self.evidence_storage_queue.mark_resynced();
            self.reconcile_after_environment_change(
                now_ms,
                "The evidence consumer lost lifecycle order under backpressure; socket state was explicitly reconciled.",
            );
        }
        let diagnostics_interval = if self.capture_mode == CaptureMode::Off {
            Duration::from_secs(15)
        } else {
            DIAGNOSTICS_INTERVAL
        };
        if self.last_diagnostics.elapsed() >= diagnostics_interval {
            self.refresh_diagnostics();
            self.last_diagnostics = Instant::now();
        }
        if self.last_retention_cleanup.elapsed() >= RETENTION_CLEANUP_INTERVAL {
            if let Err(error) = self.storage.cleanup(self.settings.retention, now_ms) {
                self.diagnostics.last_error = Some(error.to_string());
            }
            if self.capture_mode == CaptureMode::Off
                || self.settings.operation_profile == OperationProfile::BackgroundLowResource
            {
                if let Err(error) = self.storage.compact_traffic_samples(now_ms) {
                    self.diagnostics.last_error = Some(error.to_string());
                }
            }
            if self.capture_mode == CaptureMode::Off {
                let _ = self.storage.optimize();
            }
            self.refresh_storage_health();
            self.last_retention_cleanup = Instant::now();
        }
        if self.last_event_rate.elapsed() >= Duration::from_secs(1) {
            let seconds = self.last_event_rate.elapsed().as_secs_f64().max(0.001);
            self.diagnostics.capture_events_per_second = self.events_since_rate as f64 / seconds;
            self.events_since_rate = 0;
            self.last_event_rate = Instant::now();
        }

        if self.view_paused {
            self.paused_view_events = self
                .aggregator
                .revision()
                .saturating_sub(self.paused_at_revision);
        }
        self.stop_expired_recording(now_ms);
    }

    fn apply_provider_actions(&mut self, now_ms: i64) {
        while let Some(action) = self.provider_supervisor.next_action() {
            match action {
                ProviderAction::Start { .. } => {}
                ProviderAction::Restart { provider_id, .. } if provider_id == "windows.etw" =>
                {
                    #[cfg(target_os = "windows")]
                    match WindowsEventCapture::start() {
                        Ok(capture) => {
                            self.event_capture = Some(capture);
                            self.last_event_capture_drops = 0;
                            self.provider_supervisor.mark_healthy(&provider_id, now_ms);
                            self.diagnostics.capture_event_source = "windows_etw".into();
                            self.diagnostics.flow_counters_available = true;
                        }
                        Err(error) => {
                            self.provider_supervisor.mark_failure(
                                &provider_id,
                                error.to_string(),
                                now_ms,
                            );
                        }
                    }
                }
                ProviderAction::Restart { provider_id, .. } => match system_provider() {
                    Ok(provider)
                        if provider.descriptor(self.capture_generation).provider_id
                            == provider_id =>
                    {
                        self.provider = Some(provider);
                        self.capture_status = CaptureStatus::Starting;
                        self.last_capture_poll = Instant::now() - LIVE_POLL_INTERVAL;
                    }
                    Ok(_) => self.provider_supervisor.mark_failure(
                        &provider_id,
                        "the restarted system provider identity changed",
                        now_ms,
                    ),
                    Err(error) => self.provider_supervisor.mark_failure(
                        &provider_id,
                        error.to_string(),
                        now_ms,
                    ),
                },
                ProviderAction::ActivateFallback {
                    provider_id,
                    fallback_provider_id,
                } => {
                    self.diagnostics.capture_event_source = "socket_polling".into();
                    self.diagnostics.flow_counters_available = false;
                    self.diagnostics.last_error = Some(format!(
                        "Provider {provider_id} is unavailable; {fallback_provider_id} remains active."
                    ));
                }
                ProviderAction::Reconcile { reason, .. } => {
                    self.reconcile_after_environment_change(now_ms, &reason);
                }
            }
        }
    }

    fn process_events(&mut self, events: Vec<NormalizedNetworkEvent>, demo: bool) {
        if events.is_empty() {
            return;
        }
        let generation = self.capture_generation.max(1);
        let base_monotonic_ns = self
            .evidence_clock_origin
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        let persist_evidence = !self
            .session
            .as_ref()
            .is_some_and(|session| session.ephemeral);
        let mut observations = Vec::with_capacity(events.len());
        for (offset, event) in events.into_iter().enumerate() {
            let source = event.evidence.observation_source;
            let sequencer = self
                .evidence_sequencers
                .entry((source, generation))
                .or_insert_with(|| {
                    EvidenceSequencer::new(
                        evidence_provider_descriptor(source, generation),
                        event.timestamp_ms,
                    )
                });
            let observation = sequencer.wrap_network_at(
                base_monotonic_ns.saturating_add(offset as u64),
                event.timestamp_ms,
                event,
            );
            if persist_evidence {
                let class = evidence_queue_class(&observation);
                let _ = self.evidence_storage_queue.push(
                    observation.clone(),
                    class,
                    None,
                    base_monotonic_ns,
                );
            }
            observations.push(observation);
        }
        let mut batch = self.evidence_fusion.ingest(observations);
        self.diagnostics.sequence_gaps = self.diagnostics.sequence_gaps.saturating_add(
            batch
                .gaps
                .iter()
                .map(sabicheiro_core::ProviderGap::missing_count)
                .sum::<u64>(),
        );
        for event in &mut batch.events {
            if event.kind != NetworkEventKind::Opened {
                continue;
            }
            let relation = self.dns_causal_graph.relate(event);
            if let Some(confirmed) = &relation.confirmed {
                event.hostname = Some(confirmed.query_name.clone());
                event.domain = Some(confirmed.query_name.clone());
            }
            if relation.confirmed.is_some() || !relation.candidates.is_empty() {
                self.dns_relations.insert(event.key.clone(), relation);
            }
        }
        self.project_events(batch.events, demo);
    }

    fn process_dns_answer(&mut self, answer: DnsAnswerEvidence) {
        let generation = self.capture_generation.max(1);
        let monotonic_ns = self
            .evidence_clock_origin
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        let source = answer.source;
        let observed_at_ms = answer.observed_at_ms;
        self.dns_causal_graph.observe(answer.clone());
        let sequencer = self
            .evidence_sequencers
            .entry((source, generation))
            .or_insert_with(|| {
                EvidenceSequencer::new(
                    evidence_provider_descriptor(source, generation),
                    observed_at_ms,
                )
            });
        let process_known = answer.process_instance.is_some();
        let observation = sequencer.wrap_dns_at(
            monotonic_ns,
            observed_at_ms,
            answer,
            CorrelationQuality::Confirmed,
            (!process_known).then(|| {
                "DNS answer is observed, but causal process attribution is unavailable".into()
            }),
        );
        if !self
            .session
            .as_ref()
            .is_some_and(|session| session.ephemeral)
        {
            let _ = self.evidence_storage_queue.push(
                observation.clone(),
                PipelineQueueClass::Context,
                None,
                monotonic_ns,
            );
        }
        let _ = self.evidence_fusion.ingest([observation]);
    }

    fn record_provider_gap(
        &mut self,
        source: ObservationSource,
        missing: u64,
        affected_capabilities: impl IntoIterator<Item = CaptureCapabilityKind>,
        reason: &str,
        now_ms: i64,
    ) {
        if missing == 0 {
            return;
        }
        let generation = self.capture_generation.max(1);
        let monotonic_ns = self
            .evidence_clock_origin
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        let sequencer = self
            .evidence_sequencers
            .entry((source, generation))
            .or_insert_with(|| {
                EvidenceSequencer::new(evidence_provider_descriptor(source, generation), now_ms)
            });
        let Some(observation) =
            sequencer.record_gap(missing, monotonic_ns, now_ms, affected_capabilities, reason)
        else {
            return;
        };
        if !self
            .session
            .as_ref()
            .is_some_and(|session| session.ephemeral)
        {
            let _ = self.evidence_storage_queue.push(
                observation.clone(),
                PipelineQueueClass::Lifecycle,
                None,
                monotonic_ns,
            );
        }
        let batch = self.evidence_fusion.ingest([observation]);
        self.diagnostics.sequence_gaps = self.diagnostics.sequence_gaps.saturating_add(
            batch
                .gaps
                .iter()
                .map(sabicheiro_core::ProviderGap::missing_count)
                .sum::<u64>(),
        );
    }

    /// Applies derived/local enrichments only after the original observations have
    /// entered the immutable journal queue.
    fn project_events(&mut self, mut events: Vec<NormalizedNetworkEvent>, demo: bool) {
        for event in &mut events {
            if event.kind == NetworkEventKind::Opened
                && !event.key.remote.address.is_unspecified()
                && !demo
            {
                if event.key.remote.scope == AddressScope::Global
                    && !self
                        .geo
                        .enqueue(event.key.clone(), event.key.remote.address)
                {
                    self.diagnostics.dropped_ui_events =
                        self.diagnostics.dropped_ui_events.saturating_add(1);
                }
                if self.settings.reverse_dns_enabled
                    && event.key.remote.scope == AddressScope::Global
                {
                    let broker = NetworkActionBroker::new(NetworkActionPolicy {
                        offline_enforced: self.settings.offline_enforced,
                        geoip_updates_allowed: false,
                        reverse_dns_allowed: self.settings.reverse_dns_enabled,
                    });
                    match broker.authorize(
                        NetworkActionIntent::ReverseDns,
                        "settings.reverse_dns_enabled",
                    ) {
                        Ok(grant) => {
                            let _ = self.storage.record_network_action(
                                event.timestamp_ms,
                                grant.intent(),
                                grant.destination_class(),
                                Some(grant.consent_id()),
                                true,
                                0,
                                0,
                                "Queued one bounded reverse-DNS request; observed IP redacted from audit.",
                            );
                            if !self.dns.enqueue(DnsJob {
                                key: event.key.clone(),
                                address: event.key.remote.address,
                                grant,
                            }) {
                                self.diagnostics.dropped_ui_events =
                                    self.diagnostics.dropped_ui_events.saturating_add(1);
                            }
                        }
                        Err(error) => {
                            let _ = self.storage.record_network_action(
                                event.timestamp_ms,
                                NetworkActionIntent::ReverseDns,
                                NetworkActionIntent::ReverseDns.destination_class(),
                                Some("settings.reverse_dns_enabled"),
                                false,
                                0,
                                0,
                                &error.to_string(),
                            );
                        }
                    }
                }
            }
        }
        self.events_since_rate = self
            .events_since_rate
            .saturating_add(events.len().try_into().unwrap_or(u64::MAX));
        for event in events {
            let key = event.key.clone();
            let kind = event.kind;
            let timestamp_ms = event.timestamp_ms;
            self.aggregator.apply(event);
            if matches!(kind, NetworkEventKind::Opened | NetworkEventKind::Updated)
                && let Some(connection) = self.aggregator.connection(&key).cloned()
            {
                self.evaluate_local_rules(&connection, timestamp_ms);
            }
        }
    }

    fn evaluate_local_rules(&mut self, connection: &Connection, occurred_at_ms: i64) {
        if self.local_rules.is_empty()
            || self
                .session
                .as_ref()
                .is_some_and(|session| session.ephemeral)
        {
            return;
        }
        let listener_scope = if connection.state == sabicheiro_core::ConnectionState::Listening {
            if connection.key.local.address.is_loopback() {
                ListenerScope::Loopback
            } else if connection.key.local.address.is_unspecified() {
                ListenerScope::Wildcard
            } else if self.vpn_active {
                ListenerScope::Vpn
            } else {
                ListenerScope::Lan
            }
        } else {
            ListenerScope::Unknown
        };
        let path_kind = if self.vpn_active {
            NetworkPathKind::Vpn
        } else if connection.key.remote.scope == AddressScope::Global {
            NetworkPathKind::Gateway
        } else {
            NetworkPathKind::Direct
        };
        let context = RuleContext {
            network_profile_id: self.active_network_profile_id.clone(),
            path_kind,
            listener_scope,
        };
        let rules = self.local_rules.clone();
        for rule in rules {
            let rule_match = rule.evaluate(connection, &context);
            if !rule_match.matched {
                continue;
            }
            let _ = self.storage.record_rule_match(&rule_match, occurred_at_ms);
            let _ = self
                .storage
                .apply_rule_match_actions(&rule_match, occurred_at_ms);
            if !rule_match.actions.contains(&LocalRuleAction::Notify) {
                continue;
            }
            let detail = if rule_match.reasons.is_empty() {
                "The local rule matched this retained connection evidence.".into()
            } else {
                rule_match.reasons.join(", ")
            };
            let quiet_hours = QuietHours {
                enabled: self.settings.notification_quiet_hours_enabled,
                start_minute_local: self.settings.notification_quiet_start_minute,
                end_minute_local: self.settings.notification_quiet_end_minute,
            };
            let minute_local =
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(occurred_at_ms)
                    .map(|timestamp| timestamp.with_timezone(&chrono::Local))
                    .map_or(0, |timestamp| {
                        u16::try_from(timestamp.hour() * 60 + timestamp.minute()).unwrap_or(0)
                    });
            if let Some(notification) = self.notification_center.ingest(
                &rule_match,
                self.active_network_profile_id.clone(),
                rule.name,
                detail,
                occurred_at_ms,
                rule.cooldown_seconds,
                quiet_hours,
                minute_local,
            ) {
                let _ = self.storage.save_notification(&notification);
            }
        }
    }

    fn drain_geo_results(&mut self) {
        while let Ok(result) = self.geo.receiver.try_recv() {
            let waiters = self.geo.take_waiters(result.address);
            match result.result {
                Ok(lookup) => {
                    if self.geo.generations().is_some_and(|generations| {
                        generations != (lookup.city_generation, lookup.asn_generation)
                    }) {
                        for key in waiters {
                            let _ = self.geo.enqueue(key, result.address);
                        }
                        continue;
                    }
                    for key in waiters {
                        self.aggregator.enrich(
                            &key,
                            None,
                            None,
                            lookup.location.clone(),
                            lookup.asn.clone(),
                        );
                    }
                }
                Err(error) => self.diagnostics.last_error = Some(error),
            }
        }
    }

    fn refresh_selected_interface(&mut self) {
        if self.capture_mode != CaptureMode::Live
            || self.last_interface_refresh.elapsed() < INTERFACE_REFRESH_INTERVAL
        {
            return;
        }
        self.last_interface_refresh = Instant::now();
        let interfaces = match self.provider.as_mut() {
            Some(provider) => provider.interfaces(),
            None => return,
        };
        let Ok(interfaces) = interfaces else {
            return;
        };
        let mut signature_parts = interfaces
            .iter()
            .map(|network_interface| {
                let mut addresses = network_interface
                    .addresses
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                addresses.sort();
                format!(
                    "{}:{}:{}",
                    network_interface.id,
                    network_interface.is_up,
                    addresses.join("|")
                )
            })
            .collect::<Vec<_>>();
        signature_parts.sort();
        let signature = signature_parts.join(";");
        let changed = !self.network_signature.is_empty() && signature != self.network_signature;
        self.network_signature = signature;
        if changed {
            self.reconcile_after_environment_change(
                sabicheiro_core::now_ms(),
                "Network interfaces or addresses changed; flows were reconciled on a new capture generation.",
            );
        }
        self.interface_by_address = interfaces
            .iter()
            .flat_map(|network_interface| {
                network_interface
                    .addresses
                    .iter()
                    .copied()
                    .map(|address| (address, network_interface.id.clone()))
            })
            .collect();
        self.vpn_active = interfaces
            .iter()
            .any(|network_interface| network_interface.is_up && network_interface.is_vpn);
        if !self.network_signature.is_empty() {
            let mut profile = NetworkProfile::pseudonymous(
                &self.network_signature,
                None,
                self.vpn_active,
                sabicheiro_core::now_ms(),
            );
            if let Ok(existing_profiles) = self.storage.list_network_profiles() {
                if let Some(existing) = existing_profiles
                    .into_iter()
                    .find(|existing| existing.id == profile.id)
                {
                    profile.created_at_ms = existing.created_at_ms;
                    profile.display_name = existing.display_name;
                }
            }
            self.active_network_profile_id = Some(profile.id.clone());
            let _ = self
                .storage
                .save_network_profile(&profile, sabicheiro_core::now_ms());
        }
        let Some(selected) = self.selected_interface.as_deref() else {
            return;
        };
        if let Some(network_interface) = interfaces
            .iter()
            .find(|network_interface| network_interface.id == selected && network_interface.is_up)
        {
            self.selected_interface_addresses =
                network_interface.addresses.iter().copied().collect();
            return;
        }
        // A removed dock/VPN/Wi-Fi adapter must never leave capture silently empty.
        // Fall back to all interfaces and persist the recoverable selection.
        self.selected_interface = None;
        self.selected_interface_addresses.clear();
        self.interface_by_address.clear();
        self.settings.selected_interface = None;
        let _ = self.storage.save_settings(&self.settings);
        self.diagnostics.last_error =
            Some("The selected adapter disappeared; capture recovered on All interfaces.".into());
    }

    fn reconcile_after_environment_change(&mut self, now_ms: i64, reason: &str) {
        let closed = self.differ.clear(now_ms);
        self.process_events(closed, false);
        self.capture_generation = self.capture_generation.saturating_add(1).max(1);
        self.differ = SnapshotDiffer::new(self.capture_generation);
        self.correlator.clear();
        self.evidence_fusion = EvidenceFusion::default();
        self.evidence_sequencers.clear();
        self.network_metrics = Networks::new_with_refreshed_list();
        self.network_rates = TransferRates::default();
        self.diagnostics.capture_generation = self.capture_generation;
        self.diagnostics.snapshot_resyncs = self.diagnostics.snapshot_resyncs.saturating_add(1);
        self.diagnostics.last_error = Some(reason.into());
        self.ipc_baseline = None;
    }

    #[cfg(target_os = "windows")]
    fn drain_platform_events(&mut self, now_ms: i64) {
        self.dns_observations
            .retain(|_, (_, expires_at_ms)| *expires_at_ms > now_ms);
        let Some(capture) = self.event_capture.as_mut() else {
            return;
        };
        let (mut events, observations) = capture.drain(now_ms);
        let dropped = capture.dropped();
        let pending = capture.pending();
        let capacity = capture.capacity();
        if !self.selected_interface_addresses.is_empty() {
            events.retain(|event| {
                event.key.local.address.is_unspecified()
                    || self
                        .selected_interface_addresses
                        .contains(&event.key.local.address)
            });
            for event in &mut events {
                event.interface = self.selected_interface.clone();
            }
        }
        for event in &mut events {
            self.correlator.correlate(event);
        }
        for observation in observations {
            self.dns_observations.insert(
                observation.address,
                (observation.hostname.clone(), observation.expires_at_ms),
            );
            let mut answer = DnsAnswerEvidence::new(
                format!(
                    "windows-etw-dns:{}:{}:{}",
                    observation.observed_at_ms, observation.hostname, observation.address
                ),
                observation.hostname,
                [observation.address],
                observation.ttl_seconds,
                observation.process_instance,
                observation.observed_at_ms,
                ObservationSource::WindowsEtw,
            );
            answer.canonical_name = observation.canonical_name;
            answer.resolver = observation.resolver;
            self.process_dns_answer(answer);
            self.diagnostics.dns_correlations_observed =
                self.diagnostics.dns_correlations_observed.saturating_add(1);
        }
        self.diagnostics.packet_events_observed = self
            .diagnostics
            .packet_events_observed
            .saturating_add(events.len().try_into().unwrap_or(u64::MAX));
        let correlation = self.correlator.stats();
        self.diagnostics.correlated_capture_events = correlation.correlated;
        self.diagnostics.approximate_capture_events = correlation.approximate;
        self.diagnostics.orphan_capture_events = correlation.orphaned;
        self.diagnostics.dropped_capture_events = dropped;
        self.diagnostics.capture_queue_depth = pending
            .saturating_add(self.dns.pending())
            .saturating_add(self.geo.pending());
        self.diagnostics.capture_queue_capacity = capacity
            .saturating_add(DNS_QUEUE_CAPACITY)
            .saturating_add(GEO_QUEUE_CAPACITY);
        let newly_dropped = dropped.saturating_sub(self.last_event_capture_drops);
        self.last_event_capture_drops = dropped;
        self.record_provider_gap(
            ObservationSource::WindowsEtw,
            newly_dropped,
            [
                CaptureCapabilityKind::LifecycleEvents,
                CaptureCapabilityKind::FlowBytes,
                CaptureCapabilityKind::FlowPackets,
                CaptureCapabilityKind::Dns,
            ],
            "bounded Windows ETW queue overflow",
            now_ms,
        );
        if !events.is_empty() {
            self.process_events(events, false);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn drain_platform_events(&mut self, now_ms: i64) {
        self.dns_observations
            .retain(|_, (_, expires_at_ms)| *expires_at_ms > now_ms);
    }

    fn drain_dns_results(&mut self) {
        while let Ok(result) = self.dns.receiver.try_recv() {
            let domain = result.hostname.clone();
            self.aggregator
                .enrich(&result.key, result.hostname.clone(), domain, None, None);
            if let Some(hostname) = result.hostname {
                self.process_dns_answer(DnsAnswerEvidence::new(
                    format!(
                        "reverse-dns:{}:{}:{}",
                        result.observed_at_ms, hostname, result.address
                    ),
                    hostname,
                    [result.address],
                    3_600,
                    result.key.process_instance,
                    result.observed_at_ms,
                    ObservationSource::LocalDns,
                ));
            }
        }
    }

    fn flush_evidence_journal(&mut self) -> bool {
        let now_monotonic_ns = self
            .evidence_clock_origin
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        let mut batch = Vec::with_capacity(EVIDENCE_STORAGE_BATCH_SIZE);
        while batch.len() < EVIDENCE_STORAGE_BATCH_SIZE {
            let Some(observation) = self.evidence_storage_queue.pop(now_monotonic_ns) else {
                break;
            };
            batch.push(observation);
        }
        if batch.is_empty() {
            return true;
        }
        let session_id = self.session.as_ref().map(|session| session.id.as_str());
        if let Err(error) = self.storage.append_observations(session_id, &batch) {
            for observation in batch {
                let class = evidence_queue_class(&observation);
                let _ =
                    self.evidence_storage_queue
                        .push(observation, class, None, now_monotonic_ns);
            }
            self.diagnostics.last_error = Some(error.to_string());
            return false;
        }
        true
    }

    fn persist(&mut self, now_ms: i64, force: bool) {
        let _ = self.flush_evidence_journal();
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.ephemeral)
        {
            self.update_session_summary(now_ms);
            return;
        }
        let Some(session_id) = self.session.as_ref().map(|session| session.id.clone()) else {
            return;
        };
        let revision = self.aggregator.revision();
        let connections_changed = force || revision != self.last_persisted_revision;
        let mut connections =
            connections_changed.then(|| self.aggregator.connections().cloned().collect::<Vec<_>>());
        if let Some(current) = connections.as_ref() {
            let changed = current
                .iter()
                .filter(|connection| {
                    self.persisted_connections
                        .get(&connection.id)
                        .is_none_or(|previous| previous != *connection)
                })
                .cloned()
                .collect::<Vec<_>>();
            if let Err(error) = self.storage.save_connections(&session_id, &changed) {
                self.diagnostics.last_error = Some(error.to_string());
            } else {
                self.persisted_connections = current
                    .iter()
                    .cloned()
                    .map(|connection| (connection.id.clone(), connection))
                    .collect();
                self.last_persisted_revision = revision;
            }
        }
        let sample_interval = self.settings.operation_profile.traffic_sample_interval();
        if !force && self.last_traffic_sample.elapsed() < sample_interval {
            self.update_session_summary(now_ms);
            return;
        }
        self.last_traffic_sample = Instant::now();
        let connections = connections
            .take()
            .unwrap_or_else(|| self.aggregator.connections().cloned().collect::<Vec<_>>());
        let timestamp_ms = (now_ms / 1_000) * 1_000;
        let global_rates = if self.capture_mode == CaptureMode::Live {
            self.network_rates
        } else {
            connections
                .iter()
                .fold(TransferRates::default(), |mut total, connection| {
                    total.down_bytes_per_second += connection.rates.down_bytes_per_second;
                    total.up_bytes_per_second += connection.rates.up_bytes_per_second;
                    total.packets_per_second += connection.rates.packets_per_second;
                    total
                })
        };
        let global_source = if self.capture_mode == CaptureMode::Demo {
            CounterSource::Demo
        } else {
            CounterSource::InterfaceAggregate
        };
        let mut samples = vec![TrafficSample {
            timestamp_ms,
            session_id: Some(session_id.clone()),
            process_key: None,
            sample_interval_ms: sample_interval.as_millis().try_into().unwrap_or(u64::MAX),
            counters: counters_from_rates(global_rates),
            rates: global_rates,
            source: global_source,
        }];
        if self.diagnostics.flow_counters_available {
            let mut process_rates: HashMap<String, (TransferRates, CounterSource)> = HashMap::new();
            for connection in connections.iter().filter(|connection| connection.active) {
                let key = format!(
                    "{}:{}",
                    connection.process.name,
                    connection
                        .process
                        .pid
                        .map_or_else(|| "?".into(), |pid| pid.to_string())
                );
                let entry = process_rates
                    .entry(key)
                    .or_insert((TransferRates::default(), connection.counter_source));
                entry.0.down_bytes_per_second += connection.rates.down_bytes_per_second;
                entry.0.up_bytes_per_second += connection.rates.up_bytes_per_second;
                entry.0.packets_per_second += connection.rates.packets_per_second;
                if connection.counter_source != CounterSource::Unavailable {
                    entry.1 = connection.counter_source;
                }
            }
            samples.extend(
                process_rates
                    .into_iter()
                    .map(|(process_key, (rates, source))| TrafficSample {
                        timestamp_ms,
                        session_id: Some(session_id.clone()),
                        process_key: Some(process_key),
                        sample_interval_ms: sample_interval
                            .as_millis()
                            .try_into()
                            .unwrap_or(u64::MAX),
                        counters: counters_from_rates(rates),
                        rates,
                        source,
                    }),
            );
        }
        let mut signature = samples.clone();
        for sample in &mut signature {
            sample.timestamp_ms = 0;
        }
        let write_sample = force
            || self.last_traffic_write.elapsed() >= Duration::from_secs(60)
            || signature != self.last_traffic_signature;
        if write_sample {
            if let Err(error) = self.storage.save_traffic_samples(&samples) {
                self.diagnostics.last_error = Some(error.to_string());
            } else {
                self.last_traffic_signature = signature;
                self.last_traffic_write = Instant::now();
            }
        }
        if self.capture_mode == CaptureMode::Live
            && !self.settings.ephemeral_capture
            && (force || self.last_baseline_persist.elapsed() >= Duration::from_secs(30))
        {
            if let Err(error) = self.storage.save_baselines(self.aggregator.baselines()) {
                self.diagnostics.last_error = Some(error.to_string());
            } else {
                self.last_baseline_persist = Instant::now();
            }
        }
        self.update_session_summary(now_ms);
    }

    fn set_capture(&mut self, request: SetCaptureRequest) -> Result<CaptureSummary, CommandError> {
        // Prepare the fallible live provider and interface filter before changing any
        // running state. A failed interface lookup must not leave Live mode active
        // with `provider == None`, which would turn the useful error into the less
        // actionable "capture provider is unavailable" on the next runtime tick.
        let live_setup = if request.mode == CaptureMode::Live {
            let mut provider = system_provider()
                .map_err(|error| CommandError::recoverable("capture_unavailable", error))?;
            let discovered_interfaces = provider.interfaces();
            let vpn_active = discovered_interfaces.as_ref().is_ok_and(|interfaces| {
                interfaces
                    .iter()
                    .any(|network_interface| network_interface.is_up && network_interface.is_vpn)
            });
            let mut selected_addresses = HashSet::new();
            if let Some(selected) = request.selected_interface.as_deref() {
                let interfaces = discovered_interfaces
                    .map_err(|error| CommandError::recoverable("interface_unavailable", error))?;
                let network_interface = interfaces
                    .iter()
                    .find(|network_interface| network_interface.id == selected)
                    .ok_or_else(|| {
                        CommandError::recoverable(
                            "interface_unavailable",
                            format!("network interface '{selected}' is no longer available"),
                        )
                    })?;
                if network_interface.addresses.is_empty() {
                    return Err(CommandError::recoverable(
                        "interface_unavailable",
                        format!(
                            "network interface '{selected}' has no discoverable IP addresses; use All interfaces"
                        ),
                    ));
                }
                selected_addresses.extend(network_interface.addresses.iter().copied());
            }
            Some((provider, selected_addresses, vpn_active))
        } else {
            None
        };

        let now_ms = sabicheiro_core::now_ms();
        self.stop_current_session(now_ms);
        let baselines = if request.mode == CaptureMode::Demo {
            Default::default()
        } else {
            self.storage.load_baselines().unwrap_or_default()
        };
        self.aggregator = ConnectionAggregator::from_baselines(baselines);
        self.capture_generation = self.capture_generation.saturating_add(1).max(1);
        self.differ = SnapshotDiffer::new(self.capture_generation);
        self.correlator.clear();
        self.evidence_clock_origin = Instant::now();
        self.evidence_sequencers.clear();
        self.evidence_fusion = EvidenceFusion::default();
        self.evidence_storage_queue = BoundedEvidenceQueue::new(
            PipelineStage::Storage,
            self.settings.operation_profile.evidence_queue_capacity(),
        );
        self.persisted_connections.clear();
        self.last_persisted_revision = 0;
        self.last_traffic_signature.clear();
        self.last_traffic_sample = Instant::now();
        self.last_traffic_write = Instant::now();
        self.last_baseline_persist = Instant::now();
        self.last_event_capture_drops = 0;
        self.provider_supervisor = ProviderSupervisor::default();
        self.diagnostics.capture_generation = self.capture_generation;
        self.provider = None;
        #[cfg(target_os = "windows")]
        {
            self.event_capture = None;
        }
        self.dns_observations.clear();
        self.dns_causal_graph = DnsCausalGraph::default();
        self.dns_relations.clear();
        self.selected_interface_addresses.clear();
        self.interface_by_address.clear();
        self.network_signature.clear();
        self.last_tick_at = Instant::now();
        self.vpn_active = false;
        self.capture_mode = request.mode;
        self.selected_interface = request.selected_interface.clone();
        self.settings.selected_interface = request.selected_interface;
        self.paused_view_events = 0;
        self.view_paused = false;
        self.ipc_baseline = None;

        match request.mode {
            CaptureMode::Off => {
                self.capture_status = CaptureStatus::Stopped;
                self.diagnostics.backend = "stopped".into();
                self.diagnostics.capture_event_source = "none".into();
                self.diagnostics.flow_counters_available = false;
            }
            CaptureMode::Demo => {
                self.demo = sabicheiro_core::DemoGenerator::new(self.settings.demo_seed);
                self.last_demo_event = Instant::now() - DEMO_EVENT_INTERVAL;
                self.capture_status = CaptureStatus::Capturing;
                self.diagnostics.backend = "deterministic-demo".into();
                self.diagnostics.capture_event_source = "deterministic_demo".into();
                self.diagnostics.flow_counters_available = true;
                self.start_session(now_ms)?;
            }
            CaptureMode::Live => {
                let Some((provider, selected_addresses, vpn_active)) = live_setup else {
                    unreachable!("live capture setup is prepared before state mutation")
                };
                let snapshot_provider_id = provider.descriptor(self.capture_generation).provider_id;
                self.provider_supervisor
                    .register(snapshot_provider_id, None, now_ms);
                self.selected_interface_addresses = selected_addresses;
                self.vpn_active = vpn_active;
                self.diagnostics.backend = provider.name().into();
                self.provider = Some(provider);
                self.diagnostics.capture_event_source = "socket_polling".into();
                self.diagnostics.flow_counters_available = false;
                #[cfg(target_os = "windows")]
                {
                    self.provider_supervisor.register(
                        "windows.etw",
                        Some("windows.ip_helper".into()),
                        now_ms,
                    );
                    match WindowsEventCapture::start() {
                        Ok(capture) => {
                            self.event_capture = Some(capture);
                            self.provider_supervisor.mark_healthy("windows.etw", now_ms);
                            self.diagnostics.backend = "windows-etw + windows-ip-helper".into();
                            self.diagnostics.capture_event_source = "windows_etw".into();
                            self.diagnostics.flow_counters_available = true;
                            self.diagnostics.last_error = None;
                        }
                        Err(error) => {
                            let message = error.to_string();
                            self.provider_supervisor.mark_failure(
                                "windows.etw",
                                message.clone(),
                                now_ms,
                            );
                            self.diagnostics.last_error = Some(message);
                        }
                    }
                }
                self.capture_status = CaptureStatus::Starting;
                self.last_capture_poll = Instant::now() - LIVE_POLL_INTERVAL;
                self.start_session(now_ms)?;
            }
        }
        let _ = self.storage.save_settings(&self.settings);
        Ok(self.capture_summary())
    }

    fn start_session(&mut self, started_at_ms: i64) -> Result<(), CommandError> {
        let ephemeral = self.settings.ephemeral_capture || self.capture_mode == CaptureMode::Demo;
        let id = if self.capture_mode == CaptureMode::Demo {
            format!("demo-{started_at_ms}")
        } else if ephemeral {
            format!("ephemeral-{started_at_ms}")
        } else {
            format!("session-{started_at_ms}")
        };
        if !ephemeral {
            self.storage
                .start_session(&id, started_at_ms)
                .map_err(|error| CommandError::recoverable("storage_unavailable", error))?;
        }
        self.session = Some(SessionSummary {
            id,
            started_at_ms,
            ended_at_ms: None,
            connection_count: 0,
            process_count: 0,
            country_count: 0,
            destination_count: 0,
            recording_count: 0,
            ephemeral,
            traffic: TrafficCounters::default(),
        });
        Ok(())
    }

    fn stop_current_session(&mut self, ended_at_ms: i64) {
        if self.session.is_none() {
            return;
        }
        if self
            .session
            .as_ref()
            .is_some_and(|session| !session.ephemeral)
        {
            self.persist(ended_at_ms, true);
        }
        while self
            .evidence_storage_queue
            .stats(
                self.evidence_clock_origin
                    .elapsed()
                    .as_nanos()
                    .min(u128::from(u64::MAX)) as u64,
            )
            .depth
            > 0
        {
            if !self.flush_evidence_journal() {
                break;
            }
        }
        let session_ephemeral = self
            .session
            .as_ref()
            .is_some_and(|session| session.ephemeral);
        self.update_session_summary(ended_at_ms);
        if let Some(mut summary) = self.session.take() {
            summary.ended_at_ms = Some(ended_at_ms);
            if !summary.ephemeral
                && let Err(error) = self.storage.finish_session(&summary)
            {
                self.diagnostics.last_error = Some(error.to_string());
            }
        }
        if self.capture_mode == CaptureMode::Live && !session_ephemeral {
            if let Err(error) = self.storage.save_baselines(self.aggregator.baselines()) {
                self.diagnostics.last_error = Some(error.to_string());
            }
        }
    }

    fn update_session_summary(&mut self, _now_ms: i64) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut process_names = HashSet::new();
        let mut countries = HashSet::new();
        let mut destinations = HashSet::new();
        let mut traffic = TrafficCounters::default();
        let mut count = 0;
        for connection in self.aggregator.connections() {
            count += 1;
            process_names.insert(connection.process.name.clone());
            if let Some(geo) = &connection.geo {
                countries.insert(geo.country_code.clone());
            }
            destinations.insert(
                connection
                    .domain
                    .as_deref()
                    .or(connection.hostname.as_deref())
                    .map(str::to_owned)
                    .unwrap_or_else(|| connection.key.remote.address.to_string()),
            );
            traffic.add_assign(connection.traffic);
        }
        session.connection_count = count;
        session.process_count = process_names.len();
        session.country_count = countries.len();
        session.destination_count = destinations.len();
        session.traffic = traffic;
    }

    fn capture_summary(&self) -> CaptureSummary {
        CaptureSummary {
            mode: self.capture_mode,
            status: self.capture_status.clone(),
            selected_interface: self.selected_interface.clone(),
            paused_view_events: self.paused_view_events,
            // Interface discovery is cached when capture starts, not repeated per snapshot.
            vpn_active: self.vpn_active,
        }
    }

    fn snapshot(&mut self) -> DashboardSnapshot {
        let now_ms = sabicheiro_core::now_ms();
        self.update_session_summary(now_ms);
        let mut snapshot = self.aggregator.snapshot(
            now_ms,
            self.capture_summary(),
            self.diagnostics.clone(),
            self.session.clone(),
        );
        if self.capture_mode == CaptureMode::Live {
            snapshot.stats.down_bytes_per_second = self.network_rates.down_bytes_per_second;
            snapshot.stats.up_bytes_per_second = self.network_rates.up_bytes_per_second;
            snapshot.stats.packets_per_second = self.network_rates.packets_per_second;
        }
        snapshot
    }

    fn update(&mut self, after_sequence: Option<u64>) -> DashboardUpdate {
        let next_sequence = self.ipc_sequence.saturating_add(1);
        let expected_sequence = self.ipc_sequence;
        let client_in_sync = after_sequence == Some(expected_sequence);
        if after_sequence.is_some() && !client_in_sync {
            self.diagnostics.sequence_gaps = self.diagnostics.sequence_gaps.saturating_add(1);
            self.diagnostics.snapshot_resyncs = self.diagnostics.snapshot_resyncs.saturating_add(1);
        }
        if client_in_sync
            && self.ipc_baseline.as_ref().is_some_and(|previous| {
                previous.revision == self.aggregator.revision()
                    && previous.capture == self.capture_summary()
                    && previous.diagnostics == self.diagnostics
                    && previous.session == self.session
            })
        {
            self.ipc_sequence = next_sequence;
            let update = DashboardUpdate {
                sequence: next_sequence,
                base_revision: self.aggregator.revision(),
                revision: self.aggregator.revision(),
                generated_at_ms: sabicheiro_core::now_ms(),
                resync_required: false,
                snapshot: None,
                upserted_connections: Vec::new(),
                removed_connection_ids: Vec::new(),
                processes: None,
                destinations: None,
                stats: None,
                diagnostics: None,
                capture: None,
                session: None,
            };
            return self.record_dashboard_update(update);
        }
        let snapshot = self.snapshot();
        let baseline = self.ipc_baseline.as_ref();
        let can_delta = client_in_sync
            && baseline.is_some_and(|previous| previous.revision <= snapshot.revision);

        let (resync_required, full_snapshot, upserted_connections, removed_connection_ids) =
            if can_delta {
                let previous = baseline.expect("delta baseline checked above");
                let previous_connections = previous
                    .connections
                    .iter()
                    .map(|connection| (connection.id.as_str(), connection))
                    .collect::<HashMap<_, _>>();
                let current_ids = snapshot
                    .connections
                    .iter()
                    .map(|connection| connection.id.as_str())
                    .collect::<HashSet<_>>();
                let upserts = snapshot
                    .connections
                    .iter()
                    .filter(|connection| {
                        previous_connections
                            .get(connection.id.as_str())
                            .is_none_or(|previous| **previous != **connection)
                    })
                    .cloned()
                    .collect();
                let removals = previous
                    .connections
                    .iter()
                    .filter(|connection| !current_ids.contains(connection.id.as_str()))
                    .map(|connection| connection.id.clone())
                    .collect();
                (false, None, upserts, removals)
            } else {
                (true, Some(snapshot.clone()), Vec::new(), Vec::new())
            };

        let base_revision = baseline.map_or(0, |previous| previous.revision);
        let processes = can_delta
            .then(|| baseline.filter(|previous| previous.processes != snapshot.processes))
            .flatten()
            .map(|_| snapshot.processes.clone());
        let destinations = can_delta
            .then(|| baseline.filter(|previous| previous.destinations != snapshot.destinations))
            .flatten()
            .map(|_| snapshot.destinations.clone());
        let stats = can_delta
            .then(|| baseline.filter(|previous| previous.stats != snapshot.stats))
            .flatten()
            .map(|_| snapshot.stats.clone());
        let diagnostics = can_delta
            .then(|| baseline.filter(|previous| previous.diagnostics != snapshot.diagnostics))
            .flatten()
            .map(|_| snapshot.diagnostics.clone());
        let capture = can_delta
            .then(|| baseline.filter(|previous| previous.capture != snapshot.capture))
            .flatten()
            .map(|_| snapshot.capture.clone());
        let session = can_delta
            .then(|| baseline.filter(|previous| previous.session != snapshot.session))
            .flatten()
            .map(|_| snapshot.session.clone());
        self.ipc_sequence = next_sequence;
        self.ipc_baseline = Some(snapshot.clone());
        let update = DashboardUpdate {
            sequence: next_sequence,
            base_revision,
            revision: snapshot.revision,
            generated_at_ms: snapshot.generated_at_ms,
            resync_required,
            snapshot: full_snapshot,
            upserted_connections,
            removed_connection_ids,
            processes,
            destinations,
            stats,
            diagnostics,
            capture,
            session,
        };
        self.record_dashboard_update(update)
    }

    fn record_dashboard_update(&mut self, update: DashboardUpdate) -> DashboardUpdate {
        self.ipc_metrics.requests = self.ipc_metrics.requests.saturating_add(1);
        let bytes = serde_json::to_vec(&update)
            .map_or(0, |payload| payload.len())
            .try_into()
            .unwrap_or(u64::MAX);
        self.ipc_metrics.bytes_serialized = self.ipc_metrics.bytes_serialized.saturating_add(bytes);
        if update.snapshot.is_none()
            && update.upserted_connections.is_empty()
            && update.removed_connection_ids.is_empty()
            && update.processes.is_none()
            && update.destinations.is_none()
            && update.stats.is_none()
            && update.diagnostics.is_none()
            && update.capture.is_none()
            && update.session.is_none()
        {
            self.ipc_metrics.empty_deltas = self.ipc_metrics.empty_deltas.saturating_add(1);
        }
        update
    }

    fn refresh_diagnostics(&mut self) {
        self.diagnostics.dashboard_requests = self.ipc_metrics.requests;
        self.diagnostics.dashboard_bytes_serialized = self.ipc_metrics.bytes_serialized;
        self.diagnostics.dashboard_empty_deltas = self.ipc_metrics.empty_deltas;
        self.diagnostics.dashboard_overlaps = self.ipc_metrics.overlaps;
        if let Ok(pid) = sysinfo::get_current_pid() {
            self.process_metrics
                .refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
            if let Some(process) = self.process_metrics.process(pid) {
                self.diagnostics.cpu_percent = process.cpu_usage();
                self.diagnostics.memory_bytes = process.memory();
            }
        }
        let elapsed = self.last_diagnostics.elapsed();
        self.network_metrics.refresh(true);
        let mut received_bytes = 0_u64;
        let mut transmitted_bytes = 0_u64;
        let mut packets = 0_u64;
        for (_name, network) in &self.network_metrics {
            if !self.selected_interface_addresses.is_empty()
                && !network
                    .ip_networks()
                    .iter()
                    .any(|network| self.selected_interface_addresses.contains(&network.addr))
            {
                continue;
            }
            received_bytes = received_bytes.saturating_add(network.received());
            transmitted_bytes = transmitted_bytes.saturating_add(network.transmitted());
            packets = packets
                .saturating_add(network.packets_received())
                .saturating_add(network.packets_transmitted());
        }
        self.network_rates =
            transfer_rates_from_deltas(received_bytes, transmitted_bytes, packets, elapsed);
        self.diagnostics.database_size_bytes = self.storage.database_size_bytes();
        #[cfg(target_os = "windows")]
        let (event_depth, event_capacity) = self
            .event_capture
            .as_ref()
            .map_or((0, 0), |capture| (capture.pending(), capture.capacity()));
        #[cfg(not(target_os = "windows"))]
        let (event_depth, event_capacity) = (0, 0);
        let evidence_stats = self.evidence_storage_queue.stats(
            self.evidence_clock_origin
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64,
        );
        let evidence_dropped = evidence_stats
            .dropped_lifecycle
            .saturating_add(evidence_stats.dropped_context)
            .saturating_add(evidence_stats.dropped_repeated_metrics);
        self.diagnostics.capture_queue_depth = event_depth
            .saturating_add(self.dns.pending())
            .saturating_add(self.geo.pending())
            .saturating_add(evidence_stats.depth);
        self.diagnostics.capture_queue_capacity = event_capacity
            .saturating_add(DNS_QUEUE_CAPACITY)
            .saturating_add(GEO_QUEUE_CAPACITY)
            .saturating_add(evidence_stats.capacity);
        self.diagnostics.queue_saturation = self.diagnostics.capture_queue_depth as f32
            / self.diagnostics.capture_queue_capacity.max(1) as f32;
        self.diagnostics.dropped_ui_events =
            self.diagnostics.dropped_ui_events.max(self.geo.dropped());
        self.diagnostics.sequence_gaps = self
            .diagnostics
            .sequence_gaps
            .max(evidence_stats.dropped_lifecycle);
        if evidence_stats.resync_required {
            self.diagnostics.last_error = Some(
                "The evidence storage consumer lost lifecycle order; a provider resync is required."
                    .into(),
            );
        }
        self.diagnostics.dropped_capture_events = self
            .diagnostics
            .dropped_capture_events
            .max(evidence_dropped);
        self.refresh_subsystem_diagnostics();
    }

    fn refresh_subsystem_diagnostics(&mut self) {
        let now_ms = sabicheiro_core::now_ms();
        let capture = match &self.capture_status {
            CaptureStatus::Capturing => (
                SubsystemStatus::Healthy,
                format!("{} is capturing", self.diagnostics.backend),
                None,
            ),
            CaptureStatus::Starting => (
                SubsystemStatus::Degraded,
                "Capture is starting and reconciling socket state".into(),
                Some("Wait for capture to enter the Capturing state.".into()),
            ),
            CaptureStatus::Stopped => (
                SubsystemStatus::Disabled,
                "Capture is stopped".into(),
                Some("Start Demo or Live capture when observation is wanted.".into()),
            ),
            CaptureStatus::Error(error) => (
                SubsystemStatus::Unavailable,
                error.clone(),
                Some("Run the guided capture self-test and review provider permissions.".into()),
            ),
        };
        let geo = match self.geo.status() {
            Ok(status) if status.city.installed || status.asn.installed => (
                SubsystemStatus::Healthy,
                format!(
                    "City generation {}, ASN generation {}",
                    status.city.generation, status.asn.generation
                ),
                None,
            ),
            Ok(_) => (
                SubsystemStatus::Disabled,
                "No local GeoIP dataset is installed".into(),
                Some("Import free local MMDB datasets from Settings.".into()),
            ),
            Err(error) => (
                SubsystemStatus::Unavailable,
                error,
                Some("Rollback, remove, or re-import the affected local MMDB dataset.".into()),
            ),
        };
        let dns_active = self.settings.reverse_dns_enabled
            || self.diagnostics.capture_event_source == "windows_etw";
        let dns = if dns_active && self.dns.dropped() == 0 {
            (
                SubsystemStatus::Healthy,
                "Local DNS enrichment is active".into(),
                None,
            )
        } else if dns_active {
            (
                SubsystemStatus::Degraded,
                "The bounded DNS queue dropped requests".into(),
                Some("Reduce capture load or disable optional reverse DNS.".into()),
            )
        } else {
            (
                SubsystemStatus::Disabled,
                "Optional reverse DNS is disabled".into(),
                Some("Enable it explicitly in Settings only when hostnames are needed.".into()),
            )
        };
        let storage = if self.diagnostics.storage_quick_check_ok {
            (
                SubsystemStatus::Healthy,
                "SQLite quick_check passed".into(),
                None,
            )
        } else {
            (
                SubsystemStatus::Degraded,
                "SQLite integrity has not passed its latest check".into(),
                Some("Stop capture, export needed data, then run local maintenance.".into()),
            )
        };
        let render = if self.diagnostics.render_fps <= 0.0 {
            (
                SubsystemStatus::Disabled,
                "The UI has not reported a render sample yet".into(),
                None,
            )
        } else if self.diagnostics.render_fps < 30.0 {
            (
                SubsystemStatus::Degraded,
                format!("Render rate is {:.0} FPS", self.diagnostics.render_fps),
                Some("Use compact density, 2D map, or reduced motion for dense captures.".into()),
            )
        } else {
            (
                SubsystemStatus::Healthy,
                format!("Render rate is {:.0} FPS", self.diagnostics.render_fps),
                None,
            )
        };
        let queue_status = if self.diagnostics.queue_saturation >= 0.8
            || self.geo.dropped() > 0
            || self.dns.dropped() > 0
        {
            (
                SubsystemStatus::Degraded,
                format!(
                    "Queue utilization {:.0}%; {} enrichment jobs dropped",
                    self.diagnostics.queue_saturation * 100.0,
                    self.geo.dropped().saturating_add(self.dns.dropped())
                ),
                Some("Narrow capture scope or turn off optional DNS enrichment.".into()),
            )
        } else {
            (
                SubsystemStatus::Healthy,
                format!(
                    "{} of {} bounded queue slots used",
                    self.diagnostics.capture_queue_depth, self.diagnostics.capture_queue_capacity
                ),
                None,
            )
        };
        let evidence_stats = self.evidence_storage_queue.stats(
            self.evidence_clock_origin
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64,
        );
        let evidence_dropped = evidence_stats
            .dropped_lifecycle
            .saturating_add(evidence_stats.dropped_context)
            .saturating_add(evidence_stats.dropped_repeated_metrics);
        let journal = if evidence_stats.resync_required || evidence_dropped > 0 {
            (
                SubsystemStatus::Degraded,
                format!(
                    "Immutable journal queue dropped {evidence_dropped} observations; lifecycle drops {}",
                    evidence_stats.dropped_lifecycle
                ),
                Some(
                    "Recheck the provider and rebuild projections from retained journal evidence."
                        .into(),
                ),
            )
        } else {
            (
                SubsystemStatus::Healthy,
                format!(
                    "{} of {} journal queue slots used; oldest age {} ms",
                    evidence_stats.depth,
                    evidence_stats.capacity,
                    evidence_stats.maximum_age_ns / 1_000_000
                ),
                None,
            )
        };
        let specs = [
            (
                "capture",
                capture,
                0,
                0,
                self.diagnostics.dropped_capture_events,
            ),
            (
                "geoip",
                geo,
                self.geo.pending(),
                GEO_QUEUE_CAPACITY,
                self.geo.dropped(),
            ),
            (
                "dns",
                dns,
                self.dns.pending(),
                DNS_QUEUE_CAPACITY,
                self.dns.dropped(),
            ),
            (
                "storage",
                storage,
                evidence_stats.depth,
                evidence_stats.capacity,
                evidence_dropped,
            ),
            (
                "evidence_journal",
                journal,
                evidence_stats.depth,
                evidence_stats.capacity,
                evidence_dropped,
            ),
            ("render", render, 0, 0, self.diagnostics.dropped_ui_events),
            (
                "backpressure",
                queue_status,
                self.diagnostics.capture_queue_depth,
                self.diagnostics.capture_queue_capacity,
                self.geo.dropped().saturating_add(self.dns.dropped()),
            ),
        ];
        self.diagnostics.subsystems = specs
            .into_iter()
            .map(
                |(name, (status, message, action), depth, capacity, dropped)| {
                    let state = self
                        .subsystem_states
                        .entry(name.into())
                        .or_insert((status, now_ms));
                    if state.0 != status {
                        *state = (status, now_ms);
                    }
                    SubsystemDiagnostic {
                        subsystem: name.into(),
                        status,
                        since_ms: Some(state.1),
                        message,
                        action,
                        queue_depth: depth,
                        queue_capacity: capacity,
                        dropped_events: dropped,
                    }
                },
            )
            .collect();
        for health in self.provider_supervisor.health_snapshot() {
            let status = match health.state {
                sabicheiro_core::ProviderLifecycleState::Healthy => SubsystemStatus::Healthy,
                sabicheiro_core::ProviderLifecycleState::Starting
                | sabicheiro_core::ProviderLifecycleState::Degraded
                | sabicheiro_core::ProviderLifecycleState::Restarting
                | sabicheiro_core::ProviderLifecycleState::Fallback => SubsystemStatus::Degraded,
                sabicheiro_core::ProviderLifecycleState::Failed => SubsystemStatus::Unavailable,
                sabicheiro_core::ProviderLifecycleState::Stopped => SubsystemStatus::Disabled,
            };
            let message = health.reason.clone().unwrap_or_else(|| {
                format!(
                    "Provider state {:?}, generation {}",
                    health.state, health.generation
                )
            });
            self.diagnostics.subsystems.push(SubsystemDiagnostic {
                subsystem: format!("provider:{}", health.provider_id),
                status,
                since_ms: Some(health.since_ms),
                message,
                action: health.next_retry_ms.map(|retry| {
                    format!("Automatic bounded retry is scheduled at {retry} ms UTC.")
                }),
                queue_depth: 0,
                queue_capacity: 0,
                dropped_events: 0,
            });
        }
    }

    fn refresh_storage_health(&mut self) {
        match self.storage.health_check() {
            Ok(health) => {
                self.diagnostics.storage_quick_check_ok = health.quick_check_ok;
                self.diagnostics.storage_last_check_ms = Some(health.checked_at_ms);
                self.diagnostics.storage_wal_pages = health.wal_log_pages;
                if !health.quick_check_ok {
                    self.diagnostics.last_error =
                        Some(format!("SQLite quick_check: {}", health.detail));
                }
            }
            Err(error) => self.diagnostics.last_error = Some(error.to_string()),
        }
    }

    fn stop_expired_recording(&mut self, now_ms: i64) {
        let state = self.recording.state();
        if state.status != RecordingStatus::Recording {
            return;
        }
        let expired = state
            .duration_limit_seconds
            .zip(state.started_at_ms)
            .is_some_and(|(seconds, started)| {
                now_ms.saturating_sub(started)
                    >= (seconds.min(i64::MAX as u64 / 1_000) * 1_000) as i64
            });
        if expired || self.recording.limit_reached() {
            if let Err(error) = self.stop_recording() {
                self.diagnostics.last_error = Some(error.message);
            }
        }
    }

    fn start_recording(
        &mut self,
        request: RecordingRequest,
    ) -> Result<RecordingState, CommandError> {
        let state = self
            .recording
            .start(request)
            .map_err(|error| CommandError::recoverable("recording_failed", error))?;
        let session_id = self
            .session
            .as_ref()
            .filter(|session| !session.ephemeral)
            .map(|session| session.id.as_str());
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.ephemeral)
        {
            return Ok(state);
        }
        match self.storage.start_recording(session_id, &state) {
            Ok(id) => {
                self.active_recording_id = Some(id);
                if let Some(session) = self.session.as_mut() {
                    session.recording_count = session.recording_count.saturating_add(1);
                }
            }
            Err(error) => self.diagnostics.last_error = Some(error.to_string()),
        }
        Ok(state)
    }

    fn stop_recording(&mut self) -> Result<RecordingState, CommandError> {
        match self.recording.stop() {
            Ok(state) => {
                self.finish_recording_index(&state, true);
                Ok(state)
            }
            Err(error) => {
                // Preserve a failed conversion/stop in history instead of leaving an
                // apparently active recording row forever.
                let state = self.recording.state();
                self.finish_recording_index(&state, false);
                Err(CommandError::recoverable("recording_failed", error))
            }
        }
    }

    fn finish_recording_index(&mut self, state: &RecordingState, clear: bool) {
        if let Some(id) = self.active_recording_id
            && let Err(error) = self
                .storage
                .finish_recording(id, state, sabicheiro_core::now_ms())
        {
            self.diagnostics.last_error = Some(error.to_string());
        }
        if clear {
            self.active_recording_id = None;
        }
    }
}

#[tauri::command]
fn get_dashboard_update(
    after_sequence: Option<u64>,
    state: State<'_, AppState>,
) -> Result<DashboardUpdate, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))
        .map(|mut engine| engine.update(after_sequence))
}

#[tauri::command]
fn set_capture_state(
    request: SetCaptureRequest,
    state: State<'_, AppState>,
) -> Result<CaptureSummary, CommandError> {
    let result = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .set_capture(request);
    state.0.wake();
    result
}

#[tauri::command]
fn list_interfaces(
    state: State<'_, AppState>,
) -> Result<Vec<sabicheiro_core::NetworkInterface>, CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    if let Some(provider) = engine.provider.as_mut() {
        provider
            .interfaces()
            .map_err(|error| CommandError::recoverable("interface_unavailable", error))
    } else {
        system_provider()
            .and_then(|mut provider| provider.interfaces())
            .map_err(|error| CommandError::recoverable("interface_unavailable", error))
    }
}

struct LocalSocketProof {
    protocol: NetworkProtocol,
    family: AddressFamily,
    local_ports: Vec<u16>,
    setup_error: Option<String>,
}

type LocalSocketHandles = Vec<Box<dyn std::any::Any>>;
type LocalSocketProofResult = std::io::Result<(Vec<u16>, LocalSocketHandles)>;

fn open_tcp_loopback_proof(address: IpAddr) -> LocalSocketProofResult {
    let listener = std::net::TcpListener::bind(std::net::SocketAddr::new(address, 0))?;
    let listener_address = listener.local_addr()?;
    let client = std::net::TcpStream::connect(listener_address)?;
    let client_address = client.local_addr()?;
    let (server, _) = listener.accept()?;
    Ok((
        vec![listener_address.port(), client_address.port()],
        vec![Box::new(listener), Box::new(client), Box::new(server)],
    ))
}

fn open_udp_loopback_proof(address: IpAddr) -> LocalSocketProofResult {
    let server = std::net::UdpSocket::bind(std::net::SocketAddr::new(address, 0))?;
    let client = std::net::UdpSocket::bind(std::net::SocketAddr::new(address, 0))?;
    let server_address = server.local_addr()?;
    let client_address = client.local_addr()?;
    client.connect(server_address)?;
    server.connect(client_address)?;
    client.send(b"sabicheiro-local-self-test")?;
    let mut proof = [0_u8; 64];
    let _ = server.recv(&mut proof)?;
    Ok((
        vec![server_address.port(), client_address.port()],
        vec![Box::new(server), Box::new(client)],
    ))
}

fn open_local_socket_proofs() -> (Vec<LocalSocketProof>, LocalSocketHandles) {
    let specifications = [
        (
            NetworkProtocol::Tcp,
            AddressFamily::Ipv4,
            open_tcp_loopback_proof(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        ),
        (
            NetworkProtocol::Udp,
            AddressFamily::Ipv4,
            open_udp_loopback_proof(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        ),
        (
            NetworkProtocol::Tcp,
            AddressFamily::Ipv6,
            open_tcp_loopback_proof(IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
        ),
        (
            NetworkProtocol::Udp,
            AddressFamily::Ipv6,
            open_udp_loopback_proof(IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
        ),
    ];
    let mut capabilities = Vec::with_capacity(specifications.len());
    let mut handles = Vec::new();
    for (protocol, family, result) in specifications {
        match result {
            Ok((local_ports, mut proof_handles)) => {
                capabilities.push(LocalSocketProof {
                    protocol,
                    family,
                    local_ports,
                    setup_error: None,
                });
                handles.append(&mut proof_handles);
            }
            Err(error) => capabilities.push(LocalSocketProof {
                protocol,
                family,
                local_ports: Vec::new(),
                setup_error: Some(error.to_string()),
            }),
        }
    }
    (capabilities, handles)
}

fn address_family(address: IpAddr) -> AddressFamily {
    match address {
        IpAddr::V4(_) => AddressFamily::Ipv4,
        IpAddr::V6(_) => AddressFamily::Ipv6,
    }
}

#[tauri::command]
fn run_capture_self_test(state: State<'_, AppState>) -> Result<CaptureSelfTest, CommandError> {
    let mut checks = Vec::new();
    let mut provider = system_provider()
        .map_err(|error| CommandError::recoverable("capture_unavailable", error))?;
    match provider.interfaces() {
        Ok(interfaces)
            if interfaces
                .iter()
                .any(|item| item.is_up && !item.addresses.is_empty()) =>
        {
            checks.push(SelfTestCheck {
                id: "interfaces".into(),
                level: SelfTestLevel::Pass,
                title: "Network interfaces are discoverable".into(),
                detail: format!(
                    "{} adapters found with stable local identifiers",
                    interfaces.len()
                ),
                diagnostic_code: None,
                action: None,
            });
        }
        Ok(interfaces) => checks.push(SelfTestCheck {
            id: "interfaces".into(),
            level: SelfTestLevel::Warn,
            title: "No active addressed interface was found".into(),
            detail: format!(
                "{} adapters were returned; reconnect the network or select All interfaces",
                interfaces.len()
            ),
            diagnostic_code: Some("interface_no_active_address".into()),
            action: Some("Reconnect the network or select All interfaces, then retry.".into()),
        }),
        Err(error) => checks.push(SelfTestCheck {
            id: "interfaces".into(),
            level: SelfTestLevel::Fail,
            title: "Interface discovery failed".into(),
            detail: error.to_string(),
            diagnostic_code: Some("interface_unavailable".into()),
            action: Some("Check operating-system capture permissions and retry.".into()),
        }),
    }

    let (proofs, proof_handles) = open_local_socket_proofs();
    let socket_snapshot = provider.snapshot();
    let sockets = socket_snapshot.as_ref().ok();
    match &socket_snapshot {
        Ok(sockets) => checks.push(SelfTestCheck {
            id: "socket_table".into(),
            level: SelfTestLevel::Pass,
            title: "Socket tables are readable".into(),
            detail: format!(
                "{} TCP/UDP rows observed during controlled local loopback proofs",
                sockets.len()
            ),
            diagnostic_code: None,
            action: None,
        }),
        Err(error) => checks.push(SelfTestCheck {
            id: "socket_table".into(),
            level: SelfTestLevel::Fail,
            title: "Socket table proof failed".into(),
            detail: error.to_string(),
            diagnostic_code: Some("socket_table_unavailable".into()),
            action: Some("Check capture permissions and retry the guided test.".into()),
        }),
    }
    let attributed = sockets.is_some_and(|sockets| {
        sockets.iter().any(|socket| {
            proofs.iter().any(|proof| {
                proof.protocol == socket.identity.protocol
                    && proof.family == address_family(socket.identity.local.address)
                    && proof.local_ports.contains(&socket.identity.local.port)
            }) && socket.identity.pid == Some(std::process::id())
                && socket.process.name != "Unknown process"
        })
    });
    checks.push(SelfTestCheck {
        id: "process_attribution".into(),
        level: if attributed {
            SelfTestLevel::Pass
        } else {
            SelfTestLevel::Warn
        },
        title: if attributed {
            "Process attribution is working".into()
        } else {
            "The proof socket could not be attributed".into()
        },
        detail: if attributed {
            "The current Sabicheiro process was resolved from controlled loopback sockets.".into()
        } else {
            "Socket visibility works, but the operating system did not expose the proof PID in time."
                .into()
        },
        diagnostic_code: (!attributed).then(|| "process_attribution_partial".into()),
        action: (!attributed).then(|| {
            "Run the test again; if it persists, review the documented capture permissions."
                .into()
        }),
    });

    let mut differ = SnapshotDiffer::new(1);
    if let Ok(before) = &socket_snapshot {
        let _ = differ.diff(before.clone(), sabicheiro_core::now_ms());
    }
    drop(proof_handles);
    thread::sleep(Duration::from_millis(30));
    let after_close = provider.snapshot().unwrap_or_default();
    let close_events = differ.diff(after_close, sabicheiro_core::now_ms());

    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let passive_dns = engine.diagnostics.capture_event_source == "windows_etw";
    checks.push(SelfTestCheck {
        id: "dns".into(),
        level: if passive_dns || engine.settings.reverse_dns_enabled {
            SelfTestLevel::Pass
        } else {
            SelfTestLevel::Warn
        },
        title: if passive_dns {
            "Passive DNS correlation is active".into()
        } else if engine.settings.reverse_dns_enabled {
            "Opt-in reverse DNS is enabled".into()
        } else {
            "DNS enrichment is intentionally idle".into()
        },
        detail: if passive_dns {
            "Windows DNS Client answers are correlated locally with an expiring bounded cache."
                .into()
        } else {
            "Start Live capture with ETW or explicitly enable reverse DNS to enrich domains.".into()
        },
        diagnostic_code: (!passive_dns && !engine.settings.reverse_dns_enabled)
            .then(|| "dns_enrichment_disabled".into()),
        action: (!passive_dns && !engine.settings.reverse_dns_enabled).then(|| {
            "Enable reverse DNS in Settings only if hostname enrichment is wanted.".into()
        }),
    });
    let geo = engine
        .geo
        .status()
        .map_err(|error| CommandError::recoverable("geo_status_unavailable", error))?;
    checks.push(SelfTestCheck {
        id: "geoip".into(),
        level: if geo.installed { SelfTestLevel::Pass } else { SelfTestLevel::Warn },
        title: if geo.installed {
            "Local GeoIP database is ready".into()
        } else {
            "Local GeoIP database is not installed".into()
        },
        detail: if geo.installed {
            format!("{} / checksum {}", geo.dataset, geo.sha256.unwrap_or_else(|| "unavailable".into()))
        } else {
            "Download the free dataset or import a City MMDB from Settings; observed IPs are never uploaded.".into()
        },
        diagnostic_code: (!geo.installed).then(|| "geoip_not_installed".into()),
        action: (!geo.installed)
            .then(|| "Import a free local City/ASN MMDB or use the explicit free dataset update.".into()),
    });
    checks.push(SelfTestCheck {
        id: "storage".into(),
        level: SelfTestLevel::Pass,
        title: "Local history storage is available".into(),
        detail: format!(
            "SQLite is writable; current file size is {} bytes",
            engine.storage.database_size_bytes()
        ),
        diagnostic_code: None,
        action: None,
    });
    let coverage = proofs
        .into_iter()
        .map(|proof| {
            let observed = sockets
                .into_iter()
                .flatten()
                .filter(|socket| {
                    socket.identity.protocol == proof.protocol
                        && address_family(socket.identity.local.address) == proof.family
                        && proof.local_ports.contains(&socket.identity.local.port)
                })
                .collect::<Vec<_>>();
            let endpoint = if proof.setup_error.is_some() {
                SelfTestLevel::Warn
            } else if socket_snapshot.is_err() || observed.is_empty() {
                SelfTestLevel::Fail
            } else {
                SelfTestLevel::Pass
            };
            let process = if proof.setup_error.is_some() {
                SelfTestLevel::Warn
            } else if observed
                .iter()
                .any(|socket| socket.identity.pid == Some(std::process::id()))
            {
                SelfTestLevel::Pass
            } else {
                SelfTestLevel::Warn
            };
            let interface = if proof.setup_error.is_some() {
                SelfTestLevel::Warn
            } else if observed.iter().any(|socket| socket.interface.is_some()) {
                SelfTestLevel::Pass
            } else {
                SelfTestLevel::Warn
            };
            let close_event = if proof.setup_error.is_some() {
                SelfTestLevel::Warn
            } else if close_events.iter().any(|event| {
                event.kind == NetworkEventKind::Closed
                    && event.key.protocol == proof.protocol
                    && address_family(event.key.local.address) == proof.family
                    && proof.local_ports.contains(&event.key.local.port)
            }) {
                SelfTestLevel::Pass
            } else {
                SelfTestLevel::Warn
            };
            let mut limitations = Vec::new();
            if let Some(error) = proof.setup_error {
                limitations.push(format!("Local loopback fixture unavailable: {error}"));
            }
            if endpoint == SelfTestLevel::Fail {
                limitations.push(
                    "The native provider did not expose the controlled local endpoint".into(),
                );
            }
            if process == SelfTestLevel::Warn && !observed.is_empty() {
                limitations
                    .push("The operating system did not expose the proof process identity".into());
            }
            if interface == SelfTestLevel::Warn && !observed.is_empty() {
                limitations
                    .push("The provider did not expose an interface for this loopback row".into());
            }
            if close_event == SelfTestLevel::Warn && !observed.is_empty() {
                limitations
                    .push("The close was not visible within the bounded 30 ms proof window".into());
            }
            if proof.protocol == NetworkProtocol::Udp && !observed.is_empty() {
                limitations.push(
                    "Bound UDP rows stay local/listener evidence when the OS omits their peer"
                        .into(),
                );
            }
            CaptureCapability {
                protocol: proof.protocol,
                family: proof.family,
                endpoint,
                process,
                interface,
                close_event,
                limitation: (!limitations.is_empty()).then(|| limitations.join("; ")),
            }
        })
        .collect::<Vec<_>>();
    let capability_level = if coverage.iter().any(|capability| {
        [
            capability.endpoint,
            capability.process,
            capability.interface,
            capability.close_event,
        ]
        .contains(&SelfTestLevel::Fail)
    }) {
        SelfTestLevel::Fail
    } else if coverage.iter().any(|capability| {
        [
            capability.endpoint,
            capability.process,
            capability.interface,
            capability.close_event,
        ]
        .contains(&SelfTestLevel::Warn)
    }) {
        SelfTestLevel::Warn
    } else {
        SelfTestLevel::Pass
    };
    checks.push(SelfTestCheck {
        id: "capability_matrix".into(),
        level: capability_level,
        title: "TCP/UDP and IPv4/IPv6 capability proof".into(),
        detail: format!(
            "{} of {} local protocol/family fixtures exposed their endpoints",
            coverage
                .iter()
                .filter(|capability| capability.endpoint == SelfTestLevel::Pass)
                .count(),
            coverage.len()
        ),
        diagnostic_code: (capability_level != SelfTestLevel::Pass)
            .then(|| "capture_capability_partial".into()),
        action: (capability_level != SelfTestLevel::Pass).then(|| {
            "Review the capability rows; retry with the documented permissions or enable the missing local IP stack."
                .into()
        }),
    });
    let mut report_lines = checks
        .iter()
        .map(|check| format!("{}: {:?} - {}", check.id, check.level, check.title))
        .collect::<Vec<_>>();
    report_lines.extend(coverage.iter().map(|capability| {
        format!(
            "{:?}/{:?}: endpoint={:?}, process={:?}, interface={:?}, close={:?}",
            capability.protocol,
            capability.family,
            capability.endpoint,
            capability.process,
            capability.interface,
            capability.close_event
        )
    }));
    let redacted_report = report_lines.join("\n");
    Ok(CaptureSelfTest {
        generated_at_ms: sabicheiro_core::now_ms(),
        checks,
        coverage,
        redacted_report,
    })
}

#[tauri::command]
fn set_view_paused(paused: bool, state: State<'_, AppState>) -> Result<(), CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine.view_paused = paused;
    if paused {
        engine.paused_at_revision = engine.aggregator.revision();
        engine.paused_view_events = 0;
    } else {
        engine.paused_view_events = 0;
    }
    Ok(())
}

#[tauri::command]
fn query_history(
    query: HistoryQuery,
    state: State<'_, AppState>,
) -> Result<HistoryResponse, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    if engine.capture_mode == CaptureMode::Demo
        && engine.session.as_ref().is_some_and(|session| {
            query
                .session_id
                .as_ref()
                .is_none_or(|requested| requested == &session.id)
        })
    {
        let filter = query
            .query
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::parse::<ConnectionFilter>)
            .transpose()
            .map_err(|error| CommandError::recoverable("invalid_filter", error))?;
        let session = engine.session.clone().expect("demo session checked above");
        let mut connections = engine
            .aggregator
            .connections()
            .filter(|connection| {
                query
                    .from_ms
                    .is_none_or(|from| connection.last_seen_ms >= from)
                    && query.to_ms.is_none_or(|to| connection.first_seen_ms <= to)
                    && filter
                        .as_ref()
                        .is_none_or(|candidate| candidate.matches(connection))
                    && query.cursor.as_ref().is_none_or(|cursor| {
                        connection.last_seen_ms < cursor.last_seen_ms
                            || (connection.last_seen_ms == cursor.last_seen_ms
                                && connection.id < cursor.connection_id)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        connections.sort_by(|left, right| {
            right
                .last_seen_ms
                .cmp(&left.last_seen_ms)
                .then_with(|| right.id.cmp(&left.id))
        });
        let limit = query.limit.clamp(1, 5_000);
        let has_more = connections.len() > limit;
        connections.truncate(limit);
        let next_cursor = has_more.then(|| {
            let last = connections.last().expect("non-empty paginated demo page");
            HistoryCursor {
                last_seen_ms: last.last_seen_ms,
                connection_id: last.id.clone(),
            }
        });
        return Ok(HistoryResponse {
            entries: connections
                .into_iter()
                .map(|connection| HistoryEntry {
                    session_id: session.id.clone(),
                    connection,
                })
                .collect(),
            sessions: vec![session],
            recordings: Vec::new(),
            next_cursor,
            has_more,
        });
    }
    let page = engine
        .storage
        .query_history_page(&query)
        .map_err(|error| CommandError::recoverable("history_unavailable", error))?;
    let sessions = engine
        .storage
        .list_sessions(100)
        .map_err(|error| CommandError::recoverable("history_unavailable", error))?;
    let recordings = engine
        .storage
        .list_recordings(query.session_id.as_deref(), 1_000)
        .map_err(|error| CommandError::recoverable("history_unavailable", error))?;
    Ok(HistoryResponse {
        entries: page.entries,
        sessions,
        recordings,
        next_cursor: page.next_cursor,
        has_more: page.has_more,
    })
}

#[tauri::command]
fn save_investigation(
    mut investigation: SavedInvestigation,
    state: State<'_, AppState>,
) -> Result<SavedInvestigation, CommandError> {
    let now_ms = sabicheiro_core::now_ms();
    if investigation.id.trim().is_empty() {
        investigation.id = format!("investigation-{now_ms}");
    }
    if investigation.created_at_ms <= 0 {
        investigation.created_at_ms = now_ms;
    }
    investigation.updated_at_ms = now_ms;
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let prior_revision = engine
        .storage
        .list_investigations()
        .ok()
        .and_then(|items| {
            items
                .into_iter()
                .find(|item| item.id == investigation.id)
                .map(|item| item.autosave_revision)
        })
        .unwrap_or(0);
    investigation.autosave_revision = investigation
        .autosave_revision
        .max(prior_revision)
        .saturating_add(1);
    engine
        .storage
        .save_investigation(&investigation)
        .map_err(|error| CommandError::recoverable("investigation_invalid", error))?;
    Ok(investigation)
}

#[tauri::command]
fn list_investigations(
    state: State<'_, AppState>,
) -> Result<Vec<SavedInvestigation>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .list_investigations()
        .map_err(|error| CommandError::recoverable("investigation_unavailable", error))
}

#[tauri::command]
fn delete_investigation(id: String, state: State<'_, AppState>) -> Result<bool, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .delete_investigation(&id)
        .map_err(|error| CommandError::recoverable("investigation_unavailable", error))
}

#[tauri::command]
fn save_local_rule(rule: LocalRule, state: State<'_, AppState>) -> Result<LocalRule, CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine
        .storage
        .save_local_rule(&rule)
        .map_err(|error| CommandError::recoverable("local_rule_invalid", error))?;
    engine.local_rules = engine
        .storage
        .list_local_rules()
        .map_err(|error| CommandError::recoverable("local_rule_unavailable", error))?;
    Ok(rule)
}

#[tauri::command]
fn list_local_rules(state: State<'_, AppState>) -> Result<Vec<LocalRule>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .list_local_rules()
        .map_err(|error| CommandError::recoverable("local_rule_unavailable", error))
}

#[tauri::command]
fn delete_local_rule(id: String, state: State<'_, AppState>) -> Result<bool, CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let removed = engine
        .storage
        .delete_local_rule(&id)
        .map_err(|error| CommandError::recoverable("local_rule_unavailable", error))?;
    if removed {
        engine.local_rules.retain(|rule| rule.id != id);
    }
    Ok(removed)
}

#[tauri::command]
fn preview_local_rule(
    rule: LocalRule,
    mut query: HistoryQuery,
    state: State<'_, AppState>,
) -> Result<Vec<RuleMatch>, CommandError> {
    rule.validate()
        .map_err(|error| CommandError::recoverable("local_rule_invalid", error))?;
    query.limit = query.limit.clamp(1, 100_000);
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let context = RuleContext {
        network_profile_id: engine.active_network_profile_id.clone(),
        path_kind: if engine.vpn_active {
            NetworkPathKind::Vpn
        } else {
            NetworkPathKind::Unknown
        },
        listener_scope: ListenerScope::Unknown,
    };
    let entries = engine
        .storage
        .query_history(&query)
        .map_err(|error| CommandError::recoverable("local_rule_preview_failed", error))?;
    Ok(entries
        .iter()
        .map(|entry| rule.evaluate(&entry.connection, &context))
        .filter(|rule_match| rule_match.matched)
        .collect())
}

#[tauri::command]
fn list_notifications(
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<LocalNotificationRecord>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .list_notifications(limit)
        .map_err(|error| CommandError::recoverable("notifications_unavailable", error))
}

#[tauri::command]
fn update_notification_state(
    id: String,
    notification_state: NotificationState,
    snoozed_until_ms: Option<i64>,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let updated = engine
        .storage
        .update_notification_state(&id, notification_state, snoozed_until_ms)
        .map_err(|error| CommandError::recoverable("notifications_unavailable", error))?;
    if updated {
        engine.notification_center = NotificationCenterEngine::from_records(
            10_000,
            engine
                .storage
                .list_notifications(10_000)
                .map_err(|error| CommandError::recoverable("notifications_unavailable", error))?,
        );
    }
    Ok(updated)
}

#[tauri::command]
fn list_network_profiles(state: State<'_, AppState>) -> Result<Vec<NetworkProfile>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .list_network_profiles()
        .map_err(|error| CommandError::recoverable("network_profiles_unavailable", error))
}

#[tauri::command]
fn rename_network_profile(
    id: String,
    display_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<NetworkProfile, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let mut profile = engine
        .storage
        .list_network_profiles()
        .map_err(|error| CommandError::recoverable("network_profiles_unavailable", error))?
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| {
            CommandError::recoverable(
                "network_profile_not_found",
                "the pseudonymous local network profile was not found",
            )
        })?;
    profile.display_name = display_name
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    if profile
        .display_name
        .as_ref()
        .is_some_and(|name| name.chars().count() > 80)
    {
        return Err(CommandError::recoverable(
            "network_profile_invalid",
            "network profile names are limited to 80 characters",
        ));
    }
    engine
        .storage
        .save_network_profile(&profile, sabicheiro_core::now_ms())
        .map_err(|error| CommandError::recoverable("network_profile_invalid", error))?;
    Ok(profile)
}

#[tauri::command]
fn save_today_card_state(
    card: TodayCardState,
    state: State<'_, AppState>,
) -> Result<TodayCardState, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .save_today_card_state(&card)
        .map_err(|error| CommandError::recoverable("today_state_unavailable", error))?;
    Ok(card)
}

#[tauri::command]
fn list_today_card_states(state: State<'_, AppState>) -> Result<Vec<TodayCardState>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .list_today_card_states()
        .map_err(|error| CommandError::recoverable("today_state_unavailable", error))
}

#[tauri::command]
fn capture_listener_snapshot(state: State<'_, AppState>) -> Result<ListenerSnapshot, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let observed_at_ms = sabicheiro_core::now_ms();
    let listeners = engine
        .aggregator
        .connections()
        .filter(|connection| connection.state == sabicheiro_core::ConnectionState::Listening)
        .map(|connection| {
            ListenerExposure::classify(
                connection.key.local.clone(),
                connection.protocol,
                connection.key.process_instance.clone(),
                engine.vpn_active,
                observed_at_ms,
            )
        })
        .collect::<Vec<_>>();
    engine
        .storage
        .save_listener_snapshot(
            engine.session.as_ref().map(|session| session.id.as_str()),
            observed_at_ms,
            &listeners,
        )
        .map_err(|error| CommandError::recoverable("listener_snapshot_failed", error))
}

#[tauri::command]
fn list_listener_snapshots(
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<ListenerSnapshot>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .list_listener_snapshots(limit)
        .map_err(|error| CommandError::recoverable("listener_snapshot_failed", error))
}

#[tauri::command]
fn get_connection_context(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionContextResponse, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let connection = engine
        .aggregator
        .connections()
        .find(|connection| connection.id == connection_id)
        .cloned()
        .or_else(|| {
            engine
                .storage
                .query_history(&HistoryQuery {
                    limit: 100_000,
                    ..HistoryQuery::default()
                })
                .ok()?
                .into_iter()
                .find(|entry| entry.connection.id == connection_id)
                .map(|entry| entry.connection)
        })
        .ok_or_else(|| {
            CommandError::recoverable("connection_not_found", "connection evidence was not found")
        })?;
    let profile_id = engine
        .active_network_profile_id
        .clone()
        .unwrap_or_else(|| "network-unknown".into());
    let kind = if engine.vpn_active {
        NetworkPathKind::Vpn
    } else if connection.key.remote.scope == AddressScope::Global {
        NetworkPathKind::Gateway
    } else {
        NetworkPathKind::Direct
    };
    let listener = (connection.state == sabicheiro_core::ConnectionState::Listening).then(|| {
        ListenerExposure::classify(
            connection.key.local.clone(),
            connection.protocol,
            connection.key.process_instance.clone(),
            engine.vpn_active,
            connection.last_seen_ms,
        )
    });
    let service_hint =
        sabicheiro_core::service_hint(connection.protocol, connection.key.remote.port);
    let registrable_domain = connection
        .domain
        .as_deref()
        .or(connection.hostname.as_deref())
        .and_then(sabicheiro_core::registrable_domain);
    let path = lookup_network_path(
        connection.key.remote.address,
        profile_id.clone(),
        engine.capture_generation,
        engine.vpn_active,
        connection.last_seen_ms,
    )
    .unwrap_or_else(|error| NetworkPathEvidence {
        id: format!(
            "route:{}:{}:{}",
            profile_id,
            connection.interface.as_deref().unwrap_or("unknown"),
            engine.capture_generation
        ),
        destination_prefix: connection.key.remote.address.to_string(),
        prefix_length: if connection.key.remote.address.is_ipv4() {
            32
        } else {
            128
        },
        gateway_pseudonym: None,
        interface_id: connection.interface.clone(),
        metric: None,
        network_generation: engine.capture_generation,
        profile_id,
        kind,
        observed_at_ms: connection.last_seen_ms,
        source: connection.evidence.observation_source,
        quality: CorrelationQuality::Candidate,
        limitation: Some(format!(
            "Native route inspection was unavailable ({error}); this is an interface-based estimate."
        )),
    });
    Ok(ConnectionContextResponse {
        connection_id: connection.id,
        dns: engine.dns_relations.get(&connection.key).cloned(),
        path,
        listener,
        service_hint,
        registrable_domain,
    })
}

#[cfg(test)]
fn build_asn_profile(number: u32, entries: &[HistoryEntry]) -> sabicheiro_core::AsnProfile {
    let mut profile = sabicheiro_core::AsnProfile {
        number,
        ..sabicheiro_core::AsnProfile::default()
    };
    let mut ips = BTreeSet::new();
    let mut processes = BTreeSet::new();
    let mut sessions = BTreeSet::new();
    let mut countries = BTreeSet::new();
    for entry in entries.iter().filter(|entry| {
        entry
            .connection
            .asn
            .as_ref()
            .is_some_and(|asn| asn.number == number)
    }) {
        let connection = &entry.connection;
        let asn = connection.asn.as_ref().expect("filtered ASN");
        profile.organization = profile
            .organization
            .take()
            .or_else(|| asn.organization.clone());
        ips.insert(connection.key.remote.address);
        processes.insert(connection.process.name.clone());
        sessions.insert(entry.session_id.clone());
        profile.connection_ids.push(connection.id.clone());
        if let Some(country) = connection.geo.as_ref().and_then(|geo| geo.country_label()) {
            countries.insert(country);
        }
        profile.traffic.add_assign(connection.traffic);
        profile.first_seen_ms = Some(
            profile
                .first_seen_ms
                .map_or(connection.first_seen_ms, |value| {
                    value.min(connection.first_seen_ms)
                }),
        );
        profile.last_seen_ms = Some(
            profile
                .last_seen_ms
                .map_or(connection.last_seen_ms, |value| {
                    value.max(connection.last_seen_ms)
                }),
        );
    }
    profile.observed_ips = ips.into_iter().collect();
    profile.process_names = processes.into_iter().collect();
    profile.session_ids = sessions.into_iter().collect();
    profile.estimated_countries = countries.into_iter().collect();
    profile.connection_ids.sort();
    profile.connection_ids.dedup();
    profile
}

#[tauri::command]
fn get_identity_timeline(
    connection_id: String,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<IdentityTimelineEvent>, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let entries = engine
        .storage
        .query_history(&HistoryQuery {
            session_id,
            limit: 100_000,
            ..HistoryQuery::default()
        })
        .map_err(|error| CommandError::recoverable("timeline_unavailable", error))?;
    let entry = entries
        .into_iter()
        .find(|entry| entry.connection.id == connection_id)
        .ok_or_else(|| {
            CommandError::recoverable("timeline_not_found", "saved connection was not found")
        })?;
    Ok(build_identity_timeline(&entry))
}

fn build_identity_timeline(entry: &HistoryEntry) -> Vec<IdentityTimelineEvent> {
    let connection = &entry.connection;
    let mut events = vec![
        IdentityTimelineEvent {
            id: format!("{}-process", connection.id),
            timestamp_ms: connection.first_seen_ms,
            kind: IdentityEventKind::ProcessObserved,
            title: "Process instance observed".into(),
            detail: connection
                .key
                .process_instance
                .clone()
                .unwrap_or_else(|| "Process instance was not available".into()),
            source: connection.evidence.observation_source,
            confidence: connection.evidence.correlation_quality,
            connection_id: connection.id.clone(),
            session_id: Some(entry.session_id.clone()),
        },
        IdentityTimelineEvent {
            id: format!("{}-opened", connection.id),
            timestamp_ms: connection.first_seen_ms,
            kind: IdentityEventKind::ConnectionOpened,
            title: "Connection opened".into(),
            detail: format!(
                "{} -> {} ({})",
                connection.key.local.label(),
                connection.key.remote.label(),
                connection.evidence.role_reason
            ),
            source: connection.evidence.observation_source,
            confidence: connection.evidence.correlation_quality,
            connection_id: connection.id.clone(),
            session_id: Some(entry.session_id.clone()),
        },
    ];
    if let Some(hostname) = connection
        .domain
        .as_deref()
        .or(connection.hostname.as_deref())
    {
        events.push(IdentityTimelineEvent {
            id: format!("{}-dns", connection.id),
            timestamp_ms: connection.first_seen_ms,
            kind: IdentityEventKind::DnsResolved,
            title: "Local DNS name correlated".into(),
            detail: hostname.into(),
            source: ObservationSource::LocalDns,
            confidence: sabicheiro_core::CorrelationQuality::Correlated,
            connection_id: connection.id.clone(),
            session_id: Some(entry.session_id.clone()),
        });
    }
    if let Some(geo) = &connection.geo {
        events.push(IdentityTimelineEvent {
            id: format!("{}-geo", connection.id),
            timestamp_ms: connection.last_seen_ms,
            kind: IdentityEventKind::GeoEnriched,
            title: "Estimated network location enriched".into(),
            detail: format!(
                "{}; {:?}; generation {}",
                geo.country_label().unwrap_or_else(|| "Unknown".into()),
                geo.precision,
                geo.city_generation
            ),
            source: ObservationSource::LocalMmdb,
            confidence: sabicheiro_core::CorrelationQuality::Correlated,
            connection_id: connection.id.clone(),
            session_id: Some(entry.session_id.clone()),
        });
    }
    if !connection.active {
        events.push(IdentityTimelineEvent {
            id: format!("{}-closed", connection.id),
            timestamp_ms: connection.last_seen_ms,
            kind: IdentityEventKind::ConnectionClosed,
            title: "Connection closed".into(),
            detail: format!("Final state: {:?}", connection.state),
            source: connection.evidence.observation_source,
            confidence: connection.evidence.correlation_quality,
            connection_id: connection.id.clone(),
            session_id: Some(entry.session_id.clone()),
        });
    }
    events.sort_by_key(|event| (event.timestamp_ms, event.id.clone()));
    events
}

#[tauri::command]
fn query_traffic_timeline(
    query: TimelineQuery,
    state: State<'_, AppState>,
) -> Result<Vec<TrafficSample>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .query_timeline(&query)
        .map_err(|error| CommandError::recoverable("timeline_unavailable", error))
}

#[tauri::command]
fn compare_sessions(
    left_session_id: String,
    right_session_id: String,
    state: State<'_, AppState>,
) -> Result<SessionComparison, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    build_session_comparison(&engine.storage, &left_session_id, &right_session_id)
}

#[tauri::command]
fn export_session_comparison(
    request: ComparisonExportRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, CommandError> {
    let runtime = state.0.clone();
    let path = prepare_output_path(&runtime.data_directory, &request.path)?;
    let expected_extension = match request.format {
        ExportFormat::Csv => "csv",
        ExportFormat::Json => "json",
    };
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case(expected_extension))
    {
        return Err(CommandError::recoverable(
            "output_path_invalid",
            format!("comparison export must use .{expected_extension}"),
        ));
    }
    if path.exists() {
        return Err(CommandError::recoverable(
            "output_path_exists",
            "comparison export already exists; choose a new filename",
        ));
    }
    let comparison = {
        let engine = runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
        build_session_comparison(
            &engine.storage,
            &request.left_session_id,
            &request.right_session_id,
        )?
    };
    let records = comparison.processes.len()
        + comparison.destinations.len()
        + comparison.countries.len()
        + comparison.ports.len();
    let data = match request.format {
        ExportFormat::Json => serde_json::to_vec_pretty(&comparison)
            .map_err(|error| CommandError::recoverable("comparison_export_failed", error))?,
        ExportFormat::Csv => comparison_csv(&comparison).into_bytes(),
    };
    fs::write(&path, &data)
        .map_err(|error| CommandError::recoverable("comparison_export_failed", error))?;
    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        records,
        bytes_written: data.len() as u64,
    })
}

fn build_session_comparison(
    storage: &Storage,
    left_session_id: &str,
    right_session_id: &str,
) -> Result<SessionComparison, CommandError> {
    if left_session_id.trim().is_empty() || right_session_id.trim().is_empty() {
        return Err(CommandError::recoverable(
            "comparison_invalid",
            "two saved session identifiers are required",
        ));
    }
    let load = |session_id: &str| {
        storage.query_history(&HistoryQuery {
            from_ms: None,
            to_ms: None,
            session_id: Some(session_id.to_owned()),
            query: None,
            limit: 10_000,
            cursor: None,
        })
    };
    let left = load(left_session_id)
        .map_err(|error| CommandError::recoverable("comparison_unavailable", error))?;
    let right = load(right_session_id)
        .map_err(|error| CommandError::recoverable("comparison_unavailable", error))?;
    if left.is_empty() || right.is_empty() {
        return Err(CommandError::recoverable(
            "comparison_empty",
            "both sessions must contain saved connections",
        ));
    }
    let counts = |entries: &[HistoryEntry], category: &str| {
        let mut values = BTreeMap::<String, usize>::new();
        for entry in entries {
            let connection = &entry.connection;
            let key = match category {
                "process" => connection.process.name.clone(),
                "destination" => connection
                    .domain
                    .clone()
                    .or_else(|| connection.hostname.clone())
                    .unwrap_or_else(|| connection.key.remote.address.to_string()),
                "country" => connection
                    .geo
                    .as_ref()
                    .and_then(|geo| geo.country_name.clone())
                    .unwrap_or_else(|| "Unknown geography".into()),
                "port" => connection.key.remote.port.to_string(),
                _ => continue,
            };
            *values.entry(key).or_default() += connection.repeat_count.max(1) as usize;
        }
        values
    };
    Ok(SessionComparison {
        left_session_id: left_session_id.to_owned(),
        right_session_id: right_session_id.to_owned(),
        processes: comparison_rows(&counts(&left, "process"), &counts(&right, "process")),
        destinations: comparison_rows(
            &counts(&left, "destination"),
            &counts(&right, "destination"),
        ),
        countries: comparison_rows(&counts(&left, "country"), &counts(&right, "country")),
        ports: comparison_rows(&counts(&left, "port"), &counts(&right, "port")),
    })
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

fn comparison_csv(comparison: &SessionComparison) -> String {
    let mut csv = String::from("category,key,left_count,right_count,delta\n");
    for (category, rows) in [
        ("process", &comparison.processes),
        ("destination", &comparison.destinations),
        ("country", &comparison.countries),
        ("port", &comparison.ports),
    ] {
        for row in rows {
            let key = row.key.replace('"', "\"\"");
            csv.push_str(&format!(
                "{category},\"{key}\",{},{},{}\n",
                row.left_count, row.right_count, row.delta
            ));
        }
    }
    csv
}

#[tauri::command]
fn clear_session(state: State<'_, AppState>) -> Result<CaptureSummary, CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let request = SetCaptureRequest {
        mode: engine.capture_mode,
        selected_interface: engine.selected_interface.clone(),
    };
    let result = engine.set_capture(request);
    drop(engine);
    state.0.wake();
    result
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))
        .map(|engine| engine.settings.clone())
}

#[tauri::command]
fn update_settings(
    mut settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<AppSettings, CommandError> {
    if !(-90.0..=90.0).contains(&settings.home_latitude)
        || !(-180.0..=180.0).contains(&settings.home_longitude)
    {
        return Err(CommandError::recoverable(
            "invalid_settings",
            "home coordinates are outside valid latitude/longitude bounds",
        ));
    }
    if !(1..=300).contains(&settings.feed_group_window_seconds) {
        return Err(CommandError::recoverable(
            "invalid_settings",
            "feed grouping window must be between 1 and 300 seconds",
        ));
    }
    if !(75..=200).contains(&settings.text_scale_percent) {
        return Err(CommandError::recoverable(
            "invalid_settings",
            "text scale must be between 75 and 200 percent",
        ));
    }
    if settings.notification_quiet_start_minute > 1_439
        || settings.notification_quiet_end_minute > 1_439
    {
        return Err(CommandError::recoverable(
            "invalid_settings",
            "notification quiet hours must be valid local minutes",
        ));
    }
    if settings.operation_profile == sabicheiro_core::OperationProfile::Ephemeral {
        settings.ephemeral_capture = true;
    }
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let handshake_changed =
        engine.settings.handshake_metadata_enabled != settings.handshake_metadata_enabled;
    let autostart_changed = engine.settings.autostart_enabled != settings.autostart_enabled;
    let prior_autostart = engine.settings.autostart_enabled;
    if autostart_changed {
        configure_autostart(settings.autostart_enabled)
            .map_err(|error| CommandError::recoverable("autostart_unavailable", error))?;
    }
    if let Err(error) = engine.storage.save_settings(&settings) {
        if autostart_changed {
            let _ = configure_autostart(prior_autostart);
        }
        return Err(CommandError::recoverable("storage_unavailable", error));
    }
    if handshake_changed {
        engine
            .storage
            .record_handshake_mode_audit(&HandshakeModeAudit {
                occurred_at_ms: sabicheiro_core::now_ms(),
                enabled: settings.handshake_metadata_enabled,
                maximum_bytes: 16_384,
                maximum_packets: 8,
                duration_limit_ms: 5_000,
                detail: if settings.handshake_metadata_enabled {
                    "Explicit local opt-in; metadata only, bounded buffers, no payload persistence."
                        .into()
                } else {
                    "Handshake metadata mode disabled; ephemeral buffers are discarded.".into()
                },
            })
            .map_err(|error| CommandError::recoverable("storage_unavailable", error))?;
    }
    engine.settings = settings;
    let saved = engine.settings.clone();
    drop(engine);
    state.0.wake();
    Ok(saved)
}

#[tauri::command]
fn get_focus_rules(state: State<'_, AppState>) -> Result<FocusRules, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .list_focus_rules()
        .map_err(|error| CommandError::recoverable("focus_rules_unavailable", error))
}

#[tauri::command]
fn add_bookmark(
    kind: RuleKind,
    value: String,
    state: State<'_, AppState>,
) -> Result<Bookmark, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .add_bookmark(kind, &value, sabicheiro_core::now_ms())
        .map_err(|error| CommandError::recoverable("focus_rule_invalid", error))
}

#[tauri::command]
fn remove_bookmark(id: i64, state: State<'_, AppState>) -> Result<bool, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .remove_bookmark(id)
        .map_err(|error| CommandError::recoverable("focus_rules_unavailable", error))
}

#[tauri::command]
fn add_ignore_rule(
    kind: RuleKind,
    value: String,
    state: State<'_, AppState>,
) -> Result<IgnoreRule, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .add_ignore_rule(kind, &value, sabicheiro_core::now_ms())
        .map_err(|error| CommandError::recoverable("focus_rule_invalid", error))
}

#[tauri::command]
fn set_ignore_rule_enabled(
    id: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .set_ignore_rule_enabled(id, enabled)
        .map_err(|error| CommandError::recoverable("focus_rules_unavailable", error))
}

#[tauri::command]
fn remove_ignore_rule(id: i64, state: State<'_, AppState>) -> Result<bool, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .remove_ignore_rule(id)
        .map_err(|error| CommandError::recoverable("focus_rules_unavailable", error))
}

#[tauri::command]
fn get_connection_history_stats(
    remote_ip: String,
    state: State<'_, AppState>,
) -> Result<ConnectionHistoryStats, CommandError> {
    remote_ip
        .parse::<IpAddr>()
        .map_err(|_| CommandError::recoverable("history_query_invalid", "invalid remote IP"))?;
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .connection_history_stats(&remote_ip)
        .map_err(|error| CommandError::recoverable("history_unavailable", error))
}

#[tauri::command]
fn reconstruct_history(
    at_ms: i64,
    session_id: Option<String>,
    query: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Connection>, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let entries = engine
        .storage
        .query_history(&HistoryQuery {
            from_ms: None,
            to_ms: None,
            session_id,
            query,
            limit: 10_000,
            cursor: None,
        })
        .map_err(|error| CommandError::recoverable("history_unavailable", error))?;
    Ok(reconstruct_at(&entries, at_ms))
}

#[tauri::command]
fn start_session_reenrichment(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<ReenrichmentJobStatus, CommandError> {
    if session_id.trim().is_empty() {
        return Err(CommandError::recoverable(
            "reenrichment_invalid",
            "a saved session identifier is required",
        ));
    }
    let runtime = state.0.clone();
    let now_ms = sabicheiro_core::now_ms();
    let mut jobs = runtime
        .reenrichment_jobs
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job state is unavailable"))?;
    if jobs.values().any(|control| {
        control.status.lock().ok().is_some_and(|status| {
            status.session_id == session_id
                && matches!(
                    status.state,
                    ReenrichmentJobState::Queued | ReenrichmentJobState::Running
                )
        })
    }) {
        return Err(CommandError::recoverable(
            "reenrichment_active",
            "this session already has an active re-enrichment job",
        ));
    }
    if jobs.len() >= 32 {
        let mut completed = jobs
            .iter()
            .filter_map(|(id, control)| {
                control.status.lock().ok().and_then(|status| {
                    (!matches!(
                        status.state,
                        ReenrichmentJobState::Queued | ReenrichmentJobState::Running
                    ))
                    .then_some((id.clone(), status.finished_at_ms.unwrap_or(i64::MIN)))
                })
            })
            .collect::<Vec<_>>();
        completed.sort_by_key(|(_, finished)| *finished);
        for (id, _) in completed.into_iter().take(jobs.len().saturating_sub(31)) {
            jobs.remove(&id);
        }
    }
    let id = format!("reenrichment-{now_ms}-{}", jobs.len().saturating_add(1));
    let status = ReenrichmentJobStatus {
        id: id.clone(),
        session_id: session_id.clone(),
        state: ReenrichmentJobState::Queued,
        processed: 0,
        total: 0,
        started_at_ms: now_ms,
        finished_at_ms: None,
        message: None,
    };
    let control = ReenrichmentJobControl {
        cancel: Arc::new(AtomicBool::new(false)),
        status: Arc::new(Mutex::new(status.clone())),
    };
    jobs.insert(id, control.clone());
    drop(jobs);
    let _ = thread::Builder::new()
        .name("sabicheiro-reenrichment".into())
        .spawn(move || run_reenrichment_job(runtime, control, session_id));
    Ok(status)
}

fn set_reenrichment_state(
    control: &ReenrichmentJobControl,
    state: ReenrichmentJobState,
    processed: Option<usize>,
    total: Option<usize>,
    message: Option<String>,
) {
    if let Ok(mut status) = control.status.lock() {
        status.state = state;
        if let Some(processed) = processed {
            status.processed = processed;
        }
        if let Some(total) = total {
            status.total = total;
        }
        status.message = message;
        if matches!(
            state,
            ReenrichmentJobState::Completed
                | ReenrichmentJobState::Cancelled
                | ReenrichmentJobState::Failed
        ) {
            status.finished_at_ms = Some(sabicheiro_core::now_ms());
        }
    }
}

fn run_reenrichment_job(
    runtime: Arc<Runtime>,
    control: ReenrichmentJobControl,
    session_id: String,
) {
    set_reenrichment_state(&control, ReenrichmentJobState::Running, Some(0), None, None);
    let (entries, generations) = match runtime.engine.lock() {
        Ok(engine) => {
            let entries = engine.storage.query_history(&HistoryQuery {
                session_id: Some(session_id.clone()),
                limit: 100_000,
                ..HistoryQuery::default()
            });
            (entries, engine.geo.generations())
        }
        Err(_) => {
            set_reenrichment_state(
                &control,
                ReenrichmentJobState::Failed,
                None,
                None,
                Some("runtime state is unavailable".into()),
            );
            return;
        }
    };
    let Ok(entries) = entries else {
        set_reenrichment_state(
            &control,
            ReenrichmentJobState::Failed,
            None,
            None,
            Some(entries.expect_err("failed query").to_string()),
        );
        return;
    };
    let Some(generations) = generations else {
        set_reenrichment_state(
            &control,
            ReenrichmentJobState::Failed,
            None,
            Some(entries.len()),
            Some("GeoIP resolver state is unavailable".into()),
        );
        return;
    };
    set_reenrichment_state(
        &control,
        ReenrichmentJobState::Running,
        Some(0),
        Some(entries.len()),
        None,
    );
    let mut enrichments = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if control.cancel.load(Ordering::Relaxed) {
            set_reenrichment_state(
                &control,
                ReenrichmentJobState::Cancelled,
                Some(index),
                Some(entries.len()),
                Some("Cancelled before committing; observed data was not changed.".into()),
            );
            return;
        }
        let current = if entry.connection.key.remote.scope == AddressScope::Global {
            match runtime.engine.lock() {
                Ok(engine) => match engine.geo.lookup(entry.connection.key.remote.address) {
                    Ok(lookup)
                        if (lookup.city_generation, lookup.asn_generation) == generations =>
                    {
                        (lookup.location, lookup.asn)
                    }
                    Ok(_) => {
                        set_reenrichment_state(
                            &control,
                            ReenrichmentJobState::Failed,
                            Some(index),
                            Some(entries.len()),
                            Some("GeoIP datasets changed during the job; retry with the current generation.".into()),
                        );
                        return;
                    }
                    Err(error) => {
                        set_reenrichment_state(
                            &control,
                            ReenrichmentJobState::Failed,
                            Some(index),
                            Some(entries.len()),
                            Some(error),
                        );
                        return;
                    }
                },
                Err(_) => {
                    set_reenrichment_state(
                        &control,
                        ReenrichmentJobState::Failed,
                        Some(index),
                        Some(entries.len()),
                        Some("runtime state is unavailable".into()),
                    );
                    return;
                }
            }
        } else {
            (None, None)
        };
        enrichments.push(DerivedGeoEnrichment {
            connection_id: entry.connection.id.clone(),
            observed_geo: entry.connection.geo.clone(),
            observed_asn: entry.connection.asn.clone(),
            current_geo: current.0,
            current_asn: current.1,
        });
        if index % 128 == 127 || index + 1 == entries.len() {
            set_reenrichment_state(
                &control,
                ReenrichmentJobState::Running,
                Some(index + 1),
                Some(entries.len()),
                None,
            );
            thread::yield_now();
        }
    }
    if control.cancel.load(Ordering::Relaxed) {
        set_reenrichment_state(
            &control,
            ReenrichmentJobState::Cancelled,
            Some(entries.len()),
            Some(entries.len()),
            Some("Cancelled before committing; observed data was not changed.".into()),
        );
        return;
    }
    let stored = runtime
        .engine
        .lock()
        .map_err(|_| "runtime state is unavailable".to_owned())
        .and_then(|mut engine| {
            if engine.geo.generations() != Some(generations) {
                return Err(
                    "GeoIP datasets changed before commit; retry with the current generation."
                        .into(),
                );
            }
            engine
                .storage
                .replace_geo_views(
                    &session_id,
                    "current",
                    &enrichments,
                    generations.0,
                    generations.1,
                    sabicheiro_core::now_ms(),
                )
                .map_err(|error| error.to_string())
        });
    match stored {
        Ok(()) => set_reenrichment_state(
            &control,
            ReenrichmentJobState::Completed,
            Some(entries.len()),
            Some(entries.len()),
            Some("Derived view saved; original observations remain unchanged.".into()),
        ),
        Err(error) => set_reenrichment_state(
            &control,
            ReenrichmentJobState::Failed,
            Some(entries.len()),
            Some(entries.len()),
            Some(error),
        ),
    }
}

#[tauri::command]
fn get_reenrichment_job(
    id: String,
    state: State<'_, AppState>,
) -> Result<ReenrichmentJobStatus, CommandError> {
    let jobs = state
        .0
        .reenrichment_jobs
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job state is unavailable"))?;
    let control = jobs.get(&id).ok_or_else(|| {
        CommandError::recoverable("reenrichment_not_found", "re-enrichment job was not found")
    })?;
    let status = control
        .status
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job status is unavailable"))?
        .clone();
    Ok(status)
}

#[tauri::command]
fn cancel_reenrichment_job(id: String, state: State<'_, AppState>) -> Result<bool, CommandError> {
    let jobs = state
        .0
        .reenrichment_jobs
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job state is unavailable"))?;
    let Some(control) = jobs.get(&id) else {
        return Ok(false);
    };
    control.cancel.store(true, Ordering::Relaxed);
    Ok(true)
}

#[tauri::command]
fn start_executable_inspection(
    process_instance: String,
    maximum_megabytes: Option<u64>,
    state: State<'_, AppState>,
) -> Result<ExecutableInspectionStatus, CommandError> {
    if process_instance.trim().is_empty() || process_instance.len() > 256 {
        return Err(CommandError::recoverable(
            "process_identity_invalid",
            "a valid observed process instance is required",
        ));
    }
    let runtime = state.0.clone();
    let process = {
        let engine = runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
        engine
            .aggregator
            .connections()
            .find(|connection| {
                connection.process.instance_id().as_deref() == Some(&process_instance)
            })
            .map(|connection| connection.process.clone())
            .or_else(|| {
                engine
                    .storage
                    .query_history(&HistoryQuery {
                        from_ms: None,
                        to_ms: None,
                        session_id: None,
                        query: None,
                        limit: 100_000,
                        cursor: None,
                    })
                    .ok()?
                    .into_iter()
                    .find(|entry| {
                        entry.connection.process.instance_id().as_deref() == Some(&process_instance)
                    })
                    .map(|entry| entry.connection.process)
            })
    }
    .ok_or_else(|| {
        CommandError::recoverable(
            "process_identity_not_found",
            "the observed process instance is no longer present in retained local evidence",
        )
    })?;
    let path = process.executable_path.clone().ok_or_else(|| {
        CommandError::recoverable(
            "process_identity_unavailable",
            "the operating system did not expose an executable path for this process",
        )
    })?;
    let started_at_ms = sabicheiro_core::now_ms();
    let id = format!(
        "executable-{}-{started_at_ms}",
        process.pid.unwrap_or_default()
    );
    let status = ExecutableInspectionStatus {
        id: id.clone(),
        process_instance: process_instance.clone(),
        state: ExecutableInspectionState::Queued,
        identity: None,
        started_at_ms,
        finished_at_ms: None,
        message: Some("Local identity inspection is queued.".into()),
    };
    let control = ExecutableInspectionJobControl {
        cancel: Arc::new(AtomicBool::new(false)),
        status: Arc::new(Mutex::new(status.clone())),
    };
    runtime
        .executable_jobs
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job state is unavailable"))?
        .insert(id, control.clone());
    let maximum_bytes = maximum_megabytes
        .unwrap_or(256)
        .clamp(1, 512)
        .saturating_mul(1024 * 1024);
    let _ = thread::Builder::new()
        .name("sabicheiro-executable-identity".into())
        .spawn(move || {
            if let Ok(mut current) = control.status.lock() {
                current.state = ExecutableInspectionState::Running;
                current.message = Some(format!(
                    "Hashing locally with a {} MiB I/O budget.",
                    maximum_bytes / 1024 / 1024
                ));
            }
            let result = inspect_executable(
                Path::new(&path),
                Some(process_instance),
                maximum_bytes,
                &control.cancel,
            );
            if let Ok(mut current) = control.status.lock() {
                current.finished_at_ms = Some(sabicheiro_core::now_ms());
                if control.cancel.load(Ordering::Relaxed) {
                    current.state = ExecutableInspectionState::Cancelled;
                    current.message = Some("Local executable inspection was cancelled.".into());
                    return;
                }
                match result {
                    Ok(identity) => {
                        current.state = ExecutableInspectionState::Completed;
                        current.message = identity.limitation.clone().or_else(|| {
                            Some("Local executable identity completed; no reputation lookup was used.".into())
                        });
                        current.identity = Some(identity);
                    }
                    Err(error) => {
                        current.state = ExecutableInspectionState::Failed;
                        current.message = Some(error.to_string());
                    }
                }
            }
        })
        .map_err(|error| CommandError::recoverable("process_identity_unavailable", error))?;
    Ok(status)
}

#[tauri::command]
fn get_executable_inspection(
    id: String,
    state: State<'_, AppState>,
) -> Result<ExecutableInspectionStatus, CommandError> {
    let jobs = state
        .0
        .executable_jobs
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job state is unavailable"))?;
    jobs.get(&id)
        .ok_or_else(|| {
            CommandError::recoverable(
                "process_identity_not_found",
                "local executable inspection was not found",
            )
        })?
        .status
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job status is unavailable"))
        .map(|status| status.clone())
}

#[tauri::command]
fn cancel_executable_inspection(
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    let jobs = state
        .0
        .executable_jobs
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "job state is unavailable"))?;
    let Some(control) = jobs.get(&id) else {
        return Ok(false);
    };
    control.cancel.store(true, Ordering::Relaxed);
    Ok(true)
}

#[tauri::command]
fn get_session_geo_view(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<DerivedGeoEnrichment>, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .load_geo_views(&session_id, "current")
        .map_err(|error| CommandError::recoverable("reenrichment_unavailable", error))
}

#[tauri::command]
fn set_retention(
    retention: RetentionPolicy,
    state: State<'_, AppState>,
) -> Result<usize, CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let removed = engine
        .storage
        .cleanup(retention, sabicheiro_core::now_ms())
        .map_err(|error| CommandError::recoverable("storage_unavailable", error))?;
    engine.settings.retention = retention;
    let _ = engine.storage.save_settings(&engine.settings);
    Ok(removed)
}

#[tauri::command]
fn compact_storage(state: State<'_, AppState>) -> Result<StorageHealth, CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    if engine.capture_mode != CaptureMode::Off {
        return Err(CommandError::recoverable(
            "storage_busy",
            "Stop capture before compacting local history.",
        ));
    }
    let health = engine
        .storage
        .compact()
        .map_err(|error| CommandError::recoverable("storage_maintenance_failed", error))?;
    engine.diagnostics.storage_quick_check_ok = health.quick_check_ok;
    engine.diagnostics.storage_last_check_ms = Some(health.checked_at_ms);
    engine.diagnostics.storage_wal_pages = health.wal_log_pages;
    engine.diagnostics.database_size_bytes = health.database_size_bytes;
    Ok(health)
}

fn clear_log_files(logging: Option<&LocalLogging>) -> Result<usize, CommandError> {
    let Some(logging) = logging else {
        return Ok(0);
    };
    let _guard = logging
        .write_lock
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "log state is unavailable"))?;
    let entries = fs::read_dir(&logging.directory)
        .map_err(|error| CommandError::recoverable("local_data_remove_failed", error))?;
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_log = path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("sabicheiro.log"));
        if is_log {
            fs::remove_file(&path)
                .map_err(|error| CommandError::recoverable("local_data_remove_failed", error))?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[tauri::command]
fn clear_local_data(
    category: LocalDataCategory,
    state: State<'_, AppState>,
) -> Result<LocalDataRemovalResult, CommandError> {
    let runtime = state.0.clone();
    let requires_capture_restart = matches!(
        category,
        LocalDataCategory::Sessions | LocalDataCategory::All
    );
    if requires_capture_restart {
        let engine = runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
        if engine.capture_mode != CaptureMode::Off {
            return Err(CommandError::recoverable(
                "capture_must_stop",
                "Stop capture before deleting saved sessions or all local data.",
            ));
        }
    }
    let mut removed_items = 0;
    if matches!(category, LocalDataCategory::Logs | LocalDataCategory::All) {
        removed_items += clear_log_files(runtime.logging.as_ref())?;
    }
    let mut engine = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    match category {
        LocalDataCategory::Sessions => {
            removed_items += engine
                .storage
                .clear_sessions()
                .map_err(|error| CommandError::recoverable("local_data_remove_failed", error))?;
            engine.aggregator = ConnectionAggregator::default();
            engine.session = None;
        }
        LocalDataCategory::GeoIp => {
            for kind in [GeoDatabaseKind::City, GeoDatabaseKind::Asn] {
                removed_items += usize::from(
                    remove_mmdb(runtime.data_directory.join("geoip"), kind).map_err(|error| {
                        CommandError::recoverable("local_data_remove_failed", error)
                    })?,
                );
            }
            engine
                .geo
                .replace(LocalGeoResolver::empty())
                .map_err(|error| CommandError::recoverable("local_data_remove_failed", error))?;
        }
        LocalDataCategory::DnsCache => {
            removed_items += engine.dns_observations.len();
            engine.dns_observations.clear();
            engine.dns = DnsService::start();
        }
        LocalDataCategory::Logs => {}
        LocalDataCategory::All => {
            removed_items += engine
                .storage
                .clear_all_user_data()
                .map_err(|error| CommandError::recoverable("local_data_remove_failed", error))?;
            for kind in [GeoDatabaseKind::City, GeoDatabaseKind::Asn] {
                removed_items += usize::from(
                    remove_mmdb(runtime.data_directory.join("geoip"), kind).map_err(|error| {
                        CommandError::recoverable("local_data_remove_failed", error)
                    })?,
                );
            }
            removed_items += engine.dns_observations.len();
            engine.dns_observations.clear();
            engine.dns = DnsService::start();
            engine
                .geo
                .replace(LocalGeoResolver::empty())
                .map_err(|error| CommandError::recoverable("local_data_remove_failed", error))?;
            engine.settings = AppSettings::default();
            engine.aggregator = ConnectionAggregator::default();
            engine.session = None;
        }
    }
    engine.refresh_storage_health();
    Ok(LocalDataRemovalResult {
        category,
        removed_items,
        requires_capture_restart,
    })
}

#[tauri::command]
fn get_geo_database_status(state: State<'_, AppState>) -> Result<GeoDatabaseStatus, CommandError> {
    let engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine
        .geo
        .status()
        .map_err(|error| CommandError::recoverable("geo_status_unavailable", error))
}

#[tauri::command]
async fn update_geo_database(
    state: State<'_, AppState>,
) -> Result<GeoDatabaseStatus, CommandError> {
    let runtime = state.0.clone();
    let grant = {
        let engine = runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
        let broker = NetworkActionBroker::new(NetworkActionPolicy {
            offline_enforced: engine.settings.offline_enforced,
            geoip_updates_allowed: true,
            reverse_dns_allowed: false,
        });
        match broker.authorize(
            NetworkActionIntent::GeoIpDatasetUpdate,
            "geoip.update_button",
        ) {
            Ok(grant) => {
                let _ = engine.storage.record_network_action(
                    sabicheiro_core::now_ms(),
                    grant.intent(),
                    grant.destination_class(),
                    Some(grant.consent_id()),
                    true,
                    0,
                    0,
                    "Explicit fixed-origin DB-IP Lite dataset update started.",
                );
                grant
            }
            Err(error) => {
                let _ = engine.storage.record_network_action(
                    sabicheiro_core::now_ms(),
                    NetworkActionIntent::GeoIpDatasetUpdate,
                    NetworkActionIntent::GeoIpDatasetUpdate.destination_class(),
                    Some("geoip.update_button"),
                    false,
                    0,
                    0,
                    &error.to_string(),
                );
                return Err(CommandError::recoverable("network_action_denied", error));
            }
        }
    };
    let directory = runtime.data_directory.join("geoip");
    let updated = tauri::async_runtime::spawn_blocking(move || update_dbip_lite(directory, &grant))
        .await
        .map_err(|error| CommandError::recoverable("geo_update_failed", error))?
        .map_err(|error| CommandError::recoverable("geo_update_failed", error))?;
    let engine = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    let bytes_received = fs::metadata(&updated.city_path)
        .map(|metadata| metadata.len())
        .unwrap_or_default()
        .saturating_add(
            fs::metadata(&updated.asn_path)
                .map(|metadata| metadata.len())
                .unwrap_or_default(),
        );
    let _ = engine.storage.record_network_action(
        sabicheiro_core::now_ms(),
        NetworkActionIntent::GeoIpDatasetUpdate,
        NetworkActionIntent::GeoIpDatasetUpdate.destination_class(),
        Some("geoip.update_button"),
        true,
        0,
        bytes_received,
        "Fixed-origin DB-IP Lite update completed; no observed address was transmitted.",
    );
    engine
        .geo
        .reload(&updated.city_path, &updated.asn_path)
        .map_err(|error| CommandError::recoverable("geo_update_failed", error))?;
    engine
        .geo
        .status()
        .map_err(|error| CommandError::recoverable("geo_update_failed", error))
}

#[tauri::command]
async fn import_geo_database(
    source_path: String,
    kind: GeoDatabaseKind,
    state: State<'_, AppState>,
) -> Result<GeoDatabaseStatus, CommandError> {
    let runtime = state.0.clone();
    let directory = runtime.data_directory.join("geoip");
    let import_directory = directory.clone();
    tauri::async_runtime::spawn_blocking(move || import_mmdb(source_path, import_directory, kind))
        .await
        .map_err(|error| CommandError::recoverable("geo_import_failed", error))?
        .map_err(|error| CommandError::recoverable("geo_import_failed", error))?;
    let city_path = directory.join("dbip-city-lite.mmdb");
    let asn_path = directory.join("dbip-asn-lite.mmdb");
    let resolver = LocalGeoResolver::open(Some(&city_path), Some(&asn_path))
        .map_err(|error| CommandError::recoverable("geo_import_failed", error))?;
    let engine = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine
        .geo
        .replace(resolver)
        .map_err(|error| CommandError::recoverable("geo_import_failed", error))?;
    engine
        .geo
        .status()
        .map_err(|error| CommandError::recoverable("geo_import_failed", error))
}

#[tauri::command]
async fn rollback_geo_database(
    kind: GeoDatabaseKind,
    state: State<'_, AppState>,
) -> Result<GeoDatabaseStatus, CommandError> {
    let runtime = state.0.clone();
    let directory = runtime.data_directory.join("geoip");
    let rollback_directory = directory.clone();
    let restored =
        tauri::async_runtime::spawn_blocking(move || rollback_mmdb(rollback_directory, kind))
            .await
            .map_err(|error| CommandError::recoverable("geo_rollback_failed", error))?
            .map_err(|error| CommandError::recoverable("geo_rollback_failed", error))?;
    let engine = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine
        .geo
        .reload_kind(kind, &restored.path)
        .map_err(|error| CommandError::recoverable("geo_rollback_failed", error))?;
    engine
        .geo
        .status()
        .map_err(|error| CommandError::recoverable("geo_rollback_failed", error))
}

#[tauri::command]
async fn remove_geo_database(
    kind: GeoDatabaseKind,
    state: State<'_, AppState>,
) -> Result<GeoDatabaseStatus, CommandError> {
    let runtime = state.0.clone();
    let directory = runtime.data_directory.join("geoip");
    let removal_directory = directory.clone();
    tauri::async_runtime::spawn_blocking(move || remove_mmdb(removal_directory, kind))
        .await
        .map_err(|error| CommandError::recoverable("geo_remove_failed", error))?
        .map_err(|error| CommandError::recoverable("geo_remove_failed", error))?;

    let city_path = directory.join("dbip-city-lite.mmdb");
    let asn_path = directory.join("dbip-asn-lite.mmdb");
    let resolver = LocalGeoResolver::open(Some(&city_path), Some(&asn_path))
        .map_err(|error| CommandError::recoverable("geo_remove_failed", error))?;
    let engine = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine
        .geo
        .replace(resolver)
        .map_err(|error| CommandError::recoverable("geo_remove_failed", error))?;
    engine
        .geo
        .status()
        .map_err(|error| CommandError::recoverable("geo_remove_failed", error))
}

#[tauri::command]
fn export_view(
    mut request: ExportRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, CommandError> {
    let runtime = state.0.clone();
    request.path = prepare_output_path(&runtime.data_directory, &request.path)?
        .to_string_lossy()
        .into_owned();
    runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .storage
        .export(&request)
        .map_err(|error| CommandError::recoverable("export_failed", error))
}

#[tauri::command]
fn export_sabi(
    request: SabiExportRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, CommandError> {
    let runtime = state.0.clone();
    let path = prepare_output_path(&runtime.data_directory, &request.path)?;
    require_extension(&path, "sabi", "investigation bundle")?;
    if path.exists() {
        return Err(CommandError::recoverable(
            "output_path_exists",
            "investigation bundle already exists; choose a new filename",
        ));
    }
    let (investigation, connections, timeline, dataset_sha256) = {
        let engine = runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
        let investigation = engine
            .storage
            .list_investigations()
            .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?
            .into_iter()
            .find(|investigation| investigation.id == request.investigation_id)
            .ok_or_else(|| {
                CommandError::recoverable(
                    "investigation_not_found",
                    "the saved investigation no longer exists",
                )
            })?;
        let connections = engine
            .storage
            .query_investigation_redacted(&investigation.query, &request.redaction)
            .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?;
        let from_ms = investigation.query.from_ms.or_else(|| {
            connections
                .iter()
                .map(|entry| entry.connection.first_seen_ms)
                .min()
        });
        let to_ms = investigation.query.to_ms.or_else(|| {
            connections
                .iter()
                .map(|entry| entry.connection.last_seen_ms)
                .max()
        });
        let timeline = match from_ms.zip(to_ms) {
            Some((from_ms, to_ms)) if from_ms <= to_ms => engine
                .storage
                .query_timeline(&TimelineQuery {
                    from_ms,
                    to_ms,
                    session_id: investigation.query.session_id.clone(),
                    process_key: None,
                    resolution: TimelineResolution::OneSecond,
                })
                .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?,
            _ => Vec::new(),
        };
        let mut dataset_sha256 = engine
            .geo
            .status()
            .ok()
            .into_iter()
            .flat_map(|status| [status.city.sha256, status.asn.sha256])
            .flatten()
            .collect::<Vec<_>>();
        dataset_sha256.sort();
        dataset_sha256.dedup();
        (investigation, connections, timeline, dataset_sha256)
    };

    let mut files = vec![
        (
            "investigation.json".to_owned(),
            serde_json::to_vec_pretty(&investigation)
                .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?,
        ),
        (
            "connections.json".to_owned(),
            serde_json::to_vec_pretty(&connections)
                .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?,
        ),
        (
            "timeline.json".to_owned(),
            serde_json::to_vec_pretty(&timeline)
                .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?,
        ),
    ];
    let entry_sha256 = files
        .iter()
        .map(|(name, data)| (name.clone(), sha256_hex(data)))
        .collect::<BTreeMap<_, _>>();
    let manifest = SabiManifest {
        schema_version: 1,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created_at_ms: sabicheiro_core::now_ms(),
        investigation_id: investigation.id.clone(),
        query: investigation.query.clone(),
        from_ms: investigation.query.from_ms,
        to_ms: investigation.query.to_ms,
        redaction: request.redaction,
        dataset_sha256,
        entry_sha256,
    };
    files.push((
        "manifest.json".to_owned(),
        serde_json::to_vec_pretty(&manifest)
            .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?,
    ));
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let bytes_written = write_zip_archive(&path, &files)
        .map_err(|error| CommandError::recoverable("sabi_export_failed", error))?;
    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        records: connections.len(),
        bytes_written,
    })
}

#[tauri::command]
fn import_sabi(
    path: String,
    state: State<'_, AppState>,
) -> Result<ImportedSabiBundle, CommandError> {
    let runtime = state.0.clone();
    let path = normalized_output_path(&runtime.data_directory, &path)?;
    require_extension(&path, "sabi", "investigation bundle")?;
    let files = read_stored_zip_archive(&path)
        .map_err(|error| CommandError::recoverable("sabi_import_failed", error))?;
    let manifest_bytes = files.get("manifest.json").ok_or_else(|| {
        CommandError::recoverable("sabi_import_failed", "bundle manifest is missing")
    })?;
    let manifest: SabiManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| CommandError::recoverable("sabi_import_failed", error))?;
    if manifest.schema_version != 1 {
        return Err(CommandError::recoverable(
            "sabi_version_unsupported",
            format!("unsupported .sabi schema {}", manifest.schema_version),
        ));
    }
    let actual_names = files
        .keys()
        .filter(|name| name.as_str() != "manifest.json")
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_names = manifest
        .entry_sha256
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_names != expected_names {
        return Err(CommandError::recoverable(
            "sabi_hash_mismatch",
            "bundle entries do not match the signed manifest set",
        ));
    }
    for (name, expected) in &manifest.entry_sha256 {
        let actual = files
            .get(name)
            .map(|data| sha256_hex(data))
            .unwrap_or_default();
        if !constant_time_ascii_eq(expected.as_bytes(), actual.as_bytes()) {
            return Err(CommandError::recoverable(
                "sabi_hash_mismatch",
                format!("bundle entry {name} failed SHA-256 validation"),
            ));
        }
    }
    let investigation: SavedInvestigation = decode_sabi_entry(&files, "investigation.json")?;
    let connections: Vec<HistoryEntry> = decode_sabi_entry(&files, "connections.json")?;
    let timeline: Vec<TrafficSample> = decode_sabi_entry(&files, "timeline.json")?;
    if investigation.id != manifest.investigation_id || investigation.query != manifest.query {
        return Err(CommandError::recoverable(
            "sabi_manifest_mismatch",
            "investigation identity or query does not match the manifest",
        ));
    }
    Ok(ImportedSabiBundle {
        source_path: path.to_string_lossy().into_owned(),
        read_only: true,
        manifest,
        investigation,
        connections,
        timeline,
    })
}

fn decode_sabi_entry<T: serde::de::DeserializeOwned>(
    files: &BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<T, CommandError> {
    let data = files.get(name).ok_or_else(|| {
        CommandError::recoverable(
            "sabi_import_failed",
            format!("bundle entry {name} is missing"),
        )
    })?;
    serde_json::from_slice(data)
        .map_err(|error| CommandError::recoverable("sabi_import_failed", error))
}

fn require_extension(path: &Path, extension: &str, label: &str) -> Result<(), CommandError> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case(extension))
    {
        return Err(CommandError::recoverable(
            "output_path_invalid",
            format!("{label} must use the .{extension} extension"),
        ));
    }
    Ok(())
}

#[tauri::command]
fn export_support_bundle(
    request: SupportBundleRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, CommandError> {
    let runtime = state.0.clone();
    let path = prepare_output_path(&runtime.data_directory, &request.path)?;
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("zip"))
    {
        return Err(CommandError::recoverable(
            "output_path_invalid",
            "support bundle output must use the .zip extension",
        ));
    }
    if path.exists() {
        return Err(CommandError::recoverable(
            "output_path_exists",
            "support bundle already exists; choose a new filename",
        ));
    }
    let (settings, diagnostics, capture) = {
        let engine = runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
        (
            engine.settings.clone(),
            engine.diagnostics.clone(),
            engine.capture_summary(),
        )
    };
    let mut files = vec![
        (
            "manifest.json".to_owned(),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": 1,
                "generated_at_ms": sabicheiro_core::now_ms(),
                "app_version": env!("CARGO_PKG_VERSION"),
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "privacy": "No payloads, packet recordings, raw IP history, database, home coordinates, adapter identifiers or secrets are included."
            }))
            .map_err(|error| CommandError::recoverable("support_bundle_failed", error))?,
        ),
        (
            "diagnostics.json".to_owned(),
            serde_json::to_vec_pretty(&serde_json::json!({
                "cpu_percent": diagnostics.cpu_percent,
                "memory_bytes": diagnostics.memory_bytes,
                "render_fps": diagnostics.render_fps,
                "capture_events_per_second": diagnostics.capture_events_per_second,
                "database_size_bytes": diagnostics.database_size_bytes,
                "capture_queue_depth": diagnostics.capture_queue_depth,
                "capture_queue_capacity": diagnostics.capture_queue_capacity,
                "dropped_capture_events": diagnostics.dropped_capture_events,
                "dropped_ui_events": diagnostics.dropped_ui_events,
                "backend": diagnostics.backend,
                "capture_event_source": diagnostics.capture_event_source,
                "flow_counters_available": diagnostics.flow_counters_available,
                "packet_events_observed": diagnostics.packet_events_observed,
                "dns_correlations_observed": diagnostics.dns_correlations_observed,
                "has_last_error": diagnostics.last_error.is_some(),
                "capture_mode": capture.mode,
                "capture_status": capture.status,
                "vpn_active": capture.vpn_active
            }))
            .map_err(|error| CommandError::recoverable("support_bundle_failed", error))?,
        ),
    ];
    if request.include_settings {
        files.push((
            "settings.redacted.json".to_owned(),
            serde_json::to_vec_pretty(&serde_json::json!({
                "retention": settings.retention,
                "reduce_motion": settings.reduce_motion,
                "auto_focus": settings.auto_focus,
                "show_local_traffic": settings.show_local_traffic,
                "show_listening_sockets": settings.show_listening_sockets,
                "reverse_dns_enabled": settings.reverse_dns_enabled,
                "ui_language": settings.ui_language,
                "feed_group_window_seconds": settings.feed_group_window_seconds,
                "active_workspace": settings.active_workspace,
                "notification_rule_count": settings.notification_rules.len(),
                "selected_interface": "redacted",
                "home_coordinates": "redacted"
            }))
            .map_err(|error| CommandError::recoverable("support_bundle_failed", error))?,
        ));
    }
    if request.include_logs
        && let Some(logging) = &runtime.logging
        && let Ok(entries) = fs::read_dir(&logging.directory)
    {
        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        paths.sort();
        let skip = paths.len().saturating_sub(7);
        for path in paths.into_iter().skip(skip) {
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if let Ok(mut data) = fs::read(&path) {
                data.truncate(1_048_576);
                files.push((format!("logs/{name}"), data));
            }
        }
    }
    let bytes_written = write_zip_archive(&path, &files)
        .map_err(|error| CommandError::recoverable("support_bundle_failed", error))?;
    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        records: files.len(),
        bytes_written,
    })
}

fn write_zip_archive(path: &Path, files: &[(String, Vec<u8>)]) -> std::io::Result<u64> {
    struct CentralEntry {
        name: Vec<u8>,
        crc32: u32,
        size: u32,
        offset: u32,
    }
    let mut output = fs::File::create(path)?;
    let mut central = Vec::with_capacity(files.len());
    for (name, data) in files {
        let name = name.as_bytes();
        let size = u32::try_from(data.len())
            .map_err(|_| std::io::Error::other("support bundle entry exceeds 4 GiB"))?;
        let offset = u32::try_from(output.stream_position()?)
            .map_err(|_| std::io::Error::other("support bundle exceeds ZIP32 limits"))?;
        let crc32 = crc32(data);
        write_u32(&mut output, 0x0403_4b50)?;
        write_u16(&mut output, 20)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u32(&mut output, crc32)?;
        write_u32(&mut output, size)?;
        write_u32(&mut output, size)?;
        write_u16(
            &mut output,
            u16::try_from(name.len()).map_err(|_| std::io::Error::other("ZIP name too long"))?,
        )?;
        write_u16(&mut output, 0)?;
        output.write_all(name)?;
        output.write_all(data)?;
        central.push(CentralEntry {
            name: name.to_vec(),
            crc32,
            size,
            offset,
        });
    }
    let central_offset = u32::try_from(output.stream_position()?)
        .map_err(|_| std::io::Error::other("support bundle exceeds ZIP32 limits"))?;
    for entry in &central {
        write_u32(&mut output, 0x0201_4b50)?;
        write_u16(&mut output, 20)?;
        write_u16(&mut output, 20)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u32(&mut output, entry.crc32)?;
        write_u32(&mut output, entry.size)?;
        write_u32(&mut output, entry.size)?;
        write_u16(&mut output, entry.name.len() as u16)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u32(&mut output, 0)?;
        write_u32(&mut output, entry.offset)?;
        output.write_all(&entry.name)?;
    }
    let central_end = u32::try_from(output.stream_position()?)
        .map_err(|_| std::io::Error::other("support bundle exceeds ZIP32 limits"))?;
    let entry_count = u16::try_from(central.len())
        .map_err(|_| std::io::Error::other("support bundle has too many entries"))?;
    write_u32(&mut output, 0x0605_4b50)?;
    write_u16(&mut output, 0)?;
    write_u16(&mut output, 0)?;
    write_u16(&mut output, entry_count)?;
    write_u16(&mut output, entry_count)?;
    write_u32(&mut output, central_end - central_offset)?;
    write_u32(&mut output, central_offset)?;
    write_u16(&mut output, 0)?;
    output.flush()?;
    Ok(output.metadata()?.len())
}

fn read_stored_zip_archive(path: &Path) -> std::io::Result<BTreeMap<String, Vec<u8>>> {
    const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
    const MAX_ENTRY_BYTES: usize = 64 * 1024 * 1024;
    const MAX_ENTRIES: usize = 128;
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(std::io::Error::other(
            "investigation bundle is not a file or exceeds 256 MiB",
        ));
    }
    let bytes = fs::read(path)?;
    let mut offset = 0_usize;
    let mut total_uncompressed = 0_usize;
    let mut files = BTreeMap::new();
    while offset.checked_add(4).is_some_and(|end| end <= bytes.len()) {
        let signature = read_zip_u32(&bytes, offset)?;
        if signature == 0x0201_4b50 || signature == 0x0605_4b50 {
            break;
        }
        if signature != 0x0403_4b50 || offset + 30 > bytes.len() {
            return Err(std::io::Error::other("invalid ZIP local entry header"));
        }
        let flags = read_zip_u16(&bytes, offset + 6)?;
        let compression = read_zip_u16(&bytes, offset + 8)?;
        let expected_crc = read_zip_u32(&bytes, offset + 14)?;
        let compressed_size = read_zip_u32(&bytes, offset + 18)? as usize;
        let uncompressed_size = read_zip_u32(&bytes, offset + 22)? as usize;
        let name_length = usize::from(read_zip_u16(&bytes, offset + 26)?);
        let extra_length = usize::from(read_zip_u16(&bytes, offset + 28)?);
        if flags != 0
            || compression != 0
            || compressed_size != uncompressed_size
            || uncompressed_size > MAX_ENTRY_BYTES
            || name_length == 0
            || name_length > 1_024
        {
            return Err(std::io::Error::other(
                "bundle uses unsupported ZIP features or exceeds entry limits",
            ));
        }
        let name_start = offset + 30;
        let name_end = name_start
            .checked_add(name_length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| std::io::Error::other("truncated ZIP entry name"))?;
        let name = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|_| std::io::Error::other("ZIP entry name is not UTF-8"))?;
        if name.starts_with('/')
            || name.starts_with('\\')
            || name.contains("..")
            || name.contains(':')
            || name.bytes().any(|byte| byte == 0 || byte == b'\\')
        {
            return Err(std::io::Error::other("unsafe ZIP entry name"));
        }
        let data_start = name_end
            .checked_add(extra_length)
            .filter(|start| *start <= bytes.len())
            .ok_or_else(|| std::io::Error::other("truncated ZIP extra data"))?;
        let data_end = data_start
            .checked_add(compressed_size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| std::io::Error::other("truncated ZIP entry"))?;
        let data = bytes[data_start..data_end].to_vec();
        if crc32(&data) != expected_crc {
            return Err(std::io::Error::other("ZIP entry CRC mismatch"));
        }
        total_uncompressed = total_uncompressed
            .checked_add(data.len())
            .filter(|total| *total <= MAX_ARCHIVE_BYTES as usize)
            .ok_or_else(|| std::io::Error::other("ZIP uncompressed data exceeds 256 MiB"))?;
        if files.insert(name.to_owned(), data).is_some() || files.len() > MAX_ENTRIES {
            return Err(std::io::Error::other("duplicate or excessive ZIP entries"));
        }
        offset = data_end;
    }
    if files.is_empty() || offset >= bytes.len() || read_zip_u32(&bytes, offset)? != 0x0201_4b50 {
        return Err(std::io::Error::other("ZIP central directory is missing"));
    }
    Ok(files)
}

fn read_zip_u16(bytes: &[u8], offset: usize) -> std::io::Result<u16> {
    let value = bytes
        .get(offset..offset.saturating_add(2))
        .ok_or_else(|| std::io::Error::other("truncated ZIP integer"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_zip_u32(bytes: &[u8], offset: usize) -> std::io::Result<u32> {
    let value = bytes
        .get(offset..offset.saturating_add(4))
        .ok_or_else(|| std::io::Error::other("truncated ZIP integer"))?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn constant_time_ascii_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

fn write_u16(output: &mut impl Write, value: u16) -> std::io::Result<()> {
    output.write_all(&value.to_le_bytes())
}

fn write_u32(output: &mut impl Write, value: u32) -> std::io::Result<()> {
    output.write_all(&value.to_le_bytes())
}

fn crc32(data: &[u8]) -> u32 {
    let mut value = 0xffff_ffff_u32;
    for byte in data {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(value & 1)));
        }
    }
    !value
}

#[tauri::command]
fn start_recording(
    mut request: RecordingRequest,
    state: State<'_, AppState>,
) -> Result<RecordingState, CommandError> {
    let runtime = state.0.clone();
    request.output_path = prepare_output_path(&runtime.data_directory, &request.output_path)?
        .to_string_lossy()
        .into_owned();
    let result = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .start_recording(request);
    runtime.wake();
    result
}

#[tauri::command]
fn stop_recording(state: State<'_, AppState>) -> Result<RecordingState, CommandError> {
    let result = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .stop_recording();
    state.0.wake();
    result
}

#[tauri::command]
fn get_recording_state(state: State<'_, AppState>) -> Result<RecordingState, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))
        .map(|engine| engine.recording.state())
}

fn prepare_output_path(data_directory: &Path, value: &str) -> Result<PathBuf, CommandError> {
    let path = normalized_output_path(data_directory, value)?;
    if !path.is_absolute() || Path::new(value.trim()).is_relative() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| CommandError::recoverable("output_path_invalid", error))?;
        }
    }
    Ok(path)
}

fn normalized_output_path(data_directory: &Path, value: &str) -> Result<PathBuf, CommandError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.as_bytes().contains(&0) {
        return Err(CommandError::recoverable(
            "output_path_invalid",
            "output filename is empty or contains NUL",
        ));
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        return Ok(path);
    }
    if path.components().count() != 1 || path.file_name().is_none() {
        return Err(CommandError::recoverable(
            "output_path_invalid",
            "relative output must be a filename without directories",
        ));
    }
    Ok(data_directory.join("exports").join(path))
}

#[tauri::command]
fn report_ui_diagnostics(
    render_fps: f32,
    dropped_ui_events: u64,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine.diagnostics.render_fps = render_fps.clamp(0.0, 240.0);
    engine.diagnostics.dropped_ui_events = dropped_ui_events;
    if let Some(value) =
        std::env::var_os("SABICHEIRO_QA_READY_FILE").filter(|value| !value.is_empty())
    {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            let payload = format!("{{\"ready_at_ms\":{}}}\n", sabicheiro_core::now_ms());
            let _ = fs::write(path, payload);
        }
    }
    Ok(())
}

fn setup_logging(data_directory: &Path) -> Option<LocalLogging> {
    let logs = data_directory.join("logs");
    if fs::create_dir_all(&logs).is_err() {
        return None;
    }
    cleanup_old_logs(&logs, Duration::from_secs(7 * 24 * 60 * 60));
    Some(LocalLogging {
        directory: logs,
        write_lock: Mutex::new(()),
    })
}

fn cleanup_old_logs(directory: &Path, retention: Duration) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("sabicheiro.log"))
        {
            continue;
        }
        let expired = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > retention);
        if expired {
            let _ = fs::remove_file(path);
        }
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn tray_status(engine: &Engine) -> (String, String, String, String) {
    let mode = match engine.capture_mode {
        CaptureMode::Off => "off",
        CaptureMode::Demo => "demo",
        CaptureMode::Live => "live",
    };
    let state = match &engine.capture_status {
        CaptureStatus::Stopped => "stopped",
        CaptureStatus::Starting => "starting",
        CaptureStatus::Capturing => "capturing",
        CaptureStatus::Error(_) => "error",
    };
    let persistence = if engine.settings.ephemeral_capture {
        "memory only"
    } else {
        "local SQLite"
    };
    let profile = match engine.settings.operation_profile {
        sabicheiro_core::OperationProfile::Visible => "visible",
        sabicheiro_core::OperationProfile::BackgroundLowResource => "background low-resource",
        sabicheiro_core::OperationProfile::InvestigationHighFidelity => {
            "investigation high-fidelity"
        }
        sabicheiro_core::OperationProfile::Ephemeral => "ephemeral",
    };
    let degraded = engine
        .diagnostics
        .subsystems
        .iter()
        .filter(|subsystem| {
            matches!(
                subsystem.status,
                SubsystemStatus::Degraded | SubsystemStatus::Unavailable
            )
        })
        .count();
    let metadata = if engine.settings.handshake_metadata_enabled {
        " · handshake metadata ON"
    } else {
        ""
    };
    (
        format!("Status: {mode} / {state} / {persistence}"),
        format!("Health: {degraded} degraded · profile {profile}"),
        if engine.view_paused {
            "Resume view (capture continues)".into()
        } else {
            "Pause view (capture continues)".into()
        },
        format!("Sabicheiro — {mode} / {state} / {persistence}{metadata}"),
    )
}

fn setup_tray(app: &mut tauri::App, runtime: Arc<Runtime>) -> tauri::Result<()> {
    let status = MenuItem::with_id(
        app,
        "tray_status",
        "Status: off / stopped / local SQLite",
        false,
        None::<&str>,
    )?;
    let health = MenuItem::with_id(app, "tray_health", "Health: starting", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "tray_open", "Open Sabicheiro", true, None::<&str>)?;
    let pause = MenuItem::with_id(
        app,
        "tray_pause",
        "Pause view (capture continues)",
        true,
        None::<&str>,
    )?;
    let stop = MenuItem::with_id(app, "tray_stop", "Stop capture", true, None::<&str>)?;
    let quit = MenuItem::with_id(
        app,
        "tray_quit",
        "Quit and stop capture",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&status, &health, &open, &pause, &stop, &quit])?;
    let mut builder = TrayIconBuilder::with_id("sabicheiro-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Sabicheiro — capture starting")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray_open" => show_main_window(app),
            "tray_pause" => {
                let runtime = app.state::<AppState>().0.clone();
                if let Ok(mut engine) = runtime.engine.lock() {
                    engine.view_paused = !engine.view_paused;
                    engine.paused_at_revision = engine.aggregator.revision();
                    engine.paused_view_events = 0;
                }
            }
            "tray_stop" => {
                let runtime = app.state::<AppState>().0.clone();
                if let Ok(mut engine) = runtime.engine.lock() {
                    let _ = engine.set_capture(SetCaptureRequest {
                        mode: CaptureMode::Off,
                        selected_interface: None,
                    });
                }
            }
            "tray_quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let icon = builder.build(app)?;
    let updater_status = status.clone();
    let updater_health = health.clone();
    let updater_pause = pause.clone();
    let updater_icon = icon.clone();
    let updater_runtime = runtime.clone();
    let _ = thread::Builder::new()
        .name("sabicheiro-tray-status".into())
        .spawn(move || {
            while !updater_runtime.shutdown.load(Ordering::Relaxed) {
                if let Ok(engine) = updater_runtime.engine.lock() {
                    let (status_text, health_text, pause_text, tooltip) = tray_status(&engine);
                    let _ = updater_status.set_text(status_text);
                    let _ = updater_health.set_text(health_text);
                    let _ = updater_pause.set_text(pause_text);
                    let _ = updater_icon.set_tooltip(Some(tooltip));
                }
                thread::sleep(DIAGNOSTICS_INTERVAL);
            }
        });
    app.manage(TrayUi {
        _icon: icon,
        _status: status,
        _health: health,
        _pause: pause,
    });
    Ok(())
}

#[cfg(target_os = "linux")]
fn quote_desktop_exec(path: &Path) -> Result<String, String> {
    let value = path
        .to_str()
        .ok_or_else(|| "the executable path is not valid UTF-8".to_owned())?;
    if value.contains(['\r', '\n', '\0']) {
        return Err("the executable path contains an invalid control character".into());
    }
    Ok(format!(
        "\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

#[cfg(target_os = "windows")]
fn configure_autostart(enabled: bool) -> Result<(), String> {
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not resolve the Sabicheiro executable: {error}"))?;
    let executable = executable
        .to_str()
        .filter(|value| !value.contains(['\r', '\n', '\0']))
        .ok_or_else(|| "the Sabicheiro executable path is invalid".to_owned())?;
    if enabled {
        let value = format!("\"{executable}\"");
        let output = std::process::Command::new("reg.exe")
            .args([
                "ADD",
                RUN_KEY,
                "/v",
                "Sabicheiro",
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .output()
            .map_err(|error| format!("could not invoke the Windows startup registry: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "Windows rejected the per-user startup registration: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    } else {
        let _ = std::process::Command::new("reg.exe")
            .args(["DELETE", RUN_KEY, "/v", "Sabicheiro", "/f"])
            .output();
        let query = std::process::Command::new("reg.exe")
            .args(["QUERY", RUN_KEY, "/v", "Sabicheiro"])
            .output()
            .map_err(|error| format!("could not verify Windows startup state: {error}"))?;
        if query.status.success() {
            return Err("the per-user Windows startup entry could not be removed".into());
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn configure_autostart(enabled: bool) -> Result<(), String> {
    let configuration = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| "neither XDG_CONFIG_HOME nor the user home is available".to_owned())?;
    let directory = configuration.join("autostart");
    let destination = directory.join("io.sabicheiro.desktop.desktop");
    if !enabled {
        if destination.exists() {
            fs::remove_file(&destination)
                .map_err(|error| format!("could not remove {}: {error}", destination.display()))?;
        }
        return Ok(());
    }
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not resolve the Sabicheiro executable: {error}"))?;
    let executable = quote_desktop_exec(&executable)?;
    let contents = format!(
        "[Desktop Entry]\nType=Application\nName=Sabicheiro\nComment=Private local network observability\nExec={executable}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    );
    let temporary = directory.join(format!(
        ".io.sabicheiro.desktop.desktop.tmp-{}",
        std::process::id()
    ));
    fs::write(&temporary, contents)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &destination).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("could not install {}: {error}", destination.display())
    })?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn configure_autostart(_enabled: bool) -> Result<(), String> {
    Err("autostart is supported only on the Windows and Linux release targets".into())
}

pub fn run() {
    let application = tauri::Builder::default()
        .setup(|app| {
            let default_data_directory = app.path().app_data_dir()?;
            let data_directory = match std::env::var_os("SABICHEIRO_QA_DATA_DIR") {
                Some(value) if !value.is_empty() => {
                    let candidate = PathBuf::from(value);
                    if !candidate.is_absolute() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "SABICHEIRO_QA_DATA_DIR must be an absolute path",
                        )
                        .into());
                    }
                    candidate
                }
                _ => default_data_directory,
            };
            fs::create_dir_all(&data_directory)?;
            let logging = setup_logging(&data_directory);
            let runtime = Arc::new(Runtime::new(data_directory, logging));
            runtime.spawn();
            if let Some(logging) = &runtime.logging {
                logging.info("Sabicheiro desktop runtime started");
            }
            app.manage(AppState(runtime.clone()));
            if setup_tray(app, runtime.clone()).is_err() {
                if let Some(logging) = &runtime.logging {
                    logging.info("Sabicheiro tray was unavailable; the main window remains usable");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard_update,
            set_capture_state,
            list_interfaces,
            run_capture_self_test,
            set_view_paused,
            query_history,
            save_investigation,
            list_investigations,
            delete_investigation,
            save_local_rule,
            list_local_rules,
            delete_local_rule,
            preview_local_rule,
            list_notifications,
            update_notification_state,
            list_network_profiles,
            rename_network_profile,
            save_today_card_state,
            list_today_card_states,
            capture_listener_snapshot,
            list_listener_snapshots,
            get_connection_context,
            get_identity_timeline,
            query_traffic_timeline,
            compare_sessions,
            export_session_comparison,
            clear_session,
            get_settings,
            update_settings,
            set_retention,
            get_focus_rules,
            add_bookmark,
            remove_bookmark,
            add_ignore_rule,
            set_ignore_rule_enabled,
            remove_ignore_rule,
            get_connection_history_stats,
            reconstruct_history,
            start_session_reenrichment,
            get_reenrichment_job,
            cancel_reenrichment_job,
            start_executable_inspection,
            get_executable_inspection,
            cancel_executable_inspection,
            get_session_geo_view,
            compact_storage,
            clear_local_data,
            get_geo_database_status,
            update_geo_database,
            import_geo_database,
            rollback_geo_database,
            remove_geo_database,
            export_view,
            export_sabi,
            import_sabi,
            export_support_bundle,
            start_recording,
            stop_recording,
            get_recording_state,
            report_ui_diagnostics,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Sabicheiro");

    application.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            app_handle.state::<AppState>().0.shutdown();
        }
    });
}

#[cfg(test)]
mod command_tests {
    use super::*;
    #[cfg(target_os = "windows")]
    use std::net::{Ipv4Addr, TcpListener, TcpStream};

    fn isolated_runtime(label: &str) -> (Arc<Runtime>, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "sabicheiro-{label}-{}-{}",
            std::process::id(),
            sabicheiro_core::now_ms()
        ));
        fs::create_dir_all(&directory).expect("create isolated runtime directory");
        (Arc::new(Runtime::new(directory.clone(), None)), directory)
    }

    #[test]
    fn relative_outputs_are_scoped_to_the_app_export_directory() {
        let base = Path::new("sabicheiro-test-data");
        let resolved = normalized_output_path(base, "connections.csv").expect("valid output");
        assert_eq!(resolved, base.join("exports").join("connections.csv"));
    }

    #[test]
    fn relative_output_cannot_traverse_outside_the_export_directory() {
        assert!(
            normalized_output_path(Path::new("sabicheiro-test-data"), "../outside.csv").is_err()
        );
    }

    #[test]
    fn interface_deltas_are_converted_to_real_per_second_rates() {
        let rates = transfer_rates_from_deltas(4_096, 2_048, 120, Duration::from_secs(2));
        assert_eq!(rates.down_bytes_per_second, 2_048.0);
        assert_eq!(rates.up_bytes_per_second, 1_024.0);
        assert_eq!(rates.packets_per_second, 60.0);
    }

    #[test]
    fn support_bundle_writer_emits_a_consistent_zip32_archive() {
        let directory = std::env::temp_dir().join(format!(
            "sabicheiro-support-zip-test-{}-{}",
            std::process::id(),
            sabicheiro_core::now_ms()
        ));
        fs::create_dir_all(&directory).expect("create ZIP fixture directory");
        let path = directory.join("support.zip");
        let files = vec![
            ("manifest.json".to_owned(), br#"{"redacted":true}"#.to_vec()),
            ("diagnostics.json".to_owned(), b"{}".to_vec()),
        ];
        let reported_size = write_zip_archive(&path, &files).expect("write support ZIP");
        let archive = fs::read(&path).expect("read support ZIP");

        assert_eq!(reported_size, archive.len() as u64);
        assert_eq!(&archive[0..4], &0x0403_4b50_u32.to_le_bytes());
        assert_eq!(
            &archive[archive.len() - 22..archive.len() - 18],
            &0x0605_4b50_u32.to_le_bytes()
        );
        let eocd = archive.len() - 22;
        let entry_count = u16::from_le_bytes([archive[eocd + 10], archive[eocd + 11]]);
        let central_size = u32::from_le_bytes([
            archive[eocd + 12],
            archive[eocd + 13],
            archive[eocd + 14],
            archive[eocd + 15],
        ]) as usize;
        let central_offset = u32::from_le_bytes([
            archive[eocd + 16],
            archive[eocd + 17],
            archive[eocd + 18],
            archive[eocd + 19],
        ]) as usize;
        assert_eq!(entry_count, files.len() as u16);
        assert_eq!(central_offset + central_size, eocd);
        assert_eq!(
            &archive[central_offset..central_offset + 4],
            &0x0201_4b50_u32.to_le_bytes()
        );
        assert!(
            archive
                .windows("manifest.json".len())
                .any(|value| value == b"manifest.json")
        );
        assert!(
            archive
                .windows("diagnostics.json".len())
                .any(|value| value == b"diagnostics.json")
        );

        fs::remove_dir_all(directory).expect("remove ZIP fixture directory");
    }

    #[test]
    fn session_comparison_reports_additions_and_removals() {
        let mut storage = Storage::in_memory().expect("storage");
        let mut left = sabicheiro_core::Connection::placeholder(1_000);
        left.id = "left".into();
        left.process.name = "browser".into();
        left.key.remote.address = "203.0.113.1".parse().expect("left address");
        left.key.remote.port = 443;
        storage
            .start_session("left-session", 1_000)
            .expect("left session");
        storage
            .save_connections("left-session", &[left])
            .expect("save left");

        let mut right = sabicheiro_core::Connection::placeholder(2_000);
        right.id = "right".into();
        right.process.name = "updater".into();
        right.key.remote.address = "203.0.113.2".parse().expect("right address");
        right.key.remote.port = 8443;
        storage
            .start_session("right-session", 2_000)
            .expect("right session");
        storage
            .save_connections("right-session", &[right])
            .expect("save right");

        let comparison = build_session_comparison(&storage, "left-session", "right-session")
            .expect("compare sessions");
        assert!(
            comparison
                .processes
                .iter()
                .any(|row| row.key == "browser" && row.delta == -1)
        );
        assert!(
            comparison
                .processes
                .iter()
                .any(|row| row.key == "updater" && row.delta == 1)
        );
        assert!(
            comparison
                .ports
                .iter()
                .any(|row| row.key == "443" && row.delta == -1)
        );
        assert!(
            comparison
                .ports
                .iter()
                .any(|row| row.key == "8443" && row.delta == 1)
        );
    }

    #[test]
    fn dashboard_updates_delta_only_for_the_expected_sequence() {
        let (runtime, directory) = isolated_runtime("ipc-update-test");
        let mut engine = runtime.engine.lock().expect("runtime state");

        let first = engine.update(None);
        assert!(first.resync_required);
        assert!(first.snapshot.is_some());

        let second = engine.update(Some(first.sequence));
        assert!(!second.resync_required);
        assert!(second.snapshot.is_none());
        assert_eq!(second.base_revision, first.revision);
        assert!(second.processes.is_none());
        assert!(second.destinations.is_none());
        assert!(second.stats.is_none());
        assert!(second.diagnostics.is_none());
        assert!(serde_json::to_vec(&second).expect("serialize delta").len() < 512);
        assert_eq!(engine.ipc_metrics.requests, 2);
        assert_eq!(engine.ipc_metrics.empty_deltas, 1);
        assert_eq!(engine.ipc_metrics.overlaps, 0);
        assert_eq!(engine.diagnostics.sequence_gaps, 0);

        let stale = engine.update(Some(first.sequence));
        assert!(stale.resync_required);
        assert!(stale.snapshot.is_some());
        assert_eq!(engine.diagnostics.sequence_gaps, 1);
        assert_eq!(engine.diagnostics.snapshot_resyncs, 1);

        drop(engine);
        drop(runtime);
        fs::remove_dir_all(directory).expect("remove IPC fixture directory");
    }

    #[test]
    fn idle_engine_waits_for_deadlines_instead_of_fixed_ticks() {
        let (runtime, directory) = isolated_runtime("idle-deadline-test");
        let mut engine = runtime.engine.lock().expect("runtime state");
        assert_eq!(engine.capture_mode, CaptureMode::Off);
        assert!(engine.next_wakeup() >= Duration::from_secs(10));
        engine
            .set_capture(SetCaptureRequest {
                mode: CaptureMode::Demo,
                selected_interface: None,
            })
            .expect("start demo");
        assert!(engine.next_wakeup() <= Duration::from_secs(1));
        engine
            .set_capture(SetCaptureRequest {
                mode: CaptureMode::Off,
                selected_interface: None,
            })
            .expect("stop demo");

        drop(engine);
        drop(runtime);
        fs::remove_dir_all(directory).expect("remove idle fixture directory");
    }

    #[test]
    fn stable_persistence_deduplicates_samples_and_forced_stop_is_durable() {
        let (runtime, directory) = isolated_runtime("incremental-persistence-test");
        let database_path = directory.join("sabicheiro.sqlite3");
        let started_at_ms = (sabicheiro_core::now_ms() / 1_000) * 1_000;
        let session_id;
        {
            let mut engine = runtime.engine.lock().expect("runtime state");
            engine.capture_mode = CaptureMode::Live;
            engine
                .start_session(started_at_ms)
                .expect("start persisted session");
            session_id = engine.session.as_ref().expect("session").id.clone();
            let events = engine.demo.next_batch(started_at_ms);
            engine.process_events(events, false);
            engine.last_traffic_sample = Instant::now() - Duration::from_secs(11);
            engine.last_traffic_write = Instant::now() - Duration::from_secs(61);
            engine.persist(started_at_ms, false);

            engine.last_traffic_sample = Instant::now() - Duration::from_secs(11);
            engine.persist(started_at_ms + 10_000, false);
            let stable = engine
                .storage
                .query_timeline(&TimelineQuery {
                    from_ms: started_at_ms,
                    to_ms: started_at_ms + 10_000,
                    session_id: Some(session_id.clone()),
                    process_key: None,
                    resolution: TimelineResolution::OneSecond,
                })
                .expect("stable samples");
            assert_eq!(stable.len(), 1, "identical state should not add a sample");

            engine.persist(started_at_ms + 20_000, true);
            let forced = engine
                .storage
                .query_timeline(&TimelineQuery {
                    from_ms: started_at_ms,
                    to_ms: started_at_ms + 20_000,
                    session_id: Some(session_id.clone()),
                    process_key: None,
                    resolution: TimelineResolution::OneSecond,
                })
                .expect("forced samples");
            assert_eq!(forced.len(), 2);
            engine.stop_current_session(started_at_ms + 30_000);
        }
        drop(runtime);

        let reopened = Storage::open(&database_path).expect("reopen persisted database");
        assert_eq!(
            reopened
                .query_history(&HistoryQuery {
                    session_id: Some(session_id.clone()),
                    ..HistoryQuery::default()
                })
                .expect("durable history")
                .len(),
            1
        );
        let session = reopened
            .list_sessions(10)
            .expect("durable session")
            .into_iter()
            .find(|session| session.id == session_id)
            .expect("finished session");
        assert_eq!(session.ended_at_ms, Some(started_at_ms + 30_000));
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove persistence fixture directory");
    }

    #[test]
    fn ten_minutes_of_demo_remain_in_memory_and_leave_history_empty() {
        let (runtime, directory) = isolated_runtime("demo-isolation-test");
        let mut engine = runtime.engine.lock().expect("runtime state");
        engine
            .set_capture(SetCaptureRequest {
                mode: CaptureMode::Demo,
                selected_interface: None,
            })
            .expect("start isolated demo");
        let started_at_ms = sabicheiro_core::now_ms();
        for second in 0..600_i64 {
            let now_ms = started_at_ms.saturating_add(second.saturating_mul(1_000));
            let events = engine.demo.next_batch(now_ms);
            assert!(events.len() <= 1);
            engine.process_events(events, true);
            engine.persist(now_ms, true);
            assert!(engine.aggregator.connection_count() <= 4);
        }
        assert!(
            engine
                .session
                .as_ref()
                .is_some_and(|session| session.ephemeral)
        );
        assert!(
            engine
                .storage
                .list_sessions(100)
                .expect("stored sessions")
                .is_empty()
        );
        assert!(
            engine
                .storage
                .query_history(&HistoryQuery::default())
                .expect("stored history")
                .is_empty()
        );
        assert_eq!(
            engine
                .storage
                .evidence_stats()
                .expect("evidence stats")
                .observations,
            0
        );
        assert!(
            engine
                .storage
                .list_investigations()
                .expect("investigations")
                .is_empty()
        );
        assert!(engine.storage.list_local_rules().expect("rules").is_empty());
        assert!(
            engine
                .storage
                .list_notifications(100)
                .expect("notifications")
                .is_empty()
        );
        engine
            .set_capture(SetCaptureRequest {
                mode: CaptureMode::Off,
                selected_interface: None,
            })
            .expect("stop isolated demo");

        drop(engine);
        drop(runtime);
        fs::remove_dir_all(directory).expect("remove demo fixture directory");
    }

    #[test]
    fn asn_profile_and_identity_timeline_trace_retained_connections() {
        let mut connection = Connection::placeholder(1_000);
        connection.id = "traceable-connection".into();
        connection.process.name = "traceable-process".into();
        connection.key.process_instance = Some("traceable-process-instance".into());
        connection.domain = Some("local-fixture.invalid".into());
        connection.hostname = connection.domain.clone();
        connection.asn = Some(sabicheiro_core::AsnInfo {
            number: 64_512,
            organization: Some("Fixture Network".into()),
            ..sabicheiro_core::AsnInfo::default()
        });
        connection.geo = Some(sabicheiro_core::GeoLocation {
            country_code: Some("ZZ".into()),
            country_name: Some("Fixture Country".into()),
            source: "synthetic-local-mmdb".into(),
            ..sabicheiro_core::GeoLocation::default()
        });
        connection.active = false;
        let entry = HistoryEntry {
            session_id: "traceable-session".into(),
            connection,
        };

        let profile = build_asn_profile(64_512, std::slice::from_ref(&entry));
        assert_eq!(profile.organization.as_deref(), Some("Fixture Network"));
        assert_eq!(profile.observed_ips.len(), 1);
        assert_eq!(profile.process_names, vec!["traceable-process"]);
        assert_eq!(profile.session_ids, vec!["traceable-session"]);
        assert_eq!(profile.connection_ids, vec!["traceable-connection"]);

        let timeline = build_identity_timeline(&entry);
        assert!(
            timeline
                .iter()
                .any(|event| event.kind == IdentityEventKind::ProcessObserved)
        );
        assert!(
            timeline
                .iter()
                .any(|event| event.kind == IdentityEventKind::DnsResolved)
        );
        assert!(
            timeline
                .iter()
                .any(|event| event.kind == IdentityEventKind::GeoEnriched)
        );
        assert!(
            timeline
                .iter()
                .any(|event| event.kind == IdentityEventKind::ConnectionClosed)
        );
        assert!(
            timeline
                .iter()
                .all(|event| event.connection_id == "traceable-connection")
        );
    }

    #[test]
    fn cancelled_reenrichment_never_replaces_the_derived_view() {
        let (runtime, directory) = isolated_runtime("reenrichment-cancel-test");
        {
            let mut engine = runtime.engine.lock().expect("runtime state");
            engine
                .storage
                .start_session("cancel-session", 1_000)
                .expect("start session");
            let mut connection = Connection::placeholder(1_000);
            connection.id = "cancel-connection".into();
            engine
                .storage
                .save_connections("cancel-session", &[connection])
                .expect("save session evidence");
        }
        let status = ReenrichmentJobStatus {
            id: "cancel-job".into(),
            session_id: "cancel-session".into(),
            state: ReenrichmentJobState::Queued,
            processed: 0,
            total: 0,
            started_at_ms: sabicheiro_core::now_ms(),
            finished_at_ms: None,
            message: None,
        };
        let control = ReenrichmentJobControl {
            cancel: Arc::new(AtomicBool::new(true)),
            status: Arc::new(Mutex::new(status)),
        };

        run_reenrichment_job(runtime.clone(), control.clone(), "cancel-session".into());

        let finished = control.status.lock().expect("job status").clone();
        assert_eq!(finished.state, ReenrichmentJobState::Cancelled);
        let derived = runtime
            .engine
            .lock()
            .expect("runtime state")
            .storage
            .load_geo_views("cancel-session", "current")
            .expect("load derived views");
        assert!(derived.is_empty());

        drop(runtime);
        fs::remove_dir_all(directory).expect("remove re-enrichment fixture directory");
    }

    #[test]
    fn failed_live_setup_does_not_leave_capture_without_a_provider() {
        let directory = std::env::temp_dir().join(format!(
            "sabicheiro-capture-state-test-{}-{}",
            std::process::id(),
            sabicheiro_core::now_ms()
        ));
        fs::create_dir_all(&directory).expect("create isolated runtime directory");
        let runtime = Runtime::new(directory.clone(), None);
        let mut engine = runtime.engine.lock().expect("runtime state");

        let result = engine.set_capture(SetCaptureRequest {
            mode: CaptureMode::Live,
            selected_interface: Some("definitely-not-a-real-interface".into()),
        });

        assert!(result.is_err());
        assert_eq!(engine.capture_mode, CaptureMode::Off);
        assert!(matches!(engine.capture_status, CaptureStatus::Stopped));
        assert!(engine.provider.is_none());

        drop(engine);
        drop(runtime);
        fs::remove_dir_all(directory).expect("remove isolated runtime directory");
    }

    #[test]
    fn sabi_zip_parser_is_bounded_and_detects_altered_entry_bytes() {
        let directory = std::env::temp_dir().join(format!(
            "sabicheiro-sabi-parser-test-{}-{}",
            std::process::id(),
            sabicheiro_core::now_ms()
        ));
        fs::create_dir_all(&directory).expect("create bundle fixture directory");
        let path = directory.join("fixture.sabi");
        let files = vec![
            ("connections.json".into(), br#"[{"id":"fixture"}]"#.to_vec()),
            ("manifest.json".into(), br#"{"schema_version":1}"#.to_vec()),
        ];
        write_zip_archive(&path, &files).expect("write bundle fixture");
        let decoded = read_stored_zip_archive(&path).expect("read bundle fixture");
        assert_eq!(decoded.get("connections.json"), Some(&files[0].1));

        let mut altered = fs::read(&path).expect("read archive bytes");
        let needle = b"fixture";
        let index = altered
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("fixture payload offset");
        altered[index] ^= 1;
        fs::write(&path, altered).expect("write altered archive");
        assert!(read_stored_zip_archive(&path).is_err());

        for length in 0..1_024 {
            let fuzz_path = directory.join("fuzz.sabi");
            let bytes = vec![length as u8; length];
            fs::write(&fuzz_path, bytes).expect("write parser input");
            let result = std::panic::catch_unwind(|| read_stored_zip_archive(&fuzz_path));
            assert!(result.is_ok(), "bundle parser panicked at length {length}");
        }
        fs::remove_dir_all(directory).expect("remove bundle fixture directory");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn live_engine_ingests_an_established_remote_peer() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind loopback listener");
        let listener_address = listener.local_addr().expect("listener address");
        let client = TcpStream::connect(listener_address).expect("connect loopback client");
        let client_address = client.local_addr().expect("client address");
        let (server, _) = listener.accept().expect("accept loopback client");

        let directory = std::env::temp_dir().join(format!(
            "sabicheiro-live-pipeline-test-{}-{}",
            std::process::id(),
            sabicheiro_core::now_ms()
        ));
        fs::create_dir_all(&directory).expect("create isolated runtime directory");
        let runtime = Runtime::new(directory.clone(), None);
        let mut engine = runtime.engine.lock().expect("runtime state");
        engine
            .set_capture(SetCaptureRequest {
                mode: CaptureMode::Live,
                selected_interface: None,
            })
            .expect("start native capture");
        engine.tick();

        assert!(
            engine.aggregator.connections().any(|connection| {
                connection.key.pid == Some(std::process::id())
                    && connection.key.local.address.is_loopback()
                    && connection.key.local.port == client_address.port()
                    && connection.key.remote.port == listener_address.port()
                    && connection.state == sabicheiro_core::ConnectionState::Established
            }),
            "the live engine did not ingest the established peer from the native provider"
        );

        engine
            .set_capture(SetCaptureRequest {
                mode: CaptureMode::Off,
                selected_interface: None,
            })
            .expect("stop native capture");
        drop(engine);
        drop(runtime);
        drop(server);
        drop(client);
        fs::remove_dir_all(directory).expect("remove isolated runtime directory");
    }

    #[test]
    fn isolated_local_logging_writes_and_flushes_lifecycle_events() {
        let directory = std::env::temp_dir().join(format!(
            "sabicheiro-logging-test-{}-{}",
            std::process::id(),
            sabicheiro_core::now_ms()
        ));
        let logging = setup_logging(&directory).expect("local logger");
        logging.info("Sabicheiro logging test event");
        drop(logging);

        let logs = directory.join("logs");
        let path = fs::read_dir(&logs)
            .expect("log directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("sabicheiro.log"))
            })
            .expect("daily log");
        let contents = fs::read_to_string(path).expect("read local log");
        assert!(contents.contains("Sabicheiro logging test event"));
        fs::remove_dir_all(directory).expect("remove isolated log fixture");
    }
}
