use sabicheiro_core::{
    AddressScope, Connection, Endpoint, ExportFormat, ExportRedaction, ExportRequest, HistoryQuery,
    NetworkProtocol, ProcessInfo,
};
use sabicheiro_storage::Storage;
use serde::Serialize;
use std::{
    collections::HashSet,
    env, fs,
    hint::black_box,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    time::Instant,
};

const ROWS: usize = 100_000;
const PAGE_SIZE: usize = 500;
const ITERATIONS: usize = 20;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    rows: usize,
    page_size: usize,
    pages: usize,
    insert_ms: f64,
    first_page_p50_ms: f64,
    first_page_p95_ms: f64,
    filtered_page_p95_ms: f64,
    first_page_json_bytes: usize,
    traversal_ms: f64,
    unique_rows_traversed: usize,
    export_ms: f64,
    exported_rows: usize,
    export_bytes: u64,
    database_bytes: u64,
    limits_passed: bool,
}

fn percentile(values: &mut [f64], fraction: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    values[((values.len() as f64 * fraction).ceil() as usize).saturating_sub(1)]
}

fn connection(index: usize) -> Connection {
    let timestamp = 1_700_000_000_000 + index as i64;
    let mut connection = Connection::placeholder(timestamp);
    connection.id = format!("history-{index:06}");
    connection.first_seen_ms = timestamp;
    connection.last_seen_ms = timestamp;
    connection.key.pid = Some((index % 8_192) as u32 + 1);
    connection.key.process_instance = Some(format!("process-instance-{index}"));
    connection.key.local = Endpoint::new(
        IpAddr::V4(Ipv4Addr::new(
            10,
            0,
            ((index >> 8) & 0xff) as u8,
            index as u8,
        )),
        40_000 + (index % 20_000) as u16,
    );
    connection.key.remote = Endpoint::new(
        IpAddr::V4(Ipv4Addr::new(
            198,
            18 + ((index >> 16) & 1) as u8,
            ((index >> 8) & 0xff) as u8,
            index as u8,
        )),
        if index.is_multiple_of(7) { 53 } else { 443 },
    );
    connection.key.remote.scope = AddressScope::Benchmark;
    connection.protocol = if index.is_multiple_of(7) {
        NetworkProtocol::Udp
    } else {
        NetworkProtocol::Tcp
    };
    connection.key.protocol = connection.protocol;
    connection.key.capture_generation = 1;
    connection.key.flow_generation = index as u64 + 1;
    connection.process = ProcessInfo {
        pid: connection.key.pid,
        name: format!("history-process-{}", index % 64),
        executable: Some("history-fixture.exe".into()),
        started_at_ms: Some(timestamp.saturating_sub(60_000)),
        ..ProcessInfo::default()
    };
    connection.domain = Some(format!("host-{}.invalid", index % 512));
    connection.active = index.is_multiple_of(3);
    connection.traffic.bytes_received = index as u64 * 97;
    connection.traffic.bytes_sent = index as u64 * 31;
    connection
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args().nth(1).map(PathBuf::from);
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("history.sqlite3");
    let mut storage = Storage::open(&database)?;
    storage.start_session("history-benchmark", 1_700_000_000_000)?;

    let inserted = Instant::now();
    for start in (0..ROWS).step_by(2_000) {
        let batch = (start..(start + 2_000).min(ROWS))
            .map(connection)
            .collect::<Vec<_>>();
        storage.save_connections("history-benchmark", &batch)?;
    }
    let insert_ms = inserted.elapsed().as_secs_f64() * 1_000.0;

    let base_query = HistoryQuery {
        session_id: Some("history-benchmark".into()),
        limit: PAGE_SIZE,
        ..HistoryQuery::default()
    };
    let mut first_page_ms = Vec::with_capacity(ITERATIONS);
    let mut filtered_page_ms = Vec::with_capacity(ITERATIONS);
    let mut first_page_json_bytes = 0;
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        let page = storage.query_history_page(&base_query)?;
        let json = serde_json::to_vec(&(&page.entries, &page.next_cursor, page.has_more))?;
        first_page_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
        first_page_json_bytes = json.len();
        black_box(json);

        let started = Instant::now();
        let page = storage.query_history_page(&HistoryQuery {
            query: Some("protocol:tcp scope:benchmark".into()),
            ..base_query.clone()
        })?;
        black_box(page);
        filtered_page_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    let first_page_p50_ms = percentile(&mut first_page_ms.clone(), 0.50);
    let first_page_p95_ms = percentile(&mut first_page_ms, 0.95);
    let filtered_page_p95_ms = percentile(&mut filtered_page_ms, 0.95);

    let traversed = Instant::now();
    let mut query = base_query.clone();
    let mut ids = HashSet::with_capacity(ROWS);
    let mut pages = 0;
    loop {
        let page = storage.query_history_page(&query)?;
        pages += 1;
        ids.extend(page.entries.into_iter().map(|entry| entry.connection.id));
        if !page.has_more {
            break;
        }
        query.cursor = page.next_cursor;
    }
    let traversal_ms = traversed.elapsed().as_secs_f64() * 1_000.0;

    let export_path = directory.path().join("history.csv");
    let exported = Instant::now();
    let export = storage.export(&ExportRequest {
        format: ExportFormat::Csv,
        path: export_path.to_string_lossy().into_owned(),
        session_id: Some("history-benchmark".into()),
        process_key: None,
        from_ms: None,
        to_ms: None,
        query: None,
        redaction: ExportRedaction::default(),
    })?;
    let export_ms = exported.elapsed().as_secs_f64() * 1_000.0;
    let database_bytes = fs::metadata(&database)?.len();
    let limits_passed = ids.len() == ROWS
        && export.records == ROWS
        && first_page_json_bytes < 1024 * 1024
        && first_page_p95_ms <= 100.0
        && filtered_page_p95_ms <= 100.0;
    let report = Report {
        rows: ROWS,
        page_size: PAGE_SIZE,
        pages,
        insert_ms,
        first_page_p50_ms,
        first_page_p95_ms,
        filtered_page_p95_ms,
        first_page_json_bytes,
        traversal_ms,
        unique_rows_traversed: ids.len(),
        export_ms,
        exported_rows: export.records,
        export_bytes: export.bytes_written,
        database_bytes,
        limits_passed,
    };
    let json = serde_json::to_string_pretty(&report)?;
    println!("{json}");
    if let Some(output) = output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output, format!("{json}\n"))?;
    }
    if !limits_passed {
        return Err("history pagination limits exceeded".into());
    }
    Ok(())
}
