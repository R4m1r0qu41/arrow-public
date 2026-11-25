//! Arrow Protocol Consumption Adapter for Ethereum.
//!
//! Ethereum settlement is an [`EvmConsumerConfig::eth()`] instance over the
//! shared [`arrow_consume_evm`] library: re-validate an Arrow event log, reduce
//! it to a disposition, and — when the disposition permits — settle on a
//! destination EVM chain. This crate preserves the historical
//! `arrow-consume-eth` public surface by thinly wrapping the library with the
//! Ethereum config; see [`arrow_consume_evm`] for the trust-boundary notes.

pub mod provenance;
pub mod settle;

use anyhow::Result;
use arrow_adapter::{ActionIntent, ConsumerExecutionMode, DestinationExecutionStore};
use arrow_consume_evm::EvmConsumerConfig;
use arrow_core::types::event::ArrowEvent;

pub use arrow_consume_evm::{ConsumeOutcome, OperatorConfig};
pub use arrow_transport::{envelope::TransportEnvelope, replay::ReplayStore};
pub use provenance::{ProvenancePolicy, ProvenanceVerification};
pub use settle::SettleParams;

/// The Ethereum consumer configuration (adapter id `arrow-consume-eth`,
/// destination chain `ETH`, source unit `wei`).
fn eth_config() -> EvmConsumerConfig {
    EvmConsumerConfig::eth()
}

/// Re-validate every event against the Arrow validation pipeline.
pub fn validate_log(events: &[ArrowEvent]) -> Result<()> {
    arrow_consume_evm::validate_log(events)
}

/// Derive the Ethereum destination action intent from the normalized lifecycle.
pub fn settlement_intent(event: &ArrowEvent) -> Result<ActionIntent> {
    arrow_consume_evm::settlement_intent(event, &eth_config())
}

/// Re-validate, reduce, and (unless `dry_run`) settle on chain B.
pub fn consume_and_settle(
    events: &[ArrowEvent],
    operator: &OperatorConfig,
    dry_run: bool,
) -> Result<ConsumeOutcome> {
    arrow_consume_evm::consume_and_settle(events, operator, &eth_config(), dry_run)
}

/// Verify a signed envelope's provenance, reject replay, then consume its payload.
pub fn verify_and_consume(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    operator: &OperatorConfig,
    dry_run: bool,
) -> Result<ConsumeOutcome> {
    arrow_consume_evm::verify_and_consume(
        envelope,
        policy,
        replay,
        operator,
        &eth_config(),
        dry_run,
    )
}

pub fn verify_and_consume_with_actions(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    execution_store: &dyn DestinationExecutionStore,
    operator: &OperatorConfig,
    mode: ConsumerExecutionMode,
) -> Result<ConsumeOutcome> {
    arrow_consume_evm::verify_and_consume_with_actions(
        envelope,
        policy,
        replay,
        execution_store,
        operator,
        &eth_config(),
        mode,
    )
}

/// Parse an event file that holds either a single `ArrowEvent` or a JSON array.
pub fn parse_event_log(raw: &str) -> Result<Vec<ArrowEvent>> {
    arrow_consume_evm::parse_event_log(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_adapter::SourceValueTransfer;
    use arrow_identity::{
        crypto::ed25519::{generate_test_keypair, sign_ed25519},
        resolver::did_key::did_key_from_public,
    };
    use arrow_transport::{
        canonical::envelope_canonical_bytes,
        envelope::{EnvelopeSigningMetadata, ReceiverId},
        replay::InMemoryReplayStore,
        signature::{CANONICALIZATION, SIGNED_FIELDS},
    };
    use base64::Engine;

    /// Build a validation-passing, observed-style event (mirrors what the Origin
    /// Adapter mints, so it survives `validate_log`). `pending` adds the
    /// `finality_basis` that FB-001 requires when Finality=0.
    fn observed_event(arrow_code: &str, pending: bool) -> ArrowEvent {
        let settled = arrow_code.contains(":FN:") && arrow_code.ends_with(":1:A");
        let mut e = ArrowEvent {
            arrow_code: arrow_code.to_string(),
            // LFC-008: settled Class 1 events under bounded discipline declare
            // closure due immediately.
            closure_eta: if settled { Some(0) } else { None },
            closure_eta_type: if settled {
                Some("block_count".to_string())
            } else {
                None
            },
            event_id: "evt-1".to_string(),
            lifecycle_id: "life-1".to_string(),
            timestamp: "2026-05-31T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xabc".to_string(),
            chain_id: Some("ETH".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("0xabc".to_string()),
            metadata: Some(serde_json::json!({
                "to": "0x2222222222222222222222222222222222222222",
                "value_wei": "1000000000000000000",
            })),
            ..Default::default()
        };
        SourceValueTransfer::evm_native(
            "ETH",
            "0xabc",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            Some(if pending { 1 } else { 12 }),
        )
        .attach_to_event(&mut e)
        .unwrap();
        if pending {
            e.finality_basis = Some("probabilistic".to_string());
            e.finality_eta = Some(12);
            e.finality_eta_type = Some("block_count".to_string());
        }
        e
    }

    fn btc_event_with_forged_wei_transfer() -> ArrowEvent {
        let mut e = ArrowEvent {
            arrow_code: "1:BTC:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-forged-btc-wei".to_string(),
            lifecycle_id: "btc:aabbccdd".to_string(),
            timestamp: "2026-06-03T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "aabbccdd".to_string(),
            chain_id: Some("BTC".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("aabbccdd:0".to_string()),
            metadata: None,
            ..Default::default()
        };
        SourceValueTransfer::evm_native(
            "BTC",
            "aabbccdd",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            Some(6),
        )
        .attach_to_event(&mut e)
        .unwrap();
        e
    }

    fn operator() -> OperatorConfig {
        OperatorConfig {
            rpc_url: "http://127.0.0.1:1".to_string(), // unreachable on purpose
            from: "0x1111111111111111111111111111111111111111".to_string(),
            ..Default::default()
        }
    }

    fn signed_envelope(event: ArrowEvent) -> (TransportEnvelope, String) {
        let (sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let suffix = did.strip_prefix("did:key:").unwrap();
        let key_ref = format!("{did}#{suffix}");
        let mut envelope = TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: "env-1".to_string(),
            sender_id: did.clone(),
            receiver_id: ReceiverId::One("did:arrow:consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-05-31T00:00:00Z".to_string(),
            nonce: "n-replay-test".to_string(),
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
    fn pending_event_does_not_settle() {
        let out = consume_and_settle(
            &[observed_event("1:ETH:PD:HL:0:B", true)],
            &operator(),
            false,
        )
        .unwrap();
        assert!(!out.settled);
        assert!(out.intent.is_none());
        assert!(out.chain_b_tx.is_none());
    }

    #[test]
    fn approved_event_yields_event_bound_intent_under_dry_run() {
        let out = consume_and_settle(
            &[observed_event("1:ETH:FN:PR:1:A", false)],
            &operator(),
            true,
        )
        .unwrap();
        assert!(out.reduction.disposition.permits_settlement());
        assert!(!out.settled, "dry-run must not settle");
        let intent = out.intent.expect("approved event should carry an intent");
        // Intent comes from the typed source transfer, not free-form metadata or operator config.
        assert_eq!(intent.amount.value, "1000000000000000000");
        assert_eq!(
            intent.destination.address,
            "0x2222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn free_form_metadata_does_not_drive_action_intent() {
        let mut ev = observed_event("1:ETH:FN:PR:1:A", false);
        ev.metadata = Some(serde_json::json!({
            "to": "0x9999999999999999999999999999999999999999",
            "value_wei": "1"
        }));

        let out = consume_and_settle(&[ev], &operator(), true).unwrap();
        let intent = out.intent.expect("approved event should carry an intent");
        assert_eq!(intent.amount.value, "1000000000000000000");
        assert_eq!(
            intent.destination.address,
            "0x2222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn invalid_event_is_rejected_before_settlement() {
        // Finality=0 with executing Action PR violates SEM-013; finality_basis is
        // present so this is specifically a semantic rejection, not FB-001.
        let mut bad = observed_event("1:ETH:PD:PR:0:B", true);
        bad.event_id = "evt-bad".to_string();
        let err = consume_and_settle(&[bad], &operator(), false).unwrap_err();
        assert!(
            err.to_string().contains("failed Arrow validation"),
            "got: {err}"
        );
    }

    #[test]
    fn approved_event_without_typed_source_transfer_refuses_to_settle() {
        let mut ev = observed_event("1:ETH:FN:PR:1:A", false);
        ev.extensions = None;
        let err = consume_and_settle(&[ev], &operator(), false).unwrap_err();
        assert!(
            err.to_string().contains("typed source value transfer"),
            "should refuse to settle without a typed source transfer; got: {err}"
        );
    }

    #[test]
    fn non_evm_event_with_forged_wei_transfer_is_refused() {
        let err = consume_and_settle(&[btc_event_with_forged_wei_transfer()], &operator(), true)
            .unwrap_err();
        assert!(
            err.to_string().contains("EVM-family Arrow code origin"),
            "canonical Arrow origin must constrain the typed extension; got: {err}"
        );
    }

    #[test]
    fn verify_and_consume_rejects_replayed_envelope_nonce() {
        let (envelope, did) = signed_envelope(observed_event("1:ETH:PD:HL:0:B", true));
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();

        let first = verify_and_consume(&envelope, &policy, &replay, &operator(), false).unwrap();
        assert!(!first.settled);

        let err = verify_and_consume(&envelope, &policy, &replay, &operator(), false).unwrap_err();
        assert!(err.to_string().contains("replay detected"), "got: {err}");
    }

    #[test]
    fn parse_single_or_array() {
        let single = parse_event_log(r#"{"arrow_code":"1:ETH:FN:PR:1:A","event_id":"e","lifecycle_id":"l","timestamp":"t","object_type":"transfer","object_id":"o"}"#).unwrap();
        assert_eq!(single.len(), 1);
        let arr = parse_event_log(r#"[{"arrow_code":"1:ETH:FN:PR:1:A","event_id":"e","lifecycle_id":"l","timestamp":"t","object_type":"transfer","object_id":"o"}]"#).unwrap();
        assert_eq!(arr.len(), 1);
    }
}
