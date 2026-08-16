use sabicheiro_core::ProcessInfo;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};
use sysinfo::{Pid, ProcessesToUpdate, System};

pub(crate) struct ProcessResolver {
    system: System,
    cache: HashMap<u32, ProcessInfo>,
    last_refresh: Option<Instant>,
}

impl ProcessResolver {
    pub(crate) fn new() -> Self {
        Self {
            system: System::new(),
            cache: HashMap::new(),
            last_refresh: None,
        }
    }

    pub(crate) fn refresh(&mut self, pids: impl IntoIterator<Item = u32>) {
        let mut unique = pids.into_iter().filter(|pid| *pid > 0).collect::<Vec<_>>();
        unique.sort_unstable();
        unique.dedup();
        if unique.is_empty() {
            return;
        }
        if self
            .last_refresh
            .is_some_and(|last| last.elapsed() < Duration::from_millis(500))
        {
            return;
        }
        let sysinfo_pids = unique
            .iter()
            .copied()
            .map(Pid::from_u32)
            .collect::<Vec<_>>();
        self.system
            .refresh_processes(ProcessesToUpdate::Some(&sysinfo_pids), true);
        let mut parent_pids = unique
            .iter()
            .filter_map(|pid| self.system.process(Pid::from_u32(*pid)))
            .filter_map(|process| process.parent())
            .collect::<Vec<_>>();
        parent_pids.sort_unstable();
        parent_pids.dedup();
        if !parent_pids.is_empty() {
            self.system
                .refresh_processes(ProcessesToUpdate::Some(&parent_pids), false);
        }
        for pid in unique {
            let process = self.system.process(Pid::from_u32(pid));
            let info = process.map_or_else(
                || ProcessInfo {
                    pid: Some(pid),
                    ..ProcessInfo::default()
                },
                |process| {
                    let path = process
                        .exe()
                        .map(|value| value.to_string_lossy().into_owned());
                    let executable = process
                        .exe()
                        .and_then(|value| value.file_name())
                        .map(|value| value.to_string_lossy().into_owned());
                    let parent_pid = process.parent().map(Pid::as_u32);
                    let parent_instance = process.parent().and_then(|parent| {
                        self.system.process(parent).map(|parent_process| {
                            format!(
                                "pid:{}:start:{}",
                                parent.as_u32(),
                                parent_process.start_time().saturating_mul(1_000)
                            )
                        })
                    });
                    let executable_revision_id = path
                        .as_deref()
                        .and_then(|path| executable_revision_id(Path::new(path)));
                    ProcessInfo {
                        pid: Some(pid),
                        name: process.name().to_string_lossy().into_owned(),
                        executable,
                        executable_path: path,
                        started_at_ms: Some(
                            process.start_time().min((i64::MAX / 1_000) as u64) as i64 * 1_000,
                        ),
                        icon_data_url: None,
                        parent_pid,
                        parent_instance,
                        user_id: process.user_id().map(|user_id| format!("{user_id:?}")),
                        session_id: process.session_id().map(Pid::as_u32),
                        cgroup: process_cgroup(pid),
                        network_namespace: process_network_namespace(pid),
                        executable_revision_id,
                    }
                },
            );
            self.cache.insert(pid, info);
        }
        self.last_refresh = Some(Instant::now());
    }

    pub(crate) fn resolve(&self, pid: Option<u32>) -> ProcessInfo {
        pid.and_then(|pid| self.cache.get(&pid).cloned())
            .unwrap_or_else(|| ProcessInfo {
                pid,
                ..ProcessInfo::default()
            })
    }
}

fn executable_revision_id(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| {
            duration.as_millis().min(u128::from(u64::MAX)) as u64
        });
    Some(format!(
        "file:{}:{}:{}",
        metadata.len(),
        modified_ms,
        path.to_string_lossy().to_ascii_lowercase()
    ))
}

#[cfg(target_os = "linux")]
fn process_cgroup(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .ok()
        .and_then(|value| {
            value
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_owned)
        })
}

#[cfg(not(target_os = "linux"))]
fn process_cgroup(_pid: u32) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn process_network_namespace(pid: u32) -> Option<String> {
    fs::read_link(format!("/proc/{pid}/ns/net"))
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

#[cfg(not(target_os = "linux"))]
fn process_network_namespace(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_the_current_pid_and_does_not_invent_missing_processes() {
        let current_pid = std::process::id();
        let mut resolver = ProcessResolver::new();
        resolver.refresh([current_pid]);

        let current = resolver.resolve(Some(current_pid));
        assert_eq!(current.pid, Some(current_pid));
        assert_ne!(current.name, "Unknown process");
        assert!(current.started_at_ms.is_some());

        let missing = resolver.resolve(Some(u32::MAX));
        assert_eq!(missing.pid, Some(u32::MAX));
        assert_eq!(missing.name, "Unknown process");
    }
}
