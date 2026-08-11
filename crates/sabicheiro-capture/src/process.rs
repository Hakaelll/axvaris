use sabicheiro_core::ProcessInfo;
use std::{
    collections::HashMap,
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
                    ProcessInfo {
                        pid: Some(pid),
                        name: process.name().to_string_lossy().into_owned(),
                        executable,
                        executable_path: path,
                        started_at_ms: Some(
                            process.start_time().min((i64::MAX / 1_000) as u64) as i64 * 1_000,
                        ),
                        icon_data_url: None,
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
