use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::net::IpAddr;

pub const HELPER_PROTOCOL_VERSION: u16 = 1;
pub const HELPER_MAX_FRAME_BYTES: usize = 64 * 1024;
const MAX_REQUEST_ID_BYTES: usize = 64;
const MAX_REQUESTS_PER_SECOND: u32 = 120;
const MAX_CAPTURE_DURATION_SECONDS: u32 = 60 * 60;
pub const HELPER_MAX_BATCH_EVENTS: u16 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HelperNonce(pub [u8; 32]);

impl HelperNonce {
    pub const fn is_zero(self) -> bool {
        let mut index = 0;
        while index < self.0.len() {
            if self.0[index] != 0 {
                return false;
            }
            index += 1;
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum HelperOperation {
    Authenticate,
    GetCapabilities,
    StartEventStream {
        include_dns: bool,
        include_handshake_metadata: bool,
        duration_limit_seconds: u32,
    },
    StopEventStream,
    DrainEvents {
        max_events: u16,
    },
    GetStatus,
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelperEnvelope {
    pub protocol_version: u16,
    pub request_id: String,
    pub client_pid: u32,
    pub nonce: HelperNonce,
    pub request: HelperOperation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum HelperResponse {
    Ok {
        request_id: String,
        detail: String,
    },
    Rejected {
        request_id: Option<String>,
        code: String,
        detail: String,
    },
    EventBatch {
        request_id: String,
        events: Vec<sabicheiro_core::NormalizedNetworkEvent>,
        dns: Vec<HelperDnsEvent>,
        dropped_events: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperDnsEvent {
    pub address: IpAddr,
    pub hostname: String,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperError {
    IncompleteFrame,
    EmptyFrame,
    FrameTooLarge,
    InvalidJson(String),
    UnsupportedVersion(u16),
    InvalidRequestId,
    InvalidClientPid,
    InvalidNonce,
    AuthenticationRequired,
    ClientIdentityMismatch,
    RateLimited,
    InvalidDuration,
    InvalidBatchSize,
}

impl std::fmt::Display for HelperError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompleteFrame => formatter.write_str("incomplete helper frame"),
            Self::EmptyFrame => formatter.write_str("helper frame cannot be empty"),
            Self::FrameTooLarge => formatter.write_str("helper frame exceeds the size limit"),
            Self::InvalidJson(error) => write!(formatter, "invalid helper JSON: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported helper protocol version {version}")
            }
            Self::InvalidRequestId => formatter.write_str("invalid helper request id"),
            Self::InvalidClientPid => formatter.write_str("invalid helper client PID"),
            Self::InvalidNonce => formatter.write_str("invalid helper session nonce"),
            Self::AuthenticationRequired => formatter.write_str("helper authentication required"),
            Self::ClientIdentityMismatch => formatter.write_str("helper client identity mismatch"),
            Self::RateLimited => formatter.write_str("helper request rate exceeded"),
            Self::InvalidDuration => formatter.write_str("invalid helper capture duration"),
            Self::InvalidBatchSize => formatter.write_str("invalid helper event batch size"),
        }
    }
}

impl std::error::Error for HelperError {}

pub fn encode_helper_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, HelperError> {
    let payload =
        serde_json::to_vec(value).map_err(|error| HelperError::InvalidJson(error.to_string()))?;
    if payload.is_empty() {
        return Err(HelperError::EmptyFrame);
    }
    if payload.len() > HELPER_MAX_FRAME_BYTES {
        return Err(HelperError::FrameTooLarge);
    }
    let length = u32::try_from(payload.len()).map_err(|_| HelperError::FrameTooLarge)?;
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_helper_frame(frame: &[u8]) -> Result<(HelperEnvelope, usize), HelperError> {
    decode_typed_frame(frame)
}

pub fn decode_helper_response_frame(frame: &[u8]) -> Result<(HelperResponse, usize), HelperError> {
    decode_typed_frame(frame)
}

fn decode_typed_frame<T: DeserializeOwned>(frame: &[u8]) -> Result<(T, usize), HelperError> {
    let header = frame.get(..4).ok_or(HelperError::IncompleteFrame)?;
    let length = u32::from_le_bytes(header.try_into().expect("four-byte frame header")) as usize;
    if length == 0 {
        return Err(HelperError::EmptyFrame);
    }
    if length > HELPER_MAX_FRAME_BYTES {
        return Err(HelperError::FrameTooLarge);
    }
    let consumed = length.checked_add(4).ok_or(HelperError::FrameTooLarge)?;
    let payload = frame.get(4..consumed).ok_or(HelperError::IncompleteFrame)?;
    let value = serde_json::from_slice(payload)
        .map_err(|error| HelperError::InvalidJson(error.to_string()))?;
    Ok((value, consumed))
}

/// Per-pipe authorization state. OS pipe ACL and client PID checks are performed by
/// the transport; this layer still requires a fresh nonce and enforces request limits.
pub struct HelperSession {
    expected_nonce: HelperNonce,
    expected_client_pid: u32,
    authenticated: bool,
    rate_window_second: i64,
    requests_in_window: u32,
}

impl HelperSession {
    pub fn new(expected_nonce: HelperNonce, expected_client_pid: u32) -> Result<Self, HelperError> {
        if expected_nonce.is_zero() {
            return Err(HelperError::InvalidNonce);
        }
        if expected_client_pid == 0 {
            return Err(HelperError::InvalidClientPid);
        }
        Ok(Self {
            expected_nonce,
            expected_client_pid,
            authenticated: false,
            rate_window_second: i64::MIN,
            requests_in_window: 0,
        })
    }

    pub const fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    pub fn authorize(
        &mut self,
        envelope: &HelperEnvelope,
        actual_client_pid: u32,
        now_ms: i64,
    ) -> Result<(), HelperError> {
        validate_envelope(envelope)?;
        if actual_client_pid == 0
            || actual_client_pid != self.expected_client_pid
            || envelope.client_pid != actual_client_pid
        {
            return Err(HelperError::ClientIdentityMismatch);
        }
        if envelope.nonce != self.expected_nonce {
            return Err(HelperError::InvalidNonce);
        }
        let second = now_ms.div_euclid(1_000);
        if second != self.rate_window_second {
            self.rate_window_second = second;
            self.requests_in_window = 0;
        }
        self.requests_in_window = self.requests_in_window.saturating_add(1);
        if self.requests_in_window > MAX_REQUESTS_PER_SECOND {
            return Err(HelperError::RateLimited);
        }
        if !self.authenticated {
            if envelope.request != HelperOperation::Authenticate {
                return Err(HelperError::AuthenticationRequired);
            }
            self.authenticated = true;
            return Ok(());
        }
        if let HelperOperation::StartEventStream {
            duration_limit_seconds,
            ..
        } = envelope.request
            && !(1..=MAX_CAPTURE_DURATION_SECONDS).contains(&duration_limit_seconds)
        {
            return Err(HelperError::InvalidDuration);
        }
        if let HelperOperation::DrainEvents { max_events } = envelope.request
            && !(1..=HELPER_MAX_BATCH_EVENTS).contains(&max_events)
        {
            return Err(HelperError::InvalidBatchSize);
        }
        Ok(())
    }
}

fn validate_envelope(envelope: &HelperEnvelope) -> Result<(), HelperError> {
    if envelope.protocol_version != HELPER_PROTOCOL_VERSION {
        return Err(HelperError::UnsupportedVersion(envelope.protocol_version));
    }
    if envelope.request_id.trim().is_empty()
        || envelope.request_id.len() > MAX_REQUEST_ID_BYTES
        || !envelope
            .request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(HelperError::InvalidRequestId);
    }
    if envelope.client_pid == 0 {
        return Err(HelperError::InvalidClientPid);
    }
    if envelope.nonce.is_zero() {
        return Err(HelperError::InvalidNonce);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nonce() -> HelperNonce {
        HelperNonce([0x5a; 32])
    }

    fn envelope(request: HelperOperation) -> HelperEnvelope {
        HelperEnvelope {
            protocol_version: HELPER_PROTOCOL_VERSION,
            request_id: "request-1".into(),
            client_pid: 42,
            nonce: nonce(),
            request,
        }
    }

    #[test]
    fn frame_roundtrip_is_bounded_and_rejects_unknown_fields() {
        let expected = envelope(HelperOperation::Authenticate);
        let frame = encode_helper_frame(&expected).expect("encode frame");
        let (actual, consumed) = decode_helper_frame(&frame).expect("decode frame");
        assert_eq!(actual, expected);
        assert_eq!(consumed, frame.len());

        let unknown = br#"{"protocol_version":1,"request_id":"x","client_pid":42,"nonce":[90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90],"request":{"operation":"ping"},"shell":"cmd.exe"}"#;
        let mut hostile = Vec::new();
        hostile.extend_from_slice(&(unknown.len() as u32).to_le_bytes());
        hostile.extend_from_slice(unknown);
        assert!(matches!(
            decode_helper_frame(&hostile),
            Err(HelperError::InvalidJson(_))
        ));
    }

    #[test]
    fn authentication_binds_nonce_and_native_client_pid() {
        let mut session = HelperSession::new(nonce(), 42).expect("session");
        assert_eq!(
            session.authorize(&envelope(HelperOperation::GetStatus), 42, 1_000),
            Err(HelperError::AuthenticationRequired)
        );
        assert!(
            session
                .authorize(&envelope(HelperOperation::Authenticate), 42, 1_001)
                .is_ok()
        );
        assert!(session.is_authenticated());

        let mut wrong_nonce = envelope(HelperOperation::GetStatus);
        wrong_nonce.nonce = HelperNonce([0xa5; 32]);
        assert_eq!(
            session.authorize(&wrong_nonce, 42, 1_002),
            Err(HelperError::InvalidNonce)
        );
        assert_eq!(
            session.authorize(&envelope(HelperOperation::GetStatus), 7, 1_003),
            Err(HelperError::ClientIdentityMismatch)
        );
    }

    #[test]
    fn rate_duration_and_version_limits_are_enforced() {
        let mut session = HelperSession::new(nonce(), 42).expect("session");
        session
            .authorize(&envelope(HelperOperation::Authenticate), 42, 1_000)
            .expect("authenticate");
        let invalid_duration = envelope(HelperOperation::StartEventStream {
            include_dns: false,
            include_handshake_metadata: false,
            duration_limit_seconds: 0,
        });
        assert_eq!(
            session.authorize(&invalid_duration, 42, 1_001),
            Err(HelperError::InvalidDuration)
        );
        assert_eq!(
            session.authorize(
                &envelope(HelperOperation::DrainEvents {
                    max_events: HELPER_MAX_BATCH_EVENTS + 1,
                }),
                42,
                1_001,
            ),
            Err(HelperError::InvalidBatchSize)
        );
        let mut incompatible = envelope(HelperOperation::GetStatus);
        incompatible.protocol_version = HELPER_PROTOCOL_VERSION + 1;
        assert_eq!(
            session.authorize(&incompatible, 42, 1_002),
            Err(HelperError::UnsupportedVersion(2))
        );
        // Authentication and structurally valid rejected operations consume the same
        // defensive budget as successful operations. Unsupported protocol versions
        // are rejected before they reach the authenticated session budget.
        for index in 0..(MAX_REQUESTS_PER_SECOND - 3) {
            session
                .authorize(
                    &envelope(HelperOperation::Ping),
                    42,
                    1_010 + i64::from(index),
                )
                .expect("within rate limit");
        }
        assert_eq!(
            session.authorize(&envelope(HelperOperation::Ping), 42, 1_999),
            Err(HelperError::RateLimited)
        );
    }

    #[test]
    fn arbitrary_bytes_never_escape_the_parser_or_allocate_unbounded_frames() {
        let mut state = 0x1234_5678_u64;
        for length in 0..2_048 {
            let mut bytes = vec![0_u8; length];
            for byte in &mut bytes {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state as u8;
            }
            let result = std::panic::catch_unwind(|| decode_helper_frame(&bytes));
            assert!(result.is_ok(), "parser panicked for {length} bytes");
        }
        let mut oversized = vec![0_u8; 4];
        oversized.copy_from_slice(&((HELPER_MAX_FRAME_BYTES + 1) as u32).to_le_bytes());
        assert_eq!(
            decode_helper_frame(&oversized),
            Err(HelperError::FrameTooLarge)
        );
    }
}
