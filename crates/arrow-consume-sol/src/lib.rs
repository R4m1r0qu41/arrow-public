//! Arrow Protocol Consumption Adapter for Solana.
//!
//! Solana consumption uses the shared [`arrow_adapter`] contract directly
//! (no EVM dependency). **v0.1 is RecordOnly/DryRunOnly** — there is no live
//! instruction building, keypair signing, or transaction send.
//!
//! `settlement_intent` enforces five independent guards before constructing an
//! intent (same pattern as `arrow-consume-btc`):
//!
//! 1. `arrow_code` Origin field (parsed by `arrow_core`) == `SOL`.
//! 2. `event.chain_id` == `SOL`.
//! 3. `transfer.source_system` == `"solana"`.
//! 4. `transfer.source_chain_id` == `"SOL"`.
//! 5. `transfer.amount.unit` == `"lamports"`.

pub mod provenance;
pub mod settle;

use anyhow::{bail, Context, Result};
use arrow_adapter::{
    destination_execution_id, ActionIntent, ConsumerExecutionMode, ConsumerReceipt,
    ConsumerReceiptStatus, DestinationExecutionStore, InMemoryDestinationExecutionStore,
    SourceValueTransfer,
};
use arrow_core::registry::loader::Registries;
use arrow_core::types::event::ArrowEvent;
use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_reduce::{reduce, Reduction};

pub use arrow_adapter::provenance::{
    verify_envelope_provenance, ProvenancePolicy, ProvenanceVerification,
};
pub use arrow_transport::{envelope::TransportEnvelope, replay::ReplayStore};
pub use provenance::verify_envelope;

const ADAPTER_ID: &str = "arrow-consume-sol";

/// Outcome of consuming a Solana event envelope.
#[derive(Debug, Clone)]
pub struct ConsumeOutcome {
    pub reduction: Reduction,
    pub intent: Option<ActionIntent>,
    pub destination_execution_id: Option<String>,
    pub settled: bool,
    pub receipt: Option<ConsumerReceipt>,
}

/// Derive a typed Solana transfer intent from the normalized lifecycle.
///
/// Applies five fail-closed source checks before constructing the intent.
/// See module-level docs for the full guard list.
pub fn settlement_intent(event: &ArrowEvent) -> Result<ActionIntent> {
    // Guard 1: canonical Arrow code origin field.
    let parsed = arrow_core::parser::message::parse(&event.arrow_code)
        .with_context(|| format!("malformed arrow_code `{}`", event.arrow_code))?;
    if parsed.origin != "SOL" {
        bail!(
            "wrong origin: Solana consumer requires Arrow code origin SOL, \
             got `{}` (full code: `{}`)",
            parsed.origin,
            event.arrow_code
        );
    }

    // Guard 2: Tier-2 chain_id.
    match event.chain_id.as_deref() {
        Some("SOL") => {}
        other => bail!(
            "wrong origin: Solana consumer requires chain_id=SOL, got {:?}",
            other
        ),
    }

    let transfer = SourceValueTransfer::from_event(event).context(
        "typed source value transfer is required; refusing free-form-metadata settlement",
    )?;

    // Guards 3-5: source system, chain id, and unit.
    if transfer.source_system != "solana" {
        bail!(
            "wrong origin: Solana consumer requires source_system=solana, got `{}`",
            transfer.source_system
        );
    }
    if transfer.source_chain_id != "SOL" {
        bail!(
            "wrong origin: Solana consumer requires source_chain_id=SOL, got `{}`",
            transfer.source_chain_id
        );
    }
    if transfer.amount.unit != "lamports" {
        bail!(
            "wrong destination: Solana consumer requires `lamports` unit, got `{}`; \
             this event was minted for a different chain",
            transfer.amount.unit
        );
    }

    let intent = ActionIntent::sol_transfer(
        format!(
            "intent:sol-transfer:{}:{}:{}:{}",
            transfer.source_chain_id, transfer.source_tx, transfer.to, transfer.amount.value
        ),
        "SOL",
        transfer.to,
        transfer.amount.value,
        event.event_id.clone(),
        event.proof_ref.clone(),
    );
    intent
        .validate_destination("solana", "lamports")
        .map_err(|e| anyhow::anyhow!("intent destination mismatch: {e}"))?;
    Ok(intent)
}

/// Verify a signed envelope's provenance, reject replay, then consume the
/// payload in `RecordOnly` mode. No live Solana transaction is submitted.
pub fn verify_and_consume(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
) -> Result<ConsumeOutcome> {
    let execution_store = InMemoryDestinationExecutionStore::new();
    verify_and_consume_with_actions(
        envelope,
        policy,
        replay,
        &execution_store,
        ConsumerExecutionMode::RecordOnly,
    )
}

/// Lower-level entry point for tests or dry-run callers.
///
/// `ConsumerExecutionMode::Execute` is **explicitly rejected** — live Solana
/// settlement is out of scope for v0.1.
pub fn verify_and_consume_with_actions(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    execution_store: &dyn DestinationExecutionStore,
    mode: ConsumerExecutionMode,
) -> Result<ConsumeOutcome> {
    if mode.executes_destination() {
        bail!(
            "live Solana settlement is disabled in v0.1; \
             no instruction building, signing, or send"
        );
    }

    let provenance =
        verify_envelope_provenance(envelope, policy).context("provenance verification failed")?;

    if mode.records_idempotency()
        && replay.check_and_record(&envelope.sender_id, &envelope.nonce)?
    {
        bail!(
            "replay detected: nonce `{}` from `{}` was already consumed",
            envelope.nonce,
            envelope.sender_id
        );
    }

    validate_log(std::slice::from_ref(&envelope.payload))?;
    let reduction = reduce(std::slice::from_ref(&envelope.payload))?;

    if !reduction.disposition.permits_settlement() {
        let receipt = ConsumerReceipt::new(
            ADAPTER_ID,
            mode,
            ConsumerReceiptStatus::NoAction,
            envelope.envelope_id.clone(),
            envelope.payload.event_id.clone(),
            &reduction,
            provenance,
        );
        return Ok(ConsumeOutcome {
            reduction,
            intent: None,
            destination_execution_id: None,
            settled: false,
            receipt: Some(receipt),
        });
    }

    let intent = settlement_intent(&envelope.payload)?;
    let execution_id = destination_execution_id(&intent);

    if mode.records_idempotency() && execution_store.check_and_record_execution(&execution_id)? {
        bail!("destination execution replay detected: `{execution_id}` was already recorded");
    }

    let status = match mode {
        ConsumerExecutionMode::DryRun => ConsumerReceiptStatus::DryRun,
        ConsumerExecutionMode::RecordOnly => ConsumerReceiptStatus::RecordedOnly,
        ConsumerExecutionMode::Execute => unreachable!("guarded above"),
    };

    let receipt = ConsumerReceipt::new(
        ADAPTER_ID,
        mode,
        status,
        envelope.envelope_id.clone(),
        envelope.payload.event_id.clone(),
        &reduction,
        provenance,
    )
    .with_action(intent.clone(), Some(execution_id.clone()), None);

    Ok(ConsumeOutcome {
        reduction,
        intent: Some(intent),
        destination_execution_id: Some(execution_id),
        settled: false,
        receipt: Some(receipt),
    })
}

fn validate_log(events: &[ArrowEvent]) -> Result<()> {
    let registries = Registries::load_embedded().context("loading embedded registries")?;
    let pipeline = ValidationPipeline::new(&registries);
    for ev in events {
        let result = pipeline.validate_event(ev);
        if !result.passed {
            let errors: Vec<String> = result
                .violations
                .iter()
                .filter(|v| v.is_error())
                .map(|v| format!("{}: {}", v.rule_id, v.message))
                .collect();
            bail!(
                "event `{}` failed Arrow validation: {}",
                ev.event_id,
                errors.join("; ")
            );
        }
    }
    Ok(())
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

    fn sol_pending_event() -> ArrowEvent {
        let mut e = ArrowEvent {
            arrow_code: "1:SOL:PD:HL:0:B".to_string(),
            event_id: "evt-sol-pending-1".to_string(),
            lifecycle_id: "sol:5j7s8K9mN2pQ".to_string(),
            timestamp: "2026-06-03T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "5j7s8K9mN2pQ".to_string(),
            chain_id: Some("SOL".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("5j7s8K9mN2pQ".to_string()),
            finality_basis: Some("probabilistic".to_string()),
            ..Default::default()
        };
        SourceValueTransfer::sol_native(
            "SOL",
            "5j7s8K9mN2pQ",
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
            "6FEgANEPqTsHxfyoTKtGBpBqSH6QqMG1V3HxVBFLbXPa",
            "1000000000",
            None,
        )
        .attach_to_event(&mut e)
        .unwrap();
        e
    }

    fn evm_event_with_wei() -> ArrowEvent {
        let mut e = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-eth-wrong-chain-sol".to_string(),
            lifecycle_id: "eth:0xdeadbeef".to_string(),
            timestamp: "2026-06-03T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xdeadbeef".to_string(),
            chain_id: Some("ETH".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("0xdeadbeef".to_string()),
            ..Default::default()
        };
        SourceValueTransfer::evm_native(
            "ETH",
            "0xdeadbeef",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            Some(12),
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
            envelope_id: "env-sol-consume-1".to_string(),
            sender_id: did.clone(),
            receiver_id: ReceiverId::One("did:arrow:consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-06-03T00:00:00Z".to_string(),
            nonce: "n-sol-replay-test".to_string(),
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
    fn pending_sol_event_yields_no_action_receipt() {
        let (envelope, did) = signed_envelope(sol_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let out = verify_and_consume(&envelope, &policy, &replay).unwrap();
        assert!(!out.settled, "pending SOL event must not settle");
        assert!(out.intent.is_none(), "pending event carries no intent");
        let receipt = out.receipt.expect("pending event should produce a receipt");
        assert_eq!(receipt.status, ConsumerReceiptStatus::NoAction);
    }

    #[test]
    fn replay_is_rejected() {
        let (envelope, did) = signed_envelope(sol_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let first = verify_and_consume(&envelope, &policy, &replay).unwrap();
        assert!(!first.settled);
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(err.to_string().contains("replay detected"), "got: {err}");
    }

    #[test]
    fn execute_mode_is_explicitly_rejected() {
        let (envelope, did) = signed_envelope(sol_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let store = InMemoryDestinationExecutionStore::new();
        let err = verify_and_consume_with_actions(
            &envelope,
            &policy,
            &replay,
            &store,
            ConsumerExecutionMode::Execute,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("live Solana settlement is disabled"),
            "got: {err}"
        );
    }

    #[test]
    fn missing_typed_transfer_refused() {
        let mut ev = ArrowEvent {
            arrow_code: "1:SOL:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-sol-no-ext".to_string(),
            lifecycle_id: "sol:noext".to_string(),
            timestamp: "2026-06-03T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "noext".to_string(),
            chain_id: Some("SOL".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("noext".to_string()),
            ..Default::default()
        };
        ev.extensions = None;
        let (envelope, did) = signed_envelope(ev);
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(
            err.to_string().contains("typed source value transfer"),
            "got: {err}"
        );
    }

    #[test]
    fn wrong_chain_wei_transfer_is_refused_with_typed_error() {
        let (envelope, did) = signed_envelope(evm_event_with_wei());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(
            err.to_string().contains("origin SOL")
                || err.to_string().contains("chain_id")
                || err.to_string().contains("lamports"),
            "error must identify this as a wrong-chain rejection; got: {err}"
        );
    }

    #[test]
    fn eth_arrow_code_with_sol_chain_id_and_sol_transfer_is_refused() {
        // Arrow code origin ETH + chain_id=SOL + sol_native transfer: must be
        // refused at guard 1 (arrow_code origin check).
        let mut ev = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-eth-code-sol-chain".to_string(),
            lifecycle_id: "sol:forgedsig".to_string(),
            timestamp: "2026-06-03T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "forgedsig".to_string(),
            chain_id: Some("SOL".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("forgedsig".to_string()),
            ..Default::default()
        };
        SourceValueTransfer::sol_native(
            "SOL",
            "forgedsig",
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
            "6FEgANEPqTsHxfyoTKtGBpBqSH6QqMG1V3HxVBFLbXPa",
            "1000000000",
            None,
        )
        .attach_to_event(&mut ev)
        .unwrap();
        let (envelope, did) = signed_envelope(ev);
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(
            err.to_string().contains("origin SOL"),
            "must be refused at the arrow_code origin check; got: {err}"
        );
    }
}
