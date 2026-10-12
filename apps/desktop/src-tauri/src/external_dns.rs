use super::{AppState, CommandError};
use sabicheiro_core::{
    DnsImportPreview, DnsImportReport, DnsImportRequest, ExternalDnsFilter, ExternalDnsPage,
    parse_dns_import,
};
use sabicheiro_storage::Storage;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::State;

#[tauri::command]
pub async fn preview_dns_import(
    request: DnsImportRequest,
) -> Result<DnsImportPreview, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let rows = parse_dns_import(&request)
            .map_err(|error| CommandError::recoverable("dns_import_invalid", error))?;
        Ok(DnsImportPreview {
            total_rows: rows.len(),
            source: request.source.trim().into(),
            sample: rows.into_iter().take(20).collect(),
        })
    })
    .await
    .map_err(|error| CommandError::recoverable("dns_import_failed", error))?
}

#[tauri::command]
pub async fn import_dns_queries(
    request: DnsImportRequest,
    job_id: String,
    state: State<'_, AppState>,
) -> Result<DnsImportReport, CommandError> {
    if job_id.is_empty() || job_id.len() > 64 {
        return Err(CommandError::recoverable(
            "dns_import_invalid",
            "Invalid import identifier",
        ));
    }
    let runtime = state.0.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut job = runtime
            .dns_import_job
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "Import state is unavailable"))?;
        if job.is_some() {
            return Err(CommandError::recoverable(
                "dns_import_busy",
                "Another DNS import is running",
            ));
        }
        *job = Some((job_id.clone(), cancel.clone()));
    }
    let worker_runtime = runtime.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let queries = parse_dns_import(&request)
            .map_err(|error| CommandError::recoverable("dns_import_invalid", error))?;
        let path = worker_runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "Storage state is unavailable"))?
            .storage
            .database_path()
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| {
                CommandError::recoverable(
                    "storage_unavailable",
                    "Persistent storage is unavailable",
                )
            })?;
        let mut storage = Storage::open(path)
            .map_err(|error| CommandError::recoverable("storage_unavailable", error))?;
        storage
            .import_external_dns(&request.source, &queries, &cancel)
            .map_err(|error| CommandError::recoverable("dns_import_failed", error))
    })
    .await
    .map_err(|error| CommandError::recoverable("dns_import_failed", error));
    if let Ok(mut job) = runtime.dns_import_job.lock() {
        if job.as_ref().is_some_and(|(id, _)| id == &job_id) {
            *job = None;
        }
    }
    result?
}

#[tauri::command]
pub fn cancel_dns_import(job_id: String, state: State<'_, AppState>) -> Result<bool, CommandError> {
    let job = state
        .0
        .dns_import_job
        .lock()
        .map_err(|_| CommandError::fatal("state_poisoned", "Import state is unavailable"))?;
    if let Some((_, cancel)) = job.as_ref().filter(|(id, _)| id == &job_id) {
        cancel.store(true, Ordering::Release);
        return Ok(true);
    }
    Ok(false)
}

#[tauri::command]
pub async fn query_external_dns(
    filter: ExternalDnsFilter,
    state: State<'_, AppState>,
) -> Result<ExternalDnsPage, CommandError> {
    let runtime = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = runtime
            .engine
            .lock()
            .map_err(|_| CommandError::fatal("state_poisoned", "Storage state is unavailable"))?
            .storage
            .database_path()
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| {
                CommandError::recoverable(
                    "storage_unavailable",
                    "Persistent storage is unavailable",
                )
            })?;
        let storage = Storage::open(path)
            .map_err(|error| CommandError::recoverable("storage_unavailable", error))?;
        storage
            .query_external_dns(&filter)
            .map_err(|error| CommandError::recoverable("dns_query_failed", error))
    })
    .await
    .map_err(|error| CommandError::recoverable("dns_query_failed", error))?
}
