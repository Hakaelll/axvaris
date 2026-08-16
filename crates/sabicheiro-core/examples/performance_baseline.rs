use sabicheiro_core::{
    AddressScope, Connection, ConnectionFilter, Endpoint, NetworkProtocol, ProcessInfo,
};
use serde::Serialize;
use std::{
    env, fs,
    hint::black_box,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessesToUpdate, System};

#[derive(Serialize)]
struct CaseResult {
    connections: usize,
    generation_ms: f64,
    filter_p95_ms: f64,
    serialization_p95_ms: f64,
    serialized_bytes: usize,
    estimated_model_bytes: usize,
    limits_passed: bool,
}

#[derive(Serialize)]
struct PerformanceReport {
    generated_at_ms: i64,
    iterations: usize,
    limits: Limits,
    cases: Vec<CaseResult>,
    soak: Option<SoakResult>,
}

#[derive(Clone, Copy, Serialize)]
struct Limits {
    filter_p95_ms: f64,
    serialization_p95_ms: f64,
    serialized_bytes_per_connection: usize,
}

#[derive(Serialize)]
struct SoakResult {
    duration_seconds: u64,
    samples: usize,
    interval_ms: u64,
    latency_p95_ms: f64,
    minimum_memory_bytes: u64,
    maximum_memory_bytes: u64,
    memory_growth_bytes: u64,
    limits_passed: bool,
}

fn percentile95(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[((values.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)]
}

fn connection(index: usize) -> Connection {
    let octets = [
        11,
        ((index >> 16) & 0xff) as u8,
        ((index >> 8) & 0xff) as u8,
        (index & 0xff) as u8,
    ];
    let mut connection = Connection::placeholder(index as i64);
    connection.id = format!("synthetic-{index}");
    connection.key.pid = Some((index % 8_192) as u32 + 1);
    connection.key.process_instance = Some(format!("process-instance-{index}"));
    connection.key.local = Endpoint::new(
        IpAddr::V4(Ipv4Addr::new(
            10,
            0,
            ((index >> 8) & 0xff) as u8,
            (index & 0xff) as u8,
        )),
        40_000 + (index % 20_000) as u16,
    );
    connection.key.remote = Endpoint::new(IpAddr::V4(octets.into()), 443);
    connection.key.remote.scope = AddressScope::Global;
    connection.key.protocol = if index % 7 == 0 {
        NetworkProtocol::Udp
    } else {
        NetworkProtocol::Tcp
    };
    connection.key.capture_generation = 1;
    connection.key.flow_generation = index as u64 + 1;
    connection.process = ProcessInfo {
        pid: connection.key.pid,
        name: format!("synthetic-process-{}", index % 64),
        executable: Some("synthetic-process.exe".into()),
        started_at_ms: Some(1_700_000_000_000 + index as i64),
        ..ProcessInfo::default()
    };
    connection.domain = Some(format!("host-{}.invalid", index % 512));
    connection.active = true;
    connection.traffic.bytes_received = index as u64 * 97;
    connection.traffic.bytes_sent = index as u64 * 31;
    connection
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let output = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .map(PathBuf::from);
    let soak_seconds = arguments
        .windows(2)
        .find(|pair| pair[0] == "--soak-seconds")
        .map(|pair| pair[1].parse::<u64>())
        .transpose()?
        .unwrap_or(0);
    let iterations = 9;
    let limits = Limits {
        filter_p95_ms: 100.0,
        serialization_p95_ms: 1_500.0,
        serialized_bytes_per_connection: 2_500,
    };
    let filter: ConnectionFilter = "scope:global protocol:tcp -domain:blocked.invalid".parse()?;
    let mut cases = Vec::new();
    for count in [1_000, 10_000, 50_000, 100_000] {
        let generated = Instant::now();
        let connections = (0..count).map(connection).collect::<Vec<_>>();
        let generation_ms = generated.elapsed().as_secs_f64() * 1_000.0;
        let mut filter_ms = Vec::with_capacity(iterations);
        let mut serialize_ms = Vec::with_capacity(iterations);
        let mut serialized_bytes = 0;
        for _ in 0..iterations {
            let started = Instant::now();
            black_box(
                connections
                    .iter()
                    .filter(|item| filter.matches(item))
                    .count(),
            );
            filter_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
            let started = Instant::now();
            let json = serde_json::to_vec(&connections)?;
            serialize_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
            serialized_bytes = json.len();
            black_box(json);
        }
        let filter_p95_ms = percentile95(&mut filter_ms);
        let serialization_p95_ms = percentile95(&mut serialize_ms);
        let limits_passed = filter_p95_ms <= limits.filter_p95_ms
            && serialization_p95_ms <= limits.serialization_p95_ms
            && serialized_bytes <= limits.serialized_bytes_per_connection * count;
        cases.push(CaseResult {
            connections: count,
            generation_ms,
            filter_p95_ms,
            serialization_p95_ms,
            serialized_bytes,
            estimated_model_bytes: std::mem::size_of_val(connections.as_slice()),
            limits_passed,
        });
    }
    let soak = (soak_seconds > 0)
        .then(|| run_soak(soak_seconds))
        .transpose()?;
    let report = PerformanceReport {
        generated_at_ms: sabicheiro_core::now_ms(),
        iterations,
        limits,
        cases,
        soak,
    };
    let json = serde_json::to_string_pretty(&report)?;
    println!("{json}");
    if let Some(output) = output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output, format!("{json}\n"))?;
    }
    if report.cases.iter().any(|case| !case.limits_passed)
        || report.soak.as_ref().is_some_and(|soak| !soak.limits_passed)
    {
        return Err("one or more performance limits were exceeded".into());
    }
    Ok(())
}

fn run_soak(duration_seconds: u64) -> Result<SoakResult, Box<dyn std::error::Error>> {
    const INTERVAL: Duration = Duration::from_millis(400);
    const LATENCY_LIMIT_MS: f64 = 50.0;
    const MEMORY_GROWTH_LIMIT_BYTES: u64 = 32 * 1_024 * 1_024;
    let connections = (0..2_000).map(connection).collect::<Vec<_>>();
    let filter: ConnectionFilter = "scope:global -domain:blocked.invalid".parse()?;
    let deadline = Instant::now() + Duration::from_secs(duration_seconds);
    let pid = sysinfo::get_current_pid()?;
    let mut system = System::new();
    let mut latencies = Vec::new();
    let mut memory_samples = Vec::new();
    while Instant::now() < deadline {
        let cycle = Instant::now();
        black_box(
            connections
                .iter()
                .filter(|item| filter.matches(item))
                .count(),
        );
        black_box(serde_json::to_vec(&connections)?);
        latencies.push(cycle.elapsed().as_secs_f64() * 1_000.0);
        system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        if let Some(process) = system.process(pid) {
            memory_samples.push(process.memory());
        }
        if cycle.elapsed() < INTERVAL {
            thread::sleep(INTERVAL - cycle.elapsed());
        }
    }
    let latency_p95_ms = percentile95(&mut latencies);
    let minimum_memory_bytes = memory_samples.iter().copied().min().unwrap_or(0);
    let maximum_memory_bytes = memory_samples.iter().copied().max().unwrap_or(0);
    let memory_growth_bytes = maximum_memory_bytes.saturating_sub(minimum_memory_bytes);
    Ok(SoakResult {
        duration_seconds,
        samples: memory_samples.len(),
        interval_ms: INTERVAL.as_millis() as u64,
        latency_p95_ms,
        minimum_memory_bytes,
        maximum_memory_bytes,
        memory_growth_bytes,
        limits_passed: latency_p95_ms <= LATENCY_LIMIT_MS
            && memory_growth_bytes <= MEMORY_GROWTH_LIMIT_BYTES,
    })
}
