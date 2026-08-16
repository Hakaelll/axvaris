use crate::CaptureError;
use sabicheiro_core::{ExecutableIdentity, ExecutableSignatureStatus};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

const HASH_CHUNK_BYTES: usize = 64 * 1024;

/// Reads executable identity locally with an explicit I/O budget. The caller owns the
/// cancellation flag and never supplies an arbitrary webview-controlled path.
pub fn inspect_executable(
    path: &Path,
    process_instance: Option<String>,
    maximum_bytes: u64,
    cancelled: &AtomicBool,
) -> Result<ExecutableIdentity, CaptureError> {
    let metadata_before = fs::metadata(path)?;
    let modified_at_ms = modified_ms(&metadata_before);
    let revision_id = revision_id(path, metadata_before.len(), modified_at_ms);
    let mut limitation = None;
    let mut sha256 = None;
    let mut magic = [0_u8; 4];

    if metadata_before.len() > maximum_bytes.max(1) {
        limitation = Some(format!(
            "hash skipped: executable is {} bytes and the local I/O budget is {} bytes",
            metadata_before.len(),
            maximum_bytes.max(1)
        ));
    } else {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; HASH_CHUNK_BYTES];
        let mut total = 0_u64;
        loop {
            if cancelled.load(Ordering::Relaxed) {
                limitation = Some("hash cancelled before completion".into());
                break;
            }
            let read = file.read(&mut buffer)?;
            if read == 0 {
                sha256 = Some(format!("{:x}", hasher.finalize()));
                break;
            }
            if total == 0 {
                let copy_len = read.min(magic.len());
                magic[..copy_len].copy_from_slice(&buffer[..copy_len]);
            }
            total = total.saturating_add(read as u64);
            if total > maximum_bytes.max(1) {
                limitation = Some("executable changed beyond the I/O budget while hashing".into());
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }

    let metadata_after = fs::metadata(path)?;
    let after_modified = modified_ms(&metadata_after);
    if metadata_after.len() != metadata_before.len() || after_modified != modified_at_ms {
        sha256 = None;
        limitation = Some("executable changed while its local identity was being read".into());
    }

    Ok(ExecutableIdentity {
        process_instance,
        path: path.to_string_lossy().into_owned(),
        revision_id,
        size_bytes: metadata_before.len(),
        modified_at_ms,
        sha256,
        signature_status: executable_signature_status(path),
        package: local_package_identity(path),
        format: executable_format(path, &magic),
        computed_at_ms: sabicheiro_core::now_ms(),
        limitation,
    })
}

fn modified_ms(metadata: &fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
}

fn revision_id(path: &Path, size_bytes: u64, modified_at_ms: Option<i64>) -> String {
    format!(
        "file:{size_bytes}:{}:{}",
        modified_at_ms.unwrap_or_default(),
        path.to_string_lossy().to_ascii_lowercase()
    )
}

fn executable_format(path: &Path, magic: &[u8; 4]) -> Option<String> {
    if magic.starts_with(b"MZ") {
        Some("pe".into())
    } else if magic == b"\x7fELF" {
        Some("elf".into())
    } else {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
    }
}

#[cfg(target_os = "linux")]
fn local_package_identity(path: &Path) -> Option<String> {
    let path_text = path.to_string_lossy();
    let output = std::process::Command::new("dpkg-query")
        .arg("--search")
        .arg(path_text.as_ref())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .next()?
        .split_once(':')
        .map(|(package, _)| package.trim().to_owned())
        .filter(|package| !package.is_empty())
}

#[cfg(not(target_os = "linux"))]
fn local_package_identity(_path: &Path) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
fn executable_signature_status(path: &Path) -> ExecutableSignatureStatus {
    use std::{mem::size_of, os::windows::ffi::OsStrExt, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::TRUST_E_NOSIGNATURE,
        Security::WinTrust::{
            WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO,
            WTD_CACHE_ONLY_URL_RETRIEVAL, WTD_CHOICE_FILE, WTD_REVOCATION_CHECK_NONE,
            WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
            WinVerifyTrust,
        },
    };

    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide_path.as_ptr(),
        hFile: null_mut(),
        pgKnownSubject: null_mut(),
    };
    let mut data = WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL | WTD_REVOCATION_CHECK_NONE,
        ..WINTRUST_DATA::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    // SAFETY: all pointers reference initialized values that outlive both calls. URL
    // retrieval is cache-only, and the state handle is explicitly closed.
    let status = unsafe {
        WinVerifyTrust(
            null_mut(),
            &mut action,
            (&mut data as *mut WINTRUST_DATA).cast(),
        )
    };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe {
        WinVerifyTrust(
            null_mut(),
            &mut action,
            (&mut data as *mut WINTRUST_DATA).cast(),
        );
    }
    if status == 0 {
        ExecutableSignatureStatus::Valid
    } else if status == TRUST_E_NOSIGNATURE {
        ExecutableSignatureStatus::Unsigned
    } else {
        ExecutableSignatureStatus::Invalid
    }
}

#[cfg(not(target_os = "windows"))]
fn executable_signature_status(_path: &Path) -> ExecutableSignatureStatus {
    ExecutableSignatureStatus::Unsupported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashing_is_budgeted_and_invalidated_by_cancellation() {
        let directory = std::env::temp_dir().join(format!(
            "sabicheiro-executable-test-{}-{}",
            std::process::id(),
            sabicheiro_core::now_ms()
        ));
        fs::create_dir_all(&directory).expect("test directory");
        let path = directory.join("fixture.bin");
        fs::write(&path, b"MZlocal-fixture").expect("fixture");
        let identity = inspect_executable(
            &path,
            Some("fixture".into()),
            1_024,
            &AtomicBool::new(false),
        )
        .expect("identity");
        assert_eq!(identity.format.as_deref(), Some("pe"));
        assert_eq!(identity.sha256.as_deref().map(str::len), Some(64));

        let cancelled = AtomicBool::new(true);
        let cancelled_identity =
            inspect_executable(&path, None, 1_024, &cancelled).expect("cancelled identity");
        assert!(cancelled_identity.sha256.is_none());
        assert!(cancelled_identity.limitation.is_some());
        fs::remove_file(&path).expect("remove fixture");
        fs::remove_dir(&directory).expect("remove directory");
    }
}
