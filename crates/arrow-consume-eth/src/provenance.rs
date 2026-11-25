//! Backward-compatible re-export of shared adapter provenance verification.

pub use arrow_adapter::provenance::{
    verify_envelope_provenance as verify_envelope, ProvenancePolicy, ProvenanceVerification,
};

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::types::event::ArrowEvent;
    use arrow_identity::crypto::ed25519::{generate_test_keypair, sign_ed25519};
    use arrow_identity::resolver::did_key::did_key_from_public;
    use arrow_transport::{
        canonical::envelope_canonical_bytes,
        envelope::{EnvelopeSigningMetadata, ReceiverId, TransportEnvelope},
        signature::{CANONICALIZATION, SIGNED_FIELDS},
    };
    use base64::Engine;

    /// Build a signed envelope from a fresh did:key keypair; returns (envelope, did).
    fn signed_envelope() -> (TransportEnvelope, String) {
        let (sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let suffix = did.strip_prefix("did:key:").unwrap();
        let key_ref = format!("{did}#{suffix}");

        let event = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            event_id: "evt-1".to_string(),
            lifecycle_id: "life-1".to_string(),
            timestamp: "2026-05-31T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xabc".to_string(),
            ..Default::default()
        };

        let mut env = TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: "env-1".to_string(),
            sender_id: did.clone(),
            receiver_id: ReceiverId::One("did:arrow:consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-05-31T00:00:00Z".to_string(),
            nonce: "n-deadbeef".to_string(),
            sequence: None,
            signature: None,
            signing: None,
            correlation_id: None,
            reply_to: None,
            metadata: None,
            payload: event,
        };

        let canonical = envelope_canonical_bytes(&env).unwrap();
        let sig = sign_ed25519(&sk, &canonical);
        let sig_b64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(sig);
        env.signing = Some(EnvelopeSigningMetadata {
            alg: "Ed25519".to_string(),
            key_ref,
            sig_b64,
            created: None,
            canonicalization: CANONICALIZATION.to_string(),
            signed_fields: SIGNED_FIELDS.to_string(),
        });
        (env, did)
    }

    #[test]
    fn valid_signature_and_allowed_sender_passes() {
        let (env, did) = signed_envelope();
        let policy = ProvenancePolicy::new(vec![did.clone()], "did:arrow:consumer");
        assert_eq!(verify_envelope(&env, &policy).unwrap().signer_did, did);
    }

    #[test]
    fn unknown_sender_is_rejected() {
        let (env, _did) = signed_envelope();
        let policy = ProvenancePolicy::new(
            vec!["did:key:zSomeoneElse".to_string()],
            "did:arrow:consumer",
        );
        let err = verify_envelope(&env, &policy).unwrap_err();
        assert!(err.to_string().contains("allow-list"), "got: {err}");
    }

    #[test]
    fn empty_allow_list_trusts_no_one() {
        let (env, _did) = signed_envelope();
        let policy = ProvenancePolicy::new(vec![], "did:arrow:consumer");
        assert!(verify_envelope(&env, &policy).is_err());
    }

    #[test]
    fn receiver_mismatch_is_rejected() {
        let (env, did) = signed_envelope();
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:other-consumer");
        let err = verify_envelope(&env, &policy).unwrap_err();
        assert!(err.to_string().contains("receiver_id"), "got: {err}");
    }

    #[test]
    fn tampered_payload_breaks_the_signature() {
        let (mut env, did) = signed_envelope();
        // Mutate a signed field after signing — signature must no longer verify.
        env.payload.object_id = "0xTAMPERED".to_string();
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        assert!(verify_envelope(&env, &policy).is_err());
    }

    #[test]
    fn missing_signing_block_is_rejected() {
        let (mut env, did) = signed_envelope();
        env.signing = None;
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let err = verify_envelope(&env, &policy).unwrap_err();
        assert!(err.to_string().contains("signing block"), "got: {err}");
    }

    #[test]
    fn sender_id_spoof_is_rejected() {
        // sender_id changed to a different DID than the signing key — even if that
        // DID is allow-listed, the signer/sender mismatch must fail.
        let (mut env, _did) = signed_envelope();
        env.sender_id = "did:key:zSpoofed".to_string();
        let policy =
            ProvenancePolicy::new(vec!["did:key:zSpoofed".to_string()], "did:arrow:consumer");
        let err = verify_envelope(&env, &policy).unwrap_err();
        assert!(err.to_string().contains("does not match"), "got: {err}");
    }
}
