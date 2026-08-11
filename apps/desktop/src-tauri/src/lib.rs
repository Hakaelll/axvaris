use sabicheiro_capture::{
    NetworkCaptureProvider, RecordingController, SnapshotDiffer, system_provider,
};
use sabicheiro_core::{
    AppSettings, Bookmark, CaptureMode, CaptureStatus, CaptureSummary, Connection,
    ConnectionAggregator, ConnectionHistoryStats, ConnectionKey, DashboardSnapshot, Diagnostics,
    ExportRequest, ExportResult, FocusRules, GeoDatabaseStatus, HistoryEntry, HistoryQuery,
    IgnoreRule, NetworkEventKind, NormalizedNetworkEvent, RecordingRequest, RecordingState,
    RecordingStatus, RecordingSummary, RetentionPolicy, RuleKind, SessionSummary, TrafficCounters,
    TransferRates, reconstruct_at,
};
use sabicheiro_geo::{AddressScope, DnsResolver, LocalGeoResolver, update_dbip_lite};
use sabicheiro_storage::Storage;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
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
    differ: SnapshotDiffer,
    demo: sabicheiro_core::DemoGenerator,
    storage: Storage,
    geo: LocalGeoResolver,
    dns: DnsService,
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
                differ: SnapshotDiffer::default(),
                demo: sabicheiro_core::DemoGenerator::new(settings.demo_seed),
                storage,
                geo,
                dns: DnsService::start(),
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
                        if !self.settings.show_listening_sockets {
                            sockets.retain(|socket| socket.has_remote_peer());
                        }
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
            }
        }
        self.events_since_rate = self
            .events_since_rate
            .saturating_add(events.len().try_into().unwrap_or(u64::MAX));
        self.aggregator.apply_batch(events);
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
            }
            CaptureMode::Demo => {
                self.demo = sabicheiro_core::DemoGenerator::new(self.settings.demo_seed);
                self.capture_status = CaptureStatus::Capturing;
                self.diagnostics.backend = "deterministic-demo".into();
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
        let selected_interface = self.selected_interface.as_deref();
        let mut received_bytes = 0_u64;
        let mut transmitted_bytes = 0_u64;
        let mut packets = 0_u64;
        for (name, network) in &self.network_metrics {
            if selected_interface.is_some_and(|selected| selected != name) {
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
        self.diagnostics.capture_queue_depth = self.dns.pending();
        self.diagnostics.queue_saturation = self.dns.pending() as f32 / DNS_QUEUE_CAPACITY as f32;
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
            set_view_paused,
            query_history,
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
            export_view,
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
