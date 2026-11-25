use arrow_core::types::event::ArrowEvent;
use arrow_core::validation::pipeline::ValidationResult;
use serde::{Deserialize, Serialize};

/// Transport Model Part 2 — network-layer wrapper around an Arrow Event Object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportEnvelope {
    pub envelope_version: String,
    pub envelope_id: String,
    pub sender_id: String,
    pub receiver_id: ReceiverId,
    pub channel_binding: String,
    pub sent_at: String,
    pub nonce: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sequence: Option<i64>,
    /// Legacy signature field (unsigned pilot). Retained for backward compatibility.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub signature: Option<Signature>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub signing: Option<EnvelopeSigningMetadata>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<serde_json::Value>,
    pub payload: ArrowEvent,
}

/// `receiver_id` is "string | array<string>" per Transport Model Part 2.2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ReceiverId {
    One(String),
    Many(Vec<String>),
}

impl ReceiverId {
    pub fn contains(&self, node_id: &str) -> bool {
        match self {
            ReceiverId::One(id) => id == node_id,
            ReceiverId::Many(ids) => ids.iter().any(|id| id == node_id),
        }
    }

    pub fn primary(&self) -> Option<&str> {
        match self {
            ReceiverId::One(id) => Some(id.as_str()),
            ReceiverId::Many(ids) => ids.first().map(String::as_str),
        }
    }
}

/// Optional signature block (not used by http-rest binding).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub algorithm: String,
    pub key_ref: String,
    pub value: String,
}

/// Structured signing metadata for authenticated Arrow envelopes.
/// Mirrors `arrow_identity::types::SigningMetadata` but lives in the transport
/// layer to avoid a circular dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeSigningMetadata {
    /// Algorithm identifier (e.g. "Ed25519").
    pub alg: String,
    /// DID URL referencing the verification method.
    pub key_ref: String,
    /// Base64-encoded signature bytes (standard, no padding).
    pub sig_b64: String,
    /// ISO 8601 UTC timestamp of signature creation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created: Option<String>,
    /// Canonicalization scheme. Default: "arrow-canonical-json-v0".
    #[serde(default = "default_canonicalization")]
    pub canonicalization: String,
    /// What fields are signed. Default: "envelope-minus-signature-and-signing".
    #[serde(default = "default_signed_fields")]
    pub signed_fields: String,
}

fn default_canonicalization() -> String {
    "arrow-canonical-json-v0".to_string()
}

fn default_signed_fields() -> String {
    "envelope-minus-signature-and-signing".to_string()
}

/// Transport Model Part 4.2 — acknowledgment envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckEnvelope {
    pub ack_version: String,
    pub ack_id: String,
    pub ack_type: AckType,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ref_envelope_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ref_envelope_hash: Option<String>,
    pub sender_id: String,
    pub receiver_id: String,
    pub channel_binding: String,
    pub sent_at: String,
    pub result: AckResult,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub validation_result: Option<ValidationResult>,
    /// Transport warnings, e.g. {"warnings":["TPT-STALE-ENVELOPE"]}
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub details: Option<serde_json::Value>,
}

/// Transport Model Part 5.3 — transport-layer error envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub error_version: String,
    pub error_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ref_envelope_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ref_envelope_hash: Option<String>,
    pub sender_id: String,
    pub receiver_id: String,
    pub channel_binding: String,
    pub sent_at: String,
    pub error_code: String,
    pub error_message: String,
    #[serde(default)]
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AckType {
    Receipt,
    Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AckResult {
    Accepted,
    Rejected,
}
