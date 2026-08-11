use crate::CaptureError;
use sabicheiro_core::{RecordingRequest, RecordingState, RecordingStatus};
#[cfg(unix)]
use std::process::{Child, Stdio};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(target_os = "windows")]
const MAX_ROTATED_FILES: usize = 4;

#[derive(Default)]
pub struct RecordingController {
    state: RecordingState,
    platform_temporary_path: Option<PathBuf>,
    platform_stopped: bool,
    rotation_enabled: bool,
    #[cfg(target_os = "windows")]
    windows_hard_stop_bytes: Option<u64>,
    #[cfg(unix)]
    platform_child: Option<Child>,
}

impl RecordingController {
    pub fn state(&self) -> RecordingState {
        let mut state = self.state.clone();
        if let Some(path) = self
            .platform_temporary_path
            .as_ref()
            .filter(|_| state.status == RecordingStatus::Recording)
        {
            #[cfg(target_os = "windows")]
            {
                state.bytes_written = total_size(&pktmon_files(path));
            }
            #[cfg(unix)]
            {
                state.bytes_written = if self.rotation_enabled {
                    total_size(&dumpcap_files(path))
                } else {
                    fs::metadata(path).map_or(0, |metadata| metadata.len())
                };
            }
        }
        state
    }

    /// Returns true when the native Windows recorder has reached Sabicheiro's
    /// own bounded-file policy. Linux dumpcap enforces its four-file ring itself.
    pub fn limit_reached(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            if self.state.status != RecordingStatus::Recording {
                return false;
            }
            let Some(path) = self.platform_temporary_path.as_ref() else {
                return false;
            };
            let files = pktmon_files(path);
            let total = total_size(&files);
            self.windows_hard_stop_bytes
                .is_some_and(|limit| total >= limit)
                || (self.rotation_enabled && files.len() >= MAX_ROTATED_FILES)
        }

        #[cfg(not(target_os = "windows"))]
        false
    }

    pub fn start(&mut self, request: RecordingRequest) -> Result<RecordingState, CaptureError> {
        if self.state.status != RecordingStatus::Idle && self.state.status != RecordingStatus::Error
            || (self.state.status == RecordingStatus::Error
                && self.platform_temporary_path.is_some())
        {
            return Err(CaptureError::Recording(
                "a recording is already active".into(),
            ));
        }
        let output = validate_output(&request)?;
        self.platform_stopped = false;
        self.rotation_enabled = request.rotate;
        self.state = RecordingState {
            status: RecordingStatus::Starting,
            output_path: Some(output.to_string_lossy().into_owned()),
            started_at_ms: Some(sabicheiro_core::now_ms()),
            bytes_written: 0,
            max_file_size_mb: request.max_file_size_mb,
            duration_limit_seconds: request.duration_limit_seconds,
            message: None,
        };

        #[cfg(target_os = "windows")]
        {
            let outcome = (|| -> Result<RecordingState, CaptureError> {
                let temporary = output.with_extension("etl");
                if !pktmon_files(&temporary).is_empty() {
                    return Err(CaptureError::Recording(
                        "a temporary Pktmon recording with this name already exists".into(),
                    ));
                }
                let result = Command::new("pktmon.exe")
                    .args([
                        "start",
                        "--capture",
                        "--comp",
                        "nics",
                        "--pkt-size",
                        "0",
                        "--file-name",
                    ])
                    .arg(&temporary)
                    .args([
                        "--file-size",
                        &request.max_file_size_mb.to_string(),
                        "--log-mode",
                        if request.rotate {
                            "multi-file"
                        } else {
                            "circular"
                        },
                    ])
                    .output()?;
                if !result.status.success() {
                    let message = String::from_utf8_lossy(&result.stderr).trim().to_owned();
                    self.state.status = RecordingStatus::Error;
                    self.state.message = Some(if message.is_empty() {
                        "Pktmon could not start. Packet recording normally requires elevation."
                            .into()
                    } else {
                        message
                    });
                    return Err(CaptureError::PermissionDenied(
                        self.state.message.clone().unwrap_or_default(),
                    ));
                }
                self.platform_temporary_path = Some(temporary);
                let per_file_bytes = u64::from(request.max_file_size_mb).saturating_mul(1_048_576);
                self.windows_hard_stop_bytes =
                    Some(per_file_bytes.saturating_mul(if request.rotate {
                        MAX_ROTATED_FILES as u64
                    } else {
                        1
                    }));
                self.state.status = RecordingStatus::Recording;
                Ok(self.state())
            })();
            if let Err(error) = &outcome {
                self.state.status = RecordingStatus::Error;
                if self.state.message.as_deref().is_none_or(str::is_empty) {
                    self.state.message = Some(error.to_string());
                }
                if self.platform_temporary_path.is_none() {
                    self.state.output_path = None;
                    self.state.started_at_ms = None;
                }
            }
            outcome
        }

        #[cfg(unix)]
        {
            let outcome = (|| -> Result<RecordingState, CaptureError> {
                let discovery = Command::new("dumpcap")
                .arg("-D")
                .output()
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        CaptureError::Recording(
                            "dumpcap is not installed. Install the free Wireshark CLI helper and grant capture permissions."
                                .into(),
                        )
                    } else {
                        CaptureError::Io(error)
                    }
                })?;
                if !discovery.status.success() {
                    let detail = String::from_utf8_lossy(&discovery.stderr).trim().to_owned();
                    self.state.status = RecordingStatus::Error;
                    self.state.message = Some(if detail.is_empty() {
                        "dumpcap cannot access a capture interface. Configure the distro capture group/capabilities."
                            .into()
                    } else {
                        detail
                    });
                    return Err(CaptureError::PermissionDenied(
                        self.state.message.clone().unwrap_or_default(),
                    ));
                }

                let size_kib = request.max_file_size_mb.saturating_mul(1_024);
                let size_argument = format!("filesize:{size_kib}");
                let duration_argument = request
                    .duration_limit_seconds
                    .map(|seconds| format!("duration:{seconds}"));
                let mut command = Command::new("dumpcap");
                command
                    .args(["-q", "-i", "any", "-w"])
                    .arg(&output)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                if request.rotate {
                    command.args(["-b", &size_argument, "-b", "files:4"]);
                } else {
                    command.args(["-a", &size_argument]);
                }
                if let Some(duration_argument) = duration_argument.as_deref() {
                    command.args(["-a", duration_argument]);
                }
                let child = command.spawn()?;
                self.platform_child = Some(child);
                self.platform_temporary_path = Some(output);
                self.state.status = RecordingStatus::Recording;
                self.state.message = Some("Recording locally with dumpcap.".into());
                Ok(self.state())
            })();
            if let Err(error) = &outcome {
                self.state.status = RecordingStatus::Error;
                if self.state.message.as_deref().is_none_or(str::is_empty) {
                    self.state.message = Some(error.to_string());
                }
                if self.platform_temporary_path.is_none() {
                    self.state.output_path = None;
                    self.state.started_at_ms = None;
                }
            }
            outcome
        }

        #[cfg(not(any(target_os = "windows", unix)))]
        Err(CaptureError::Unsupported)
    }

    pub fn stop(&mut self) -> Result<RecordingState, CaptureError> {
        let retrying_preserved_recording =
            self.state.status == RecordingStatus::Error && self.platform_temporary_path.is_some();
        if self.state.status != RecordingStatus::Recording && !retrying_preserved_recording {
            return Err(CaptureError::Recording("no recording is active".into()));
        }
        self.state.status = RecordingStatus::Stopping;

        #[cfg(target_os = "windows")]
        {
            let outcome = (|| -> Result<RecordingState, CaptureError> {
                if !self.platform_stopped {
                    let stop = Command::new("pktmon.exe").arg("stop").output()?;
                    if !stop.status.success() {
                        self.state.status = RecordingStatus::Error;
                        self.state.message =
                            Some(String::from_utf8_lossy(&stop.stderr).trim().to_owned());
                        return Err(CaptureError::Recording(
                            self.state.message.clone().unwrap_or_default(),
                        ));
                    }
                    self.platform_stopped = true;
                }
                let temporary = self.platform_temporary_path.as_ref().ok_or_else(|| {
                    CaptureError::Recording("temporary ETL path is missing".into())
                })?;
                let output = PathBuf::from(
                    self.state
                        .output_path
                        .as_deref()
                        .ok_or_else(|| CaptureError::Recording("output path is missing".into()))?,
                );
                let temporary_files = pktmon_files(temporary);
                if temporary_files.is_empty() {
                    self.state.status = RecordingStatus::Error;
                    self.state.message = Some("Pktmon produced no ETL recording files".into());
                    return Err(CaptureError::Recording(
                        self.state.message.clone().unwrap_or_default(),
                    ));
                }
                let output_files = temporary_files
                    .iter()
                    .map(|path| rotated_output_path(&output, pktmon_file_index(temporary, path)))
                    .collect::<Vec<_>>();
                if let Some(existing) = output_files.iter().find(|path| path.exists()) {
                    self.state.status = RecordingStatus::Error;
                    self.state.message = Some(format!(
                        "refusing to overwrite existing recording output: {}",
                        existing.display()
                    ));
                    return Err(CaptureError::Recording(
                        self.state.message.clone().unwrap_or_default(),
                    ));
                }
                for (temporary_file, output_file) in temporary_files.iter().zip(&output_files) {
                    let conversion = Command::new("pktmon.exe")
                        .arg("etl2pcap")
                        .arg(temporary_file)
                        .arg("--out")
                        .arg(output_file)
                        .output()?;
                    if !conversion.status.success() {
                        self.state.status = RecordingStatus::Error;
                        let detail = String::from_utf8_lossy(&conversion.stderr)
                            .trim()
                            .to_owned();
                        self.state.message = Some(if detail.is_empty() {
                            format!("Pktmon could not convert {}", temporary_file.display())
                        } else {
                            detail
                        });
                        return Err(CaptureError::Recording(
                            self.state.message.clone().unwrap_or_default(),
                        ));
                    }
                    // The source ETL is removed only after its PCAPNG conversion succeeds.
                    if let Err(error) = fs::remove_file(temporary_file) {
                        self.state.status = RecordingStatus::Error;
                        self.state.message = Some(format!(
                            "PCAPNG was created, but the temporary ETL could not be removed: {error}"
                        ));
                        return Err(CaptureError::Recording(
                            self.state.message.clone().unwrap_or_default(),
                        ));
                    }
                }
                self.platform_temporary_path = None;
                self.platform_stopped = false;
                self.windows_hard_stop_bytes = None;
                self.state.status = RecordingStatus::Idle;
                let completed_outputs = recording_output_files(&output);
                self.state.bytes_written = total_size(&completed_outputs);
                self.state.message = Some(if completed_outputs.len() == 1 {
                    "PCAPNG recording completed.".into()
                } else {
                    format!(
                        "PCAPNG recording completed in {} bounded parts.",
                        completed_outputs.len()
                    )
                });
                Ok(self.state())
            })();
            if let Err(error) = &outcome {
                self.state.status = RecordingStatus::Error;
                if self.state.message.as_deref().is_none_or(str::is_empty) {
                    self.state.message = Some(error.to_string());
                }
            }
            outcome
        }

        #[cfg(unix)]
        {
            let outcome = (|| -> Result<RecordingState, CaptureError> {
                if !self.platform_stopped {
                    let child = self.platform_child.as_mut().ok_or_else(|| {
                        CaptureError::Recording("dumpcap process is missing".into())
                    })?;
                    let status = if let Some(status) = child.try_wait()? {
                        status
                    } else {
                        let signal_result = unsafe { libc::kill(child.id() as i32, libc::SIGINT) };
                        if signal_result != 0 {
                            self.state.status = RecordingStatus::Error;
                            return Err(CaptureError::Io(std::io::Error::last_os_error()));
                        }
                        child.wait()?
                    };
                    self.platform_stopped = true;
                    if !status.success() {
                        self.state.status = RecordingStatus::Error;
                        return Err(CaptureError::Recording(format!(
                            "dumpcap exited with status {status}"
                        )));
                    }
                }
                self.platform_child = None;
                let output = self
                    .platform_temporary_path
                    .take()
                    .ok_or_else(|| CaptureError::Recording("recording path is missing".into()))?;
                self.platform_stopped = false;
                self.state.status = RecordingStatus::Idle;
                self.state.bytes_written = if self.rotation_enabled {
                    total_size(&dumpcap_files(&output))
                } else {
                    fs::metadata(output).map_or(0, |metadata| metadata.len())
                };
                self.state.message = Some("PCAPNG recording completed.".into());
                Ok(self.state())
            })();
            if let Err(error) = &outcome {
                self.state.status = RecordingStatus::Error;
                if self.state.message.as_deref().is_none_or(str::is_empty) {
                    self.state.message = Some(error.to_string());
                }
            }
            outcome
        }

        #[cfg(not(any(target_os = "windows", unix)))]
        Err(CaptureError::Unsupported)
    }
}

fn validate_output(request: &RecordingRequest) -> Result<PathBuf, CaptureError> {
    if !(1..=4_096).contains(&request.max_file_size_mb) {
        return Err(CaptureError::Recording(
            "maximum file size must be between 1 and 4096 MiB".into(),
        ));
    }
    if request
        .duration_limit_seconds
        .is_some_and(|seconds| seconds == 0 || seconds > 86_400)
    {
        return Err(CaptureError::Recording(
            "duration limit must be between 1 second and 24 hours".into(),
        ));
    }
    let output = PathBuf::from(request.output_path.trim());
    if output
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("pcapng"))
    {
        return Err(CaptureError::Recording(
            "recording output must use the .pcapng extension".into(),
        ));
    }
    let parent = output
        .parent()
        .ok_or_else(|| CaptureError::Recording("output path has no parent".into()))?;
    if !parent.is_dir() {
        return Err(CaptureError::Recording(
            "recording output directory does not exist".into(),
        ));
    }
    if output.exists() || !output_parts(&output).is_empty() {
        return Err(CaptureError::Recording(
            "recording output already exists; choose a new filename".into(),
        ));
    }
    #[cfg(unix)]
    if request.rotate && !dumpcap_files(&output).is_empty() {
        return Err(CaptureError::Recording(
            "rotated recording outputs with this filename already exist".into(),
        ));
    }
    reject_platform_temporary_collision(&output)?;
    Ok(output)
}

fn total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().fold(0_u64, |total, path| {
        total.saturating_add(fs::metadata(path).map_or(0, |metadata| metadata.len()))
    })
}

fn sibling_files(path: &Path, matches_stem: impl Fn(&str, &str) -> bool) -> Vec<PathBuf> {
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let Some(base_stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    let extension = path.extension().and_then(|value| value.to_str());
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate.is_file()
                && candidate
                    .extension()
                    .and_then(|value| value.to_str())
                    .zip(extension)
                    .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
                && candidate
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(|stem| matches_stem(base_stem, stem))
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        natural_suffix(left, base_stem).cmp(&natural_suffix(right, base_stem))
    });
    files
}

fn natural_suffix(path: &Path, base_stem: &str) -> u64 {
    path.file_stem()
        .and_then(|value| value.to_str())
        .and_then(|stem| stem.strip_prefix(base_stem))
        .map(|suffix| suffix.trim_matches(|character: char| !character.is_ascii_digit()))
        .and_then(|suffix| suffix.parse().ok())
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn pktmon_file_index(base: &Path, file: &Path) -> usize {
    let base_stem = base
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    usize::try_from(natural_suffix(file, base_stem)).unwrap_or(usize::MAX)
}

#[cfg(target_os = "windows")]
fn pktmon_files(path: &Path) -> Vec<PathBuf> {
    sibling_files(path, |base, stem| {
        stem == base
            || stem.strip_prefix(base).is_some_and(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|value| value.is_ascii_digit())
            })
    })
}

#[cfg(unix)]
fn dumpcap_files(path: &Path) -> Vec<PathBuf> {
    sibling_files(path, |base, stem| {
        stem == base || stem.starts_with(&format!("{base}_"))
    })
}

fn output_parts(path: &Path) -> Vec<PathBuf> {
    sibling_files(path, |base, stem| {
        stem.strip_prefix(&format!("{base}.part-"))
            .is_some_and(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|value| value.is_ascii_digit())
            })
    })
}

#[cfg(target_os = "windows")]
fn recording_output_files(path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if path.is_file() {
        paths.push(path.to_path_buf());
    }
    paths.extend(output_parts(path));
    paths
}

#[cfg(target_os = "windows")]
fn rotated_output_path(output: &Path, index: usize) -> PathBuf {
    if index == 0 {
        return output.to_path_buf();
    }
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("sabicheiro-capture");
    output.with_file_name(format!("{stem}.part-{:03}.pcapng", index + 1))
}

fn reject_platform_temporary_collision(output: &Path) -> Result<(), CaptureError> {
    if output.with_extension("etl") == output {
        return Err(CaptureError::Recording(
            "recording output conflicts with its temporary path".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_pcapng_paths_before_platform_action() {
        let request = RecordingRequest {
            output_path: "capture.exe".into(),
            max_file_size_mb: 50,
            rotate: false,
            duration_limit_seconds: None,
        };
        assert!(validate_output(&request).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn rotated_outputs_are_stable_and_preserve_the_requested_first_path() {
        let output = Path::new(r"C:\captures\sabicheiro.pcapng");
        assert_eq!(rotated_output_path(output, 0), output);
        assert_eq!(
            rotated_output_path(output, 1),
            PathBuf::from(r"C:\captures\sabicheiro.part-002.pcapng")
        );
    }
}
