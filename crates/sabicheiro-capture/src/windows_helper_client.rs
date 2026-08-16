use crate::{
    HELPER_MAX_FRAME_BYTES, HELPER_PIPE_NAME, HELPER_PROTOCOL_VERSION, HelperEnvelope, HelperNonce,
    HelperOperation, HelperResponse, decode_helper_response_frame, encode_helper_frame,
};
use std::{io, ptr, thread, time::Duration};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE},
    Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom},
    Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING, ReadFile, WriteFile},
};

pub struct WindowsHelperClient {
    handle: HANDLE,
    nonce: HelperNonce,
    client_pid: u32,
    sequence: u64,
}

impl WindowsHelperClient {
    pub fn connect(nonce: HelperNonce) -> io::Result<Self> {
        let name = wide(HELPER_PIPE_NAME);
        let mut last_error = None;
        for _ in 0..100 {
            let handle = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    ptr::null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    ptr::null_mut(),
                )
            };
            if handle != INVALID_HANDLE_VALUE {
                return Ok(Self {
                    handle,
                    nonce,
                    client_pid: std::process::id(),
                    sequence: 0,
                });
            }
            last_error = Some(io::Error::last_os_error());
            thread::sleep(Duration::from_millis(20));
        }
        Err(last_error.unwrap_or_else(|| io::Error::other("helper pipe unavailable")))
    }

    pub fn authenticate(&mut self) -> io::Result<HelperResponse> {
        self.request(HelperOperation::Authenticate)
    }

    pub fn request(&mut self, request: HelperOperation) -> io::Result<HelperResponse> {
        self.sequence = self.sequence.saturating_add(1);
        let envelope = HelperEnvelope {
            protocol_version: HELPER_PROTOCOL_VERSION,
            request_id: format!("native-{}", self.sequence),
            client_pid: self.client_pid,
            nonce: self.nonce,
            request,
        };
        let frame = encode_helper_frame(&envelope)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        write_all(self.handle, &frame)?;
        let frame = read_frame(self.handle)?;
        let (response, consumed) = decode_helper_response_frame(&frame)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if consumed != frame.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "helper response contains trailing data",
            ));
        }
        Ok(response)
    }
}

impl Drop for WindowsHelperClient {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

pub fn secure_helper_nonce() -> io::Result<HelperNonce> {
    let mut nonce = [0_u8; 32];
    let status = unsafe {
        BCryptGenRandom(
            ptr::null_mut(),
            nonce.as_mut_ptr(),
            nonce.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        Err(io::Error::other(format!(
            "BCryptGenRandom failed with NTSTATUS 0x{:08x}",
            status as u32
        )))
    } else {
        let nonce = HelperNonce(nonce);
        if nonce.is_zero() {
            Err(io::Error::other("system RNG returned a zero nonce"))
        } else {
            Ok(nonce)
        }
    }
}

fn read_frame(handle: HANDLE) -> io::Result<Vec<u8>> {
    let mut header = [0_u8; 4];
    read_exact(handle, &mut header)?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > HELPER_MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "helper response length is outside the allowed range",
        ));
    }
    let mut frame = Vec::with_capacity(length + 4);
    frame.extend_from_slice(&header);
    frame.resize(length + 4, 0);
    read_exact(handle, &mut frame[4..])?;
    Ok(frame)
}

fn read_exact(handle: HANDLE, mut destination: &mut [u8]) -> io::Result<()> {
    while !destination.is_empty() {
        let mut read = 0_u32;
        if unsafe {
            ReadFile(
                handle,
                destination.as_mut_ptr(),
                destination.len().min(u32::MAX as usize) as u32,
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
        let mut written = 0_u32;
        if unsafe {
            WriteFile(
                handle,
                source.as_ptr(),
                source.len().min(u32::MAX as usize) as u32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_nonce_is_non_zero_and_changes() {
        let first = secure_helper_nonce().expect("first nonce");
        let second = secure_helper_nonce().expect("second nonce");
        assert!(!first.is_zero());
        assert!(!second.is_zero());
        assert_ne!(first, second);
    }

    #[test]
    fn pipe_binds_the_native_client_and_roundtrips_closed_operations() {
        let nonce = secure_helper_nonce().expect("nonce");
        let pid = std::process::id();
        let server = thread::spawn(move || crate::run_windows_helper(nonce, pid));
        let mut client = WindowsHelperClient::connect(nonce).expect("connect helper");
        assert!(matches!(
            client.authenticate().expect("authenticate"),
            HelperResponse::Ok { .. }
        ));
        assert!(matches!(
            client.request(HelperOperation::Ping).expect("ping"),
            HelperResponse::Ok { detail, .. } if detail == "pong"
        ));
        drop(client);
        server
            .join()
            .expect("server thread")
            .expect("server result");
    }
}
