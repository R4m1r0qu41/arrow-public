use arrow_transport::{
    envelope::TransportEnvelope,
    error::TransportError,
    signature::{verify_envelope_signature, SignatureVerificationPolicy},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Whom a consumer is willing to act on, and with which algorithms.
#[derive(Debug, Clone)]
pub struct ProvenancePolicy {
    /// DIDs the consumer accepts as action-driving origins. Empty trusts no one.
    pub allowed_senders: Vec<String>,
    /// DID of this consumer. The envelope's `receiver_id` must include it.
    pub receiver_id: String,
    /// Accepted signature algorithms (default: `["Ed25519"]`).
    pub allowed_algorithms: Vec<String>,
}

impl ProvenancePolicy {
    pub fn new(allowed_senders: Vec<String>, receiver_id: impl Into<String>) -> Self {
        Self {
            allowed_senders,
            receiver_id: receiver_id.into(),
            allowed_algorithms: vec!["Ed25519".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceVerification {
    pub signer_did: String,
    pub key_ref: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub verified_at: String,
}

pub fn verify_envelope_provenance(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
) -> Result<ProvenanceVerification, TransportError> {
    if policy.allowed_senders.is_empty() {
        return Err(TransportError::UnknownSender(
            "no allowed senders configured; refusing to trust any provenance".to_string(),
        ));
    }
    if !policy
        .allowed_senders
        .iter()
        .any(|sender| sender == &envelope.sender_id)
    {
        return Err(TransportError::UnknownSender(format!(
            "sender `{}` is not in the provenance allow-list",
            envelope.sender_id
        )));
    }

    if policy.receiver_id.trim().is_empty() {
        return Err(TransportError::ReceiverMismatch(
            "no receiver_id configured; refusing to consume a bearer envelope".to_string(),
        ));
    }
    if !envelope.receiver_id.contains(&policy.receiver_id) {
        return Err(TransportError::ReceiverMismatch(format!(
            "receiver_id does not include this consumer `{}`",
            policy.receiver_id
        )));
    }

    let signature_policy = SignatureVerificationPolicy {
        require_signature: true,
        allowed_did_methods: Vec::new(),
        allowed_algorithms: policy.allowed_algorithms.clone(),
    };
    let verified = verify_envelope_signature(envelope, &signature_policy)?.ok_or_else(|| {
        TransportError::SignatureMissing(
            "envelope has no signing block; provenance requires a signature".to_string(),
        )
    })?;

    Ok(ProvenanceVerification {
        signer_did: verified.signer_did,
        key_ref: verified.key_ref,
        sender_id: envelope.sender_id.clone(),
        receiver_id: policy.receiver_id.clone(),
        verified_at: Utc::now().to_rfc3339(),
    })
}
