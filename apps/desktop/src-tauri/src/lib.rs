#[cfg(target_os = "windows")]
use sabicheiro_capture::WindowsEventCapture;
use sabicheiro_capture::{
    NetworkCaptureProvider, RecordingController, SnapshotDiffer, system_provider,
};
use sabicheiro_core::{
    AppSettings, Bookmark, CaptureMode, CaptureSelfTest, CaptureStatus, CaptureSummary,
    ComparisonRow, Connection, ConnectionAggregator, ConnectionHistoryStats, ConnectionKey,
    CounterSource, DashboardSnapshot, Diagnostics, ExportFormat, ExportRequest, ExportResult,
    FocusRules, GeoDatabaseStatus, HistoryEntry, HistoryQuery, IgnoreRule, NetworkEventKind,
    NormalizedNetworkEvent, RecordingRequest, RecordingState, RecordingStatus, RecordingSummary,
    RetentionPolicy, RuleKind, SelfTestCheck, SelfTestLevel, SessionComparison, SessionSummary,
    SupportBundleRequest, TimelineQuery, TrafficCounters, TrafficSample, TransferRates,
    reconstruct_at,
};
use sabicheiro_geo::{
    AddressScope, DnsResolver, GeoDatabaseKind, LocalGeoResolver, import_mmdb, update_dbip_lite,
};
use sabicheiro_storage::Storage;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{Seek, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Networks, ProcessesToUpdate, System};
use tauri::{Manager, State};

const TICK_INTERVAL: Duration = Duration::from_millis(200);
const LIVE_POLL_INTERVAL: Duration = Duration::from_millis(750);
const PERSIST_INTERVAL: Duration = Duration::from_secs(1);
const DIAGNOSTICS_INTERVAL: Duration = Duration::from_secs(2);
const INTERFACE_REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const RETENTION_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const DNS_QUEUE_CAPACITY: usize = 1_024;

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

struct Runtime {
    engine: Mutex<Engine>,
    shutdown: AtomicBool,
    data_directory: PathBuf,
    logging: Option<LocalLogging>,
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
}

#[derive(Debug)]
struct DnsResult {
    key: ConnectionKey,
    hostname: Option<String>,
}

struct DnsService {
    sender: SyncSender<DnsJob>,
    receiver: Receiver<DnsResult>,
    pending: Arc<AtomicUsize>,
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
                    let hostname = resolver.resolve(job.address, sabicheiro_core::now_ms());
                    worker_pending.fetch_sub(1, Ordering::Relaxed);
                    if result_sender
                        .send(DnsResult {
                            key: job.key,
                            hostname,
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
        }
    }

    fn enqueue(&self, job: DnsJob) -> bool {
        match self.sender.try_send(job) {
            Ok(()) => {
                self.pending.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false,
        }
    }

    fn pending(&self) -> usize {
        self.pending.load(Ordering::Relaxed)
    }
}

struct Engine {
    aggregator: ConnectionAggregator,
    capture_mode: CaptureMode,
    capture_status: CaptureStatus,
    selected_interface: Option<String>,
    selected_interface_addresses: HashSet<IpAddr>,
    vpn_active: bool,
    provider: Option<Box<dyn NetworkCaptureProvider>>,
    #[cfg(target_os = "windows")]
    event_capture: Option<WindowsEventCapture>,
    differ: SnapshotDiffer,
    demo: sabicheiro_core::DemoGenerator,
    storage: Storage,
    geo: LocalGeoResolver,
    dns: DnsService,
    dns_observations: HashMap<IpAddr, (String, i64)>,
    settings: AppSettings,
    session: Option<SessionSummary>,
    recording: RecordingController,
    active_recording_id: Option<i64>,
    diagnostics: Diagnostics,
    process_metrics: System,
    network_metrics: Networks,
    network_rates: TransferRates,
    last_capture_poll: Instant,
    last_persist: Instant,
    last_diagnostics: Instant,
    last_interface_refresh: Instant,
    last_retention_cleanup: Instant,
    last_event_rate: Instant,
    events_since_rate: u64,
    paused_view_events: u64,
    view_paused: bool,
    paused_at_revision: u64,
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
        let retention_error = storage
            .cleanup(settings.retention, sabicheiro_core::now_ms())
            .err()
            .map(|error| error.to_string());
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
            .or(retention_error);
        let mut diagnostics = Diagnostics {
            backend: "stopped".into(),
            capture_event_source: "none".into(),
            capture_queue_capacity: DNS_QUEUE_CAPACITY,
            last_error: initial_error,
            ..Diagnostics::default()
        };
        diagnostics.database_size_bytes = storage.database_size_bytes();
        Self {
            engine: Mutex::new(Engine {
                aggregator: ConnectionAggregator::from_baselines(baselines),
                capture_mode: CaptureMode::Off,
                capture_status: CaptureStatus::Stopped,
                selected_interface: settings.selected_interface.clone(),
                selected_interface_addresses: HashSet::new(),
                vpn_active: false,
                provider: None,
                #[cfg(target_os = "windows")]
                event_capture: None,
                differ: SnapshotDiffer::default(),
                demo: sabicheiro_core::DemoGenerator::new(settings.demo_seed),
                storage,
                geo,
                dns: DnsService::start(),
                dns_observations: HashMap::new(),
                settings,
                session: None,
                recording: RecordingController::default(),
                active_recording_id: None,
                diagnostics,
                process_metrics: System::new(),
                network_metrics: Networks::new_with_refreshed_list(),
                network_rates: TransferRates::default(),
                last_capture_poll: Instant::now() - LIVE_POLL_INTERVAL,
                last_persist: Instant::now(),
                last_diagnostics: Instant::now() - DIAGNOSTICS_INTERVAL,
                last_interface_refresh: Instant::now() - INTERFACE_REFRESH_INTERVAL,
                last_retention_cleanup: Instant::now(),
                last_event_rate: Instant::now(),
                events_since_rate: 0,
                paused_view_events: 0,
                view_paused: false,
                paused_at_revision: 0,
            }),
            shutdown: AtomicBool::new(false),
            data_directory,
            logging,
        }
    }

    fn spawn(self: &Arc<Self>) {
        let runtime = self.clone();
        let _ = thread::Builder::new()
            .name("sabicheiro-capture".into())
            .spawn(move || {
                while !runtime.shutdown.load(Ordering::Relaxed) {
                    let started = Instant::now();
                    if let Ok(mut engine) = runtime.engine.lock() {
                        engine.tick();
                    }
                    let elapsed = started.elapsed();
                    if elapsed < TICK_INTERVAL {
                        thread::sleep(TICK_INTERVAL - elapsed);
                    }
                }
            });
    }

    fn shutdown(&self) {
        if let Some(logging) = &self.logging {
            logging.info("Sabicheiro desktop runtime stopping");
        }
        self.shutdown.store(true, Ordering::Relaxed);
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
    fn tick(&mut self) {
        let now_ms = sabicheiro_core::now_ms();
        self.drain_dns_results();
        self.drain_platform_events(now_ms);
        self.refresh_selected_interface();
        match self.capture_mode {
            CaptureMode::Off => {}
            CaptureMode::Demo => {
                let events = self.demo.next_batch(now_ms);
                self.process_events(events, true);
            }
            CaptureMode::Live if self.last_capture_poll.elapsed() >= LIVE_POLL_INTERVAL => {
                self.last_capture_poll = Instant::now();
                let result = self.provider.as_mut().map(|provider| provider.snapshot());
                match result {
                    Some(Ok(mut sockets)) => {
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
                        }
                        let events = self.differ.diff(sockets, now_ms);
                        self.process_events(events, false);
                        self.capture_status = CaptureStatus::Capturing;
                    }
                    Some(Err(error)) => {
                        let message = error.to_string();
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
        self.aggregator.prune(now_ms);

        if self.capture_mode != CaptureMode::Off && self.last_persist.elapsed() >= PERSIST_INTERVAL
        {
            self.persist(now_ms);
            self.last_persist = Instant::now();
        }
        if self.last_diagnostics.elapsed() >= DIAGNOSTICS_INTERVAL {
            self.refresh_diagnostics();
            self.last_diagnostics = Instant::now();
        }
        if self.last_retention_cleanup.elapsed() >= RETENTION_CLEANUP_INTERVAL {
            if let Err(error) = self.storage.cleanup(self.settings.retention, now_ms) {
                self.diagnostics.last_error = Some(error.to_string());
            }
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

    fn process_events(&mut self, mut events: Vec<NormalizedNetworkEvent>, demo: bool) {
        for event in &mut events {
            if event.kind == NetworkEventKind::Opened
                && !event.key.remote.address.is_unspecified()
                && !demo
            {
                match self.geo.lookup(event.key.remote.address) {
                    Ok(lookup) => {
                        event.geo = lookup.location;
                        event.asn = lookup.asn;
                        if self.settings.reverse_dns_enabled
                            && lookup.scope == AddressScope::Public
                            && !self.dns.enqueue(DnsJob {
                                key: event.key.clone(),
                                address: event.key.remote.address,
                            })
                        {
                            self.diagnostics.dropped_ui_events =
                                self.diagnostics.dropped_ui_events.saturating_add(1);
                        }
                    }
                    Err(error) => self.diagnostics.last_error = Some(error.to_string()),
                }
                if let Some((hostname, expires_at_ms)) =
                    self.dns_observations.get(&event.key.remote.address)
                    && *expires_at_ms > event.timestamp_ms
                {
                    event.hostname = Some(hostname.clone());
                    event.domain = Some(hostname.clone());
                }
            }
        }
        self.events_since_rate = self
            .events_since_rate
            .saturating_add(events.len().try_into().unwrap_or(u64::MAX));
        self.aggregator.apply_batch(events);
    }

    fn refresh_selected_interface(&mut self) {
        if self.capture_mode != CaptureMode::Live
            || self.last_interface_refresh.elapsed() < INTERFACE_REFRESH_INTERVAL
        {
            return;
        }
        self.last_interface_refresh = Instant::now();
        let Some(provider) = self.provider.as_mut() else {
            return;
        };
        let Ok(interfaces) = provider.interfaces() else {
            return;
        };
        self.vpn_active = interfaces
            .iter()
            .any(|network_interface| network_interface.is_up && network_interface.is_vpn);
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
        self.settings.selected_interface = None;
        let _ = self.storage.save_settings(&self.settings);
        self.diagnostics.last_error =
            Some("The selected adapter disappeared; capture recovered on All interfaces.".into());
    }

    #[cfg(target_os = "windows")]
    fn drain_platform_events(&mut self, now_ms: i64) {
        self.dns_observations
            .retain(|_, (_, expires_at_ms)| *expires_at_ms > now_ms);
        let Some(capture) = self.event_capture.as_mut() else {
            return;
        };
        let (mut events, observations) = capture.drain(now_ms);
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
        for observation in observations {
            self.dns_observations.insert(
                observation.address,
                (observation.hostname.clone(), observation.expires_at_ms),
            );
            let keys = self
                .aggregator
                .connections()
                .filter(|connection| connection.key.remote.address == observation.address)
                .map(|connection| connection.key.clone())
                .collect::<Vec<_>>();
            for key in keys {
                self.aggregator.enrich(
                    &key,
                    Some(observation.hostname.clone()),
                    Some(observation.hostname.clone()),
                    None,
                    None,
                );
            }
            self.diagnostics.dns_correlations_observed =
                self.diagnostics.dns_correlations_observed.saturating_add(1);
        }
        self.diagnostics.packet_events_observed = self
            .diagnostics
            .packet_events_observed
            .saturating_add(events.len().try_into().unwrap_or(u64::MAX));
        self.diagnostics.dropped_capture_events = capture.dropped();
        self.diagnostics.capture_queue_depth = capture.pending().saturating_add(self.dns.pending());
        self.diagnostics.capture_queue_capacity =
            capture.capacity().saturating_add(DNS_QUEUE_CAPACITY);
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
                .enrich(&result.key, result.hostname, domain, None, None);
        }
    }

    fn persist(&mut self, now_ms: i64) {
        let Some(session_id) = self.session.as_ref().map(|session| session.id.clone()) else {
            return;
        };
        let connections = self.aggregator.connections().cloned().collect::<Vec<_>>();
        if let Err(error) = self.storage.save_connections(&session_id, &connections) {
            self.diagnostics.last_error = Some(error.to_string());
        }
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
                        counters: counters_from_rates(rates),
                        rates,
                        source,
                    }),
            );
        }
        if let Err(error) = self.storage.save_traffic_samples(&samples) {
            self.diagnostics.last_error = Some(error.to_string());
        }
        if self.capture_mode == CaptureMode::Live {
            if let Err(error) = self.storage.save_baselines(self.aggregator.baselines()) {
                self.diagnostics.last_error = Some(error.to_string());
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
        self.differ = SnapshotDiffer::default();
        self.provider = None;
        #[cfg(target_os = "windows")]
        {
            self.event_capture = None;
        }
        self.dns_observations.clear();
        self.selected_interface_addresses.clear();
        self.vpn_active = false;
        self.capture_mode = request.mode;
        self.selected_interface = request.selected_interface.clone();
        self.settings.selected_interface = request.selected_interface;
        self.paused_view_events = 0;
        self.view_paused = false;

        match request.mode {
            CaptureMode::Off => {
                self.capture_status = CaptureStatus::Stopped;
                self.diagnostics.backend = "stopped".into();
                self.diagnostics.capture_event_source = "none".into();
                self.diagnostics.flow_counters_available = false;
            }
            CaptureMode::Demo => {
                self.demo = sabicheiro_core::DemoGenerator::new(self.settings.demo_seed);
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
                self.selected_interface_addresses = selected_addresses;
                self.vpn_active = vpn_active;
                self.diagnostics.backend = provider.name().into();
                self.provider = Some(provider);
                self.diagnostics.capture_event_source = "socket_polling".into();
                self.diagnostics.flow_counters_available = false;
                #[cfg(target_os = "windows")]
                match WindowsEventCapture::start() {
                    Ok(capture) => {
                        self.event_capture = Some(capture);
                        self.diagnostics.backend = "windows-etw + windows-ip-helper".into();
                        self.diagnostics.capture_event_source = "windows_etw".into();
                        self.diagnostics.flow_counters_available = true;
                        self.diagnostics.last_error = None;
                    }
                    Err(error) => {
                        self.diagnostics.last_error = Some(error.to_string());
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
        let id = format!("session-{started_at_ms}");
        self.storage
            .start_session(&id, started_at_ms)
            .map_err(|error| CommandError::recoverable("storage_unavailable", error))?;
        self.session = Some(SessionSummary {
            id,
            started_at_ms,
            ended_at_ms: None,
            connection_count: 0,
            process_count: 0,
            country_count: 0,
            destination_count: 0,
            recording_count: 0,
            traffic: TrafficCounters::default(),
        });
        Ok(())
    }

    fn stop_current_session(&mut self, ended_at_ms: i64) {
        if self.session.is_none() {
            return;
        }
        self.update_session_summary(ended_at_ms);
        if let Some(mut summary) = self.session.take() {
            summary.ended_at_ms = Some(ended_at_ms);
            if let Err(error) = self.storage.finish_session(&summary) {
                self.diagnostics.last_error = Some(error.to_string());
            }
        }
        if self.capture_mode == CaptureMode::Live {
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

    fn refresh_diagnostics(&mut self) {
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
        self.diagnostics.capture_queue_depth = event_depth.saturating_add(self.dns.pending());
        self.diagnostics.capture_queue_capacity = event_capacity.saturating_add(DNS_QUEUE_CAPACITY);
        self.diagnostics.queue_saturation = self.diagnostics.capture_queue_depth as f32
            / self.diagnostics.capture_queue_capacity.max(1) as f32;
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
        let session_id = self.session.as_ref().map(|session| session.id.as_str());
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
fn get_snapshot(state: State<'_, AppState>) -> Result<DashboardSnapshot, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))
        .map(|mut engine| engine.snapshot())
}

#[tauri::command]
fn set_capture_state(
    request: SetCaptureRequest,
    state: State<'_, AppState>,
) -> Result<CaptureSummary, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .set_capture(request)
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
        }),
        Err(error) => checks.push(SelfTestCheck {
            id: "interfaces".into(),
            level: SelfTestLevel::Fail,
            title: "Interface discovery failed".into(),
            detail: error.to_string(),
            diagnostic_code: Some("interface_unavailable".into()),
        }),
    }

    let loopback =
        std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).and_then(|listener| {
            let address = listener.local_addr()?;
            let client = std::net::TcpStream::connect(address)?;
            let (server, _) = listener.accept()?;
            Ok((listener, client, server))
        });
    match loopback.and_then(|proof| {
        let sockets = provider
            .snapshot()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        Ok((proof, sockets))
    }) {
        Ok((_proof, sockets)) => {
            checks.push(SelfTestCheck {
                id: "socket_table".into(),
                level: SelfTestLevel::Pass,
                title: "Socket tables are readable".into(),
                detail: format!(
                    "{} TCP/UDP rows observed during a local loopback proof",
                    sockets.len()
                ),
                diagnostic_code: None,
            });
            let attributed = sockets.iter().any(|socket| {
                socket.identity.pid == Some(std::process::id())
                    && socket.process.name != "Unknown process"
            });
            checks.push(SelfTestCheck {
                id: "process_attribution".into(),
                level: if attributed { SelfTestLevel::Pass } else { SelfTestLevel::Warn },
                title: if attributed {
                    "Process attribution is working".into()
                } else {
                    "The proof socket could not be attributed".into()
                },
                detail: if attributed {
                    "The current Sabicheiro process was resolved from a real loopback connection.".into()
                } else {
                    "Socket visibility works, but the operating system did not expose the proof PID in time.".into()
                },
                diagnostic_code: (!attributed).then(|| "process_attribution_partial".into()),
            });
        }
        Err(error) => checks.push(SelfTestCheck {
            id: "socket_table".into(),
            level: SelfTestLevel::Fail,
            title: "Socket table proof failed".into(),
            detail: error.to_string(),
            diagnostic_code: Some("socket_table_unavailable".into()),
        }),
    }

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
    });
    let geo = engine.geo.status();
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
    });
    Ok(CaptureSelfTest {
        generated_at_ms: sabicheiro_core::now_ms(),
        checks,
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
    let entries = engine
        .storage
        .query_history(&query)
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
        entries,
        sessions,
        recordings,
    })
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
                    .map(|geo| geo.country_name.clone())
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
    engine.set_capture(request)
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
    settings: AppSettings,
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
    let mut engine = state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine
        .storage
        .save_settings(&settings)
        .map_err(|error| CommandError::recoverable("storage_unavailable", error))?;
    engine.settings = settings;
    Ok(engine.settings.clone())
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
        })
        .map_err(|error| CommandError::recoverable("history_unavailable", error))?;
    Ok(reconstruct_at(&entries, at_ms))
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
fn get_geo_database_status(state: State<'_, AppState>) -> Result<GeoDatabaseStatus, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))
        .map(|engine| engine.geo.status())
}

#[tauri::command]
async fn update_geo_database(
    state: State<'_, AppState>,
) -> Result<GeoDatabaseStatus, CommandError> {
    let runtime = state.0.clone();
    let directory = runtime.data_directory.join("geoip");
    let updated = tauri::async_runtime::spawn_blocking(move || update_dbip_lite(directory))
        .await
        .map_err(|error| CommandError::recoverable("geo_update_failed", error))?
        .map_err(|error| CommandError::recoverable("geo_update_failed", error))?;
    let mut engine = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine
        .geo
        .reload(&updated.city_path, &updated.asn_path)
        .map_err(|error| CommandError::recoverable("geo_update_failed", error))?;
    Ok(engine.geo.status())
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
    let mut engine = runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?;
    engine.geo = resolver;
    Ok(engine.geo.status())
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
    runtime
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .start_recording(request)
}

#[tauri::command]
fn stop_recording(state: State<'_, AppState>) -> Result<RecordingState, CommandError> {
    state
        .0
        .engine
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "runtime state is unavailable"))?
        .stop_recording()
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

pub fn run() {
    let application = tauri::Builder::default()
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&data_directory)?;
            let logging = setup_logging(&data_directory);
            let runtime = Arc::new(Runtime::new(data_directory, logging));
            runtime.spawn();
            if let Some(logging) = &runtime.logging {
                logging.info("Sabicheiro desktop runtime started");
            }
            app.manage(AppState(runtime));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            set_capture_state,
            list_interfaces,
            run_capture_self_test,
            set_view_paused,
            query_history,
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
            get_geo_database_status,
            update_geo_database,
            import_geo_database,
            export_view,
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
