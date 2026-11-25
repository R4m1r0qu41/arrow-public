//! Arrow Protocol Consumption Adapter for Arbitrum One.
//!
//! Arbitrum consumption is an [`arrow_consume_evm::EvmConsumerConfig::arb()`]
//! instance over the shared EVM consumer library. **v0.1 is RecordOnly** —
//! the 7-day BoLD challenge window must elapse (and be confirmed by a
//! settlement watcher) before any live Arbitrum transaction is safe to execute.
//! This wrapper hardwires `RecordOnly` mode and does not require an operator
//! config from callers.

pub mod provenance;
pub mod settle;

use anyhow::{bail, Result};
use arrow_adapter::{
    ActionIntent, ConsumerExecutionMode, DestinationExecutionStore,
    InMemoryDestinationExecutionStore,
};
use arrow_consume_evm::{EvmConsumerConfig, OperatorConfig};
use arrow_core::types::event::ArrowEvent;

pub use arrow_consume_evm::{parse_event_log, validate_log, ConsumeOutcome};
pub use arrow_transport::{envelope::TransportEnvelope, replay::ReplayStore};
pub use provenance::{ProvenancePolicy, ProvenanceVerification};
pub use settle::SettleParams;

fn arb_config() -> EvmConsumerConfig {
    EvmConsumerConfig::arb()
}

fn dummy_operator() -> OperatorConfig {
    OperatorConfig {
        rpc_url: String::new(),
        from: String::new(),
        ..Default::default()
    }
}

/// Derive the Arbitrum destination action intent from the normalized lifecycle.
pub fn settlement_intent(event: &ArrowEvent) -> Result<ActionIntent> {
    arrow_consume_evm::settlement_intent(event, &arb_config())
}

/// Verify a signed envelope's provenance, reject replay, then consume the
/// payload in `RecordOnly` mode. No live Arbitrum transaction is submitted.
pub fn verify_and_consume(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
) -> Result<ConsumeOutcome> {
    let execution_store = InMemoryDestinationExecutionStore::new();
    arrow_consume_evm::verify_and_consume_with_actions(
        envelope,
        policy,
        replay,
        &execution_store,
        &dummy_operator(),
        &arb_config(),
        ConsumerExecutionMode::RecordOnly,
    )
}

/// Lower-level entry point for tests that need explicit action over the
/// execution store or mode.
pub fn verify_and_consume_with_actions(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    execution_store: &dyn DestinationExecutionStore,
    mode: ConsumerExecutionMode,
) -> Result<ConsumeOutcome> {
    if mode.executes_destination() {
        bail!(
            "live Arbitrum settlement is disabled in v0.1; settlement-watcher proof is required before Execute mode"
        );
    }
    arrow_consume_evm::verify_and_consume_with_actions(
        envelope,
        policy,
        replay,
        execution_store,
        &dummy_operator(),
        &arb_config(),
        mode,
    )
}

/// Parse an event file holding a single `ArrowEvent` or a JSON array.
pub fn parse_event_log_file(raw: &str) -> Result<Vec<ArrowEvent>> {
    arrow_consume_evm::parse_event_log(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_adapter::{ConsumerReceiptStatus, SourceValueTransfer};
    use arrow_identity::crypto::ed25519::{generate_test_keypair, sign_ed25519};
    use arrow_identity::resolver::did_key::did_key_from_public;
    use arrow_transport::{
        canonical::envelope_canonical_bytes,
        envelope::{EnvelopeSigningMetadata, ReceiverId},
        replay::InMemoryReplayStore,
        signature::{CANONICALIZATION, SIGNED_FIELDS},
    };
    use base64::Engine;

    fn arb_pending_event() -> ArrowEvent {
        let mut e = ArrowEvent {
            arrow_code: "1:ARB:PD:HL:0:B".to_string(),
            event_id: "evt-arb-pending-1".to_string(),
            lifecycle_id: "arb:0xarb123".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xarb123".to_string(),
            chain_id: Some("ARB".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("0xarb123".to_string()),
            finality_basis: Some("optimistic_window".to_string()),
            // optimistic_window sets requires_finality_eta (FB-001): date the
            // challenge-window close (7 days after the event timestamp).
            finality_eta: Some(1780963200),
            finality_eta_type: Some("unix_ts".to_string()),
            ..Default::default()
        };
        SourceValueTransfer::evm_native(
            "ARB",
            "0xarb123",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            None,
        )
        .attach_to_event(&mut e)
        .unwrap();
        e
    }

    fn signed_envelope(event: ArrowEvent) -> (TransportEnvelope, String) {
        let (sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let suffix = did.strip_prefix("did:key:").unwrap();
        let key_ref = format!("{did}#{suffix}");
        let mut envelope = TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: "env-arb-consume-1".to_string(),
            sender_id: did.clone(),
            receiver_id: ReceiverId::One("did:arrow:consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-06-02T00:00:00Z".to_string(),
            nonce: "n-arb-replay-test".to_string(),
            sequence: None,
            signature: None,
            signing: None,
            correlation_id: None,
            reply_to: None,
            metadata: None,
            payload: event,
        };
        let canonical = envelope_canonical_bytes(&envelope).unwrap();
        let sig_b64 =
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(sign_ed25519(&sk, &canonical));
        envelope.signing = Some(EnvelopeSigningMetadata {
            alg: "Ed25519".to_string(),
            key_ref,
            sig_b64,
            created: None,
            canonicalization: CANONICALIZATION.to_string(),
            signed_fields: SIGNED_FIELDS.to_string(),
        });
        (envelope, did)
    }

    #[test]
    fn pending_l2_event_yields_no_action_receipt() {
        let (envelope, did) = signed_envelope(arb_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let out = verify_and_consume(&envelope, &policy, &replay).unwrap();
        assert!(!out.settled, "pending L2 event must not settle");
        assert!(out.intent.is_none(), "pending event carries no intent");
        let receipt = out
            .receipt
            .expect("pending event should produce a NoAction receipt");
        assert_eq!(receipt.status, ConsumerReceiptStatus::NoAction);
    }

    #[test]
    fn replay_is_rejected() {
        let (envelope, did) = signed_envelope(arb_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let first = verify_and_consume(&envelope, &policy, &replay).unwrap();
        assert!(!first.settled);
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(err.to_string().contains("replay detected"), "got: {err}");
    }

    #[test]
    fn execute_mode_is_explicitly_rejected() {
        let (envelope, did) = signed_envelope(arb_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let execution_store = InMemoryDestinationExecutionStore::new();
        let err = verify_and_consume_with_actions(
            &envelope,
            &policy,
            &replay,
            &execution_store,
            ConsumerExecutionMode::Execute,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("live Arbitrum settlement is disabled in v0.1"),
            "Execute must fail closed before any destination path; got: {err}"
        );
    }

    #[test]
    fn missing_typed_transfer_refused() {
        let mut ev = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-arb-no-ext".to_string(),
            lifecycle_id: "eth:0xnone".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xnone".to_string(),
            chain_id: Some("ETH".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("0xnone".to_string()),
            ..Default::default()
        };
        ev.extensions = None;
        let (envelope, did) = signed_envelope(ev);
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(
            err.to_string().contains("typed source value transfer"),
            "must refuse without typed source transfer; got: {err}"
        );
    }
}
