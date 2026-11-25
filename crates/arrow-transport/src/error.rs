use crate::envelope::ErrorEnvelope;
use chrono::Utc;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug)]
pub enum ReplayStoreError {
    Open { path: String, message: String },
    Initialize { path: String, message: String },
    LockPoisoned,
    Io { source: String },
}

impl fmt::Display for ReplayStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplayStoreError::Open { path, message } => {
                write!(f, "failed to open replay store `{path}`: {message}")
            }
            ReplayStoreError::Initialize { path, message } => {
                write!(f, "failed to initialize replay store `{path}`: {message}")
            }
            ReplayStoreError::LockPoisoned => write!(f, "replay store lock poisoned"),
            ReplayStoreError::Io { source } => write!(f, "replay store I/O error: {source}"),
        }
    }
}

impl std::error::Error for ReplayStoreError {}

#[derive(Debug, Clone, Error)]
pub enum TransportError {
    #[error("{0}")]
    Malformed(String),
    #[error("{0}")]
    ReplayDetected(String),
    #[error("{0}")]
    ReplayStoreUnavailable(String),
    #[error("{0}")]
    UnknownSender(String),
    #[error("{0}")]
    VersionUnsupported(String),
    #[error("{0}")]
    Stale(String),
    #[error("{0}")]
    ChannelMismatch(String),
    #[error("{0}")]
    ReceiverMismatch(String),
    #[error("{0}")]
    SignatureMissing(String),
    #[error("{0}")]
    SignatureInvalid(String),
}

impl TransportError {
    /// Transport Model Part 5.2 error code string.
    pub fn code(&self) -> &'static str {
        match self {
            TransportError::Malformed(_) => "TPT-MALFORMED",
            TransportError::ReplayDetected(_) => "TPT-REPLAY-DETECTED",
            TransportError::ReplayStoreUnavailable(_) => "TPT-REPLAY-STORE-UNAVAILABLE",
            TransportError::UnknownSender(_) => "TPT-UNKNOWN-SENDER",
            TransportError::VersionUnsupported(_) => "TPT-VERSION-UNSUPPORTED",
            TransportError::Stale(_) => "TPT-STALE",
            TransportError::ChannelMismatch(_) => "TPT-CHANNEL-MISMATCH",
            TransportError::ReceiverMismatch(_) => "TPT-RECEIVER-MISMATCH",
            TransportError::SignatureMissing(_) => "TPT-SIGNATURE-MISSING",
            TransportError::SignatureInvalid(_) => "TPT-SIGNATURE-INVALID",
        }
    }

    /// HTTP status mapping (overlay Section 2.9).
    pub fn http_status(&self) -> u16 {
        match self {
            TransportError::Malformed(_) => 400,
            TransportError::VersionUnsupported(_) => 400,
            TransportError::Stale(_) => 400,
            TransportError::SignatureMissing(_) => 401,
            TransportError::SignatureInvalid(_) => 401,
            TransportError::UnknownSender(_) => 403,
            TransportError::ReceiverMismatch(_) => 403,
            TransportError::ReplayDetected(_) => 409,
            TransportError::ReplayStoreUnavailable(_) => 503,
            TransportError::ChannelMismatch(_) => 409,
        }
    }

    /// Build an ErrorEnvelope from this error (overlay Section 2.10).
    pub fn into_error_envelope(
        self,
        sender_id: String,
        receiver_id: String,
        channel_binding: String,
        ref_envelope_id: Option<String>,
        ref_envelope_hash: Option<String>,
    ) -> ErrorEnvelope {
        ErrorEnvelope {
            error_version: "1.0".to_string(),
            error_id: format!("err-{}", Uuid::new_v4()),
            ref_envelope_id,
            ref_envelope_hash,
            sender_id,
            receiver_id,
            channel_binding,
            sent_at: Utc::now().to_rfc3339(),
            error_code: self.code().to_string(),
            error_message: self.to_string(),
            details: serde_json::Value::Object(Default::default()),
        }
    }
}
