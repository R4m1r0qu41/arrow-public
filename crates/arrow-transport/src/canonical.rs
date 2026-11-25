use crate::{envelope::TransportEnvelope, error::TransportError};
use sha2::{Digest, Sha256};

/// Transport Model Part 2.4 — SHA-256 over the RFC 8785 / JCS canonical serialization
/// of the envelope with the `signature` and `signing` fields removed.
/// Returns a lowercase "0x"-prefixed hex string.
pub fn envelope_hash(envelope: &TransportEnvelope) -> Result<String, TransportError> {
    let canonical = envelope_canonical_bytes(envelope)?;
    let digest = Sha256::digest(&canonical);
    Ok(format!("0x{}", hex::encode(digest)))
}

/// Returns the raw JCS-canonical bytes of the envelope with `signature` and `signing`
/// fields stripped. These are the bytes that get signed / verified.
pub fn envelope_canonical_bytes(envelope: &TransportEnvelope) -> Result<Vec<u8>, TransportError> {
    let mut value = serde_json::to_value(envelope)
        .map_err(|e| TransportError::Malformed(format!("envelope serialization failed: {e}")))?;

    if let Some(obj) = value.as_object_mut() {
        obj.remove("signature");
        obj.remove("signing");
    }

    serde_jcs::to_vec(&value)
        .map_err(|e| TransportError::Malformed(format!("JCS canonicalization failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::ReceiverId;
    use arrow_core::types::event::ArrowEvent;

    fn minimal_event() -> ArrowEvent {
        serde_json::from_str(include_str!("../tests/fixtures/valid-event.json")).unwrap()
    }

    fn base_envelope() -> TransportEnvelope {
        TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: "env-test-001".to_string(),
            sender_id: "did:arrow:pilot:node-a".to_string(),
            receiver_id: ReceiverId::One("did:arrow:pilot:node-b".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-05-21T12:00:00Z".to_string(),
            nonce: "n-aabbccddeeff00112233445566778899".to_string(),
            sequence: None,
            signature: None,
            signing: None,
            correlation_id: None,
            reply_to: None,
            metadata: None,
            payload: minimal_event(),
        }
    }

    #[test]
    fn hash_is_deterministic() {
        let e = base_envelope();
        let h1 = envelope_hash(&e).unwrap();
        let h2 = envelope_hash(&e).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_excludes_signature() {
        use crate::envelope::Signature;
        let mut e_signed = base_envelope();
        e_signed.signature = Some(Signature {
            algorithm: "Ed25519".to_string(),
            key_ref: "did:arrow:pilot:node-a#key-1".to_string(),
            value: "MEUCIQ==".to_string(),
        });
        let h_signed = envelope_hash(&e_signed).unwrap();
        let h_plain = envelope_hash(&base_envelope()).unwrap();
        assert_eq!(
            h_signed, h_plain,
            "signature field must not affect the hash"
        );
    }

    #[test]
    fn hash_starts_with_0x() {
        let h = envelope_hash(&base_envelope()).unwrap();
        assert!(h.starts_with("0x"), "hash must start with 0x, got: {h}");
        assert_eq!(h.len(), 2 + 64, "hash must be 0x + 64 hex chars");
    }
}
