use crate::{
    HELPER_MAX_FRAME_BYTES, HelperDnsEvent, HelperNonce, HelperOperation, HelperResponse,
    HelperSession, WindowsEventCapture, decode_helper_frame, encode_helper_frame,
};
use std::{
    collections::VecDeque,
    io,
    mem::size_of,
    ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_PIPE_CONNECTED, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
    },
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{FlushFileBuffers, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile},
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
        PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    },
};

pub const HELPER_PIPE_NAME: &str = r"\\.\pipe\sabicheiro-capture-v1";
const PIPE_BUFFER_BYTES: u32 = HELPER_MAX_FRAME_BYTES as u32 + 4;
const PIPE_TIMEOUT_MS: u32 = 2_000;
const PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;OW)";

pub fn run_windows_helper(expected_nonce: HelperNonce, expected_client_pid: u32) -> io::Result<()> {
    let pipe = create_restricted_pipe()?;
    pipe.connect()?;
    let actual_client_pid = pipe.client_pid()?;
    let mut session = HelperSession::new(expected_nonce, expected_client_pid)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut event_capture = None;
    let mut event_queue = VecDeque::new();
    let mut dns_queue = VecDeque::new();
    let mut stream_started = None;
    let mut stream_limit = Duration::ZERO;

    loop {
        if stream_started.is_some_and(|started: Instant| started.elapsed() >= stream_limit) {
            event_capture = None;
            stream_started = None;
        }
        let frame = match pipe.read_frame() {
            Ok(frame) => frame,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::UnexpectedEof
                        | io::ErrorKind::BrokenPipe
                        | io::ErrorKind::ConnectionReset
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let (envelope, consumed) = match decode_helper_frame(&frame) {
            Ok(decoded) => decoded,
            Err(error) => {
                pipe.write_response(&HelperResponse::Rejected {
                    request_id: None,
                    code: "invalid_frame".into(),
                    detail: error.to_string(),
                })?;
                continue;
            }
        };
        if consumed != frame.len() {
            pipe.write_response(&HelperResponse::Rejected {
                request_id: Some(envelope.request_id),
                code: "trailing_frame_data".into(),
                detail: "exactly one frame is accepted per request".into(),
            })?;
            continue;
        }
        if let Err(error) =
            session.authorize(&envelope, actual_client_pid, sabicheiro_core::now_ms())
        {
            pipe.write_response(&HelperResponse::Rejected {
                request_id: Some(envelope.request_id),
                code: helper_error_code(&error).into(),
                detail: error.to_string(),
            })?;
            continue;
        }
        let request_id = envelope.request_id;
        match envelope.request {
            HelperOperation::Authenticate => pipe.write_response(&HelperResponse::Ok {
                request_id,
                detail: "authenticated native client".into(),
            })?,
            HelperOperation::GetCapabilities => pipe.write_response(&HelperResponse::Ok {
                request_id,
                detail: "windows_etw_network,windows_etw_dns,metadata_only".into(),
            })?,
            HelperOperation::StartEventStream {
                include_dns: _,
                include_handshake_metadata,
                duration_limit_seconds,
            } => {
                if include_handshake_metadata {
                    pipe.write_response(&HelperResponse::Rejected {
                        request_id: Some(request_id),
                        code: "handshake_metadata_unavailable".into(),
                        detail: "this helper build does not inspect packet payload".into(),
                    })?;
                    continue;
                }
                match WindowsEventCapture::start() {
                    Ok(capture) => {
                        event_capture = Some(capture);
                        event_queue.clear();
                        dns_queue.clear();
                        stream_started = Some(Instant::now());
                        stream_limit = Duration::from_secs(u64::from(duration_limit_seconds));
                        pipe.write_response(&HelperResponse::Ok {
                            request_id,
                            detail: "event stream started".into(),
                        })?;
                    }
                    Err(error) => pipe.write_response(&HelperResponse::Rejected {
                        request_id: Some(request_id),
                        code: "provider_start_failed".into(),
                        detail: error.to_string(),
                    })?,
                }
            }
            HelperOperation::StopEventStream => {
                event_capture = None;
                event_queue.clear();
                dns_queue.clear();
                stream_started = None;
                pipe.write_response(&HelperResponse::Ok {
                    request_id,
                    detail: "event stream stopped".into(),
                })?;
            }
            HelperOperation::DrainEvents { max_events } => {
                let Some(capture) = event_capture.as_mut() else {
                    pipe.write_response(&HelperResponse::Rejected {
                        request_id: Some(request_id),
                        code: "stream_not_running".into(),
                        detail: "start the event stream before draining it".into(),
                    })?;
                    continue;
                };
                let (events, dns) = capture.drain(sabicheiro_core::now_ms());
                event_queue.extend(events);
                dns_queue.extend(dns.into_iter().map(|observation| HelperDnsEvent {
                    address: observation.address,
                    hostname: observation.hostname,
                    expires_at_ms: observation.expires_at_ms,
                }));
                let maximum = usize::from(max_events);
                let events = event_queue
                    .drain(..event_queue.len().min(maximum))
                    .collect();
                let dns = dns_queue.drain(..dns_queue.len().min(maximum)).collect();
                pipe.write_response(&HelperResponse::EventBatch {
                    request_id,
                    events,
                    dns,
                    dropped_events: capture.dropped(),
                })?;
            }
            HelperOperation::GetStatus => {
                let detail = event_capture.as_ref().map_or_else(
                    || "stopped".to_string(),
                    |capture| {
                        format!(
                            "running,pending={},capacity={},dropped={}",
                            capture.pending(),
                            capture.capacity(),
                            capture.dropped()
                        )
                    },
                );
                pipe.write_response(&HelperResponse::Ok { request_id, detail })?;
            }
            HelperOperation::Ping => pipe.write_response(&HelperResponse::Ok {
                request_id,
                detail: "pong".into(),
            })?,
        }
    }
}

fn helper_error_code(error: &crate::HelperError) -> &'static str {
    match error {
        crate::HelperError::UnsupportedVersion(_) => "unsupported_version",
        crate::HelperError::InvalidRequestId => "invalid_request_id",
        crate::HelperError::InvalidClientPid | crate::HelperError::ClientIdentityMismatch => {
            "client_identity_mismatch"
        }
        crate::HelperError::InvalidNonce => "invalid_nonce",
        crate::HelperError::AuthenticationRequired => "authentication_required",
        crate::HelperError::RateLimited => "rate_limited",
        crate::HelperError::InvalidDuration | crate::HelperError::InvalidBatchSize => {
            "invalid_limit"
        }
        crate::HelperError::IncompleteFrame
        | crate::HelperError::EmptyFrame
        | crate::HelperError::FrameTooLarge
        | crate::HelperError::InvalidJson(_) => "invalid_frame",
    }
}

struct RestrictedPipe(HANDLE);

impl RestrictedPipe {
    fn connect(&self) -> io::Result<()> {
        let connected = unsafe { ConnectNamedPipe(self.0, ptr::null_mut()) };
        if connected != 0 || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn client_pid(&self) -> io::Result<u32> {
        let mut pid = 0_u32;
        if unsafe { GetNamedPipeClientProcessId(self.0, &mut pid) } == 0 || pid == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(pid)
        }
    }

    fn read_frame(&self) -> io::Result<Vec<u8>> {
        let mut header = [0_u8; 4];
        read_exact(self.0, &mut header)?;
        let length = u32::from_le_bytes(header) as usize;
        if length == 0 || length > HELPER_MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "helper frame length is outside the allowed range",
            ));
        }
        let mut frame = Vec::with_capacity(length + 4);
        frame.extend_from_slice(&header);
        frame.resize(length + 4, 0);
        read_exact(self.0, &mut frame[4..])?;
        Ok(frame)
    }

    fn write_response(&self, response: &HelperResponse) -> io::Result<()> {
        let frame = encode_helper_frame(response)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        write_all(self.0, &frame)?;
        if unsafe { FlushFileBuffers(self.0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for RestrictedPipe {
    fn drop(&mut self) {
        unsafe {
            DisconnectNamedPipe(self.0);
            CloseHandle(self.0);
        }
    }
}

fn create_restricted_pipe() -> io::Result<RestrictedPipe> {
    let sddl = wide(PIPE_SDDL);
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let name = wide(HELPER_PIPE_NAME);
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            PIPE_BUFFER_BYTES,
            PIPE_BUFFER_BYTES,
            PIPE_TIMEOUT_MS,
            &attributes,
        )
    };
    unsafe {
        LocalFree(descriptor);
    }
    if handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(RestrictedPipe(handle))
    }
}

fn read_exact(handle: HANDLE, mut destination: &mut [u8]) -> io::Result<()> {
    while !destination.is_empty() {
        let request = destination.len().min(u32::MAX as usize) as u32;
        let mut read = 0_u32;
        if unsafe {
            ReadFile(
                handle,
                destination.as_mut_ptr(),
                request,
                &mut read,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "helper pipe closed",
            ));
        }
        destination = &mut destination[read as usize..];
    }
    Ok(())
}

fn write_all(handle: HANDLE, mut source: &[u8]) -> io::Result<()> {
    while !source.is_empty() {
        let request = source.len().min(u32::MAX as usize) as u32;
        let mut written = 0_u32;
        if unsafe {
            WriteFile(
                handle,
                source.as_ptr(),
                request,
                &mut written,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "helper pipe wrote zero bytes",
            ));
        }
        source = &source[written as usize..];
    }
    Ok(())
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}
