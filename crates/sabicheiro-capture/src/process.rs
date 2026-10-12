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
            self.cache.clear();
            self.system = System::new();
            return;
        }
        self.cache
            .retain(|pid, _| unique.binary_search(pid).is_ok());
        let starts = unique
            .iter()
            .map(|pid| (*pid, native_process_start_ms(*pid)))
            .collect::<HashMap<_, _>>();
        let new_pids = unique
            .iter()
            .copied()
            .filter(|pid| {
                !self.cache.get(pid).is_some_and(|cached| {
                    starts
                        .get(pid)
                        .copied()
                        .flatten()
                        .is_some_and(|start| cached.started_at_ms == Some(start))
                })
            })
            .collect::<Vec<_>>();
        let throttled = self
            .last_refresh
            .is_some_and(|last| last.elapsed() < Duration::from_millis(500));
        if throttled {
            if new_pids.is_empty() {
                return;
            }
            unique = new_pids;
        } else {
            self.system = System::new();
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
                    let parent_pid = process.parent().map(Pid::as_u32);
                    let parent_instance = parent_pid.and_then(|parent| {
                        let parent_start = native_process_start_ms(parent)?;
                        let child_start = starts.get(&pid).copied().flatten()?;
                        (parent_start <= child_start)
                            .then(|| format!("pid:{parent}:start:{parent_start}"))
                    });
                    let executable_revision_id = path
                        .as_deref()
                        .and_then(|path| executable_revision_id(Path::new(path)));
                    ProcessInfo {
                        pid: Some(pid),
                        name: process.name().to_string_lossy().into_owned(),
                        executable,
                        executable_path: path,
                        started_at_ms: starts.get(&pid).copied().flatten().or(Some(
                            process.start_time().min((i64::MAX / 1_000) as u64) as i64 * 1_000,
                        )),
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
        // Keep only instances still present in sysinfo; historical events own their copies.
        self.cache
            .retain(|pid, _| self.system.process(Pid::from_u32(*pid)).is_some());
        self.last_refresh = Some(Instant::now());
    }

    pub(crate) fn resolve(&self, pid: Option<u32>) -> ProcessInfo {
        pid.and_then(|pid| self.cache.get(&pid).cloned())
            .unwrap_or_else(|| ProcessInfo {
                pid,
                ..ProcessInfo::default()
            })
    }

    pub(crate) fn resolve_at(&self, pid: Option<u32>, observed_at_ms: i64) -> ProcessInfo {
        let process = self.resolve(pid);
        if process
            .started_at_ms
            .is_some_and(|start| start > observed_at_ms)
        {
            ProcessInfo {
                pid,
                ..ProcessInfo::default()
            }
        } else {
            process
        }
    }
}

#[cfg(target_os = "windows")]
fn native_process_start_ms(pid: u32) -> Option<i64> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    // The query-only handle is closed on every path and never grants process mutation.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut created: FILETIME = std::mem::zeroed();
        let mut exited: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        let ok = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user);
        CloseHandle(handle);
        if ok == 0 {
            return None;
        }
        let ticks = (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
        i64::try_from(ticks / 10_000)
            .ok()?
            .checked_sub(11_644_473_600_000)
    }
}

#[cfg(target_os = "linux")]
fn native_process_start_ms(pid: u32) -> Option<i64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let ticks: i64 = stat
        .rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()?;
    let boot: i64 = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .parse()
        .ok()?;
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if hz <= 0 {
        return None;
    }
    boot.checked_mul(1_000)?
        .checked_add(ticks.checked_mul(1_000)? / hz)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn native_process_start_ms(_pid: u32) -> Option<i64> {
    None
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
        executable_path_identity(path)
    ))
}

fn executable_path_identity(path: &Path) -> String {
    let value = path.to_string_lossy().into_owned();
    if cfg!(target_os = "windows") {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn newly_observed_pid_bypasses_the_refresh_throttle() {
        let mut resolver = ProcessResolver::new();
        resolver.last_refresh = Some(Instant::now());
        let pid = std::process::id();
        resolver.refresh([pid]);
        let process = resolver.resolve(Some(pid));
        assert_eq!(process.pid, Some(pid));
        assert!(process.started_at_ms.is_some());
        assert!(!process.name.is_empty());
    }

    #[test]
    fn missing_pid_does_not_inherit_a_cached_process() {
        let mut resolver = ProcessResolver::new();
        resolver.cache.insert(
            u32::MAX,
            ProcessInfo {
                name: "previous owner".into(),
                ..ProcessInfo::default()
            },
        );
        resolver.refresh([u32::MAX]);
        assert_eq!(resolver.resolve(Some(u32::MAX)).name, "Unknown process");
    }

    #[test]
    fn an_event_before_process_creation_cannot_belong_to_the_new_owner() {
        let mut resolver = ProcessResolver::new();
        resolver.cache.insert(
            7,
            ProcessInfo {
                pid: Some(7),
                name: "new owner".into(),
                started_at_ms: Some(2_000),
                ..ProcessInfo::default()
            },
        );
        assert_eq!(resolver.resolve_at(Some(7), 1_999).name, "Unknown process");
        assert_eq!(resolver.resolve_at(Some(7), 2_000).name, "new owner");
    }

    #[test]
    fn empty_snapshot_releases_process_metadata() {
        let mut resolver = ProcessResolver::new();
        resolver.refresh([std::process::id()]);
        assert!(!resolver.cache.is_empty());
        resolver.refresh([]);
        assert!(resolver.cache.is_empty());
        assert!(resolver.system.processes().is_empty());
    }
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
