//! Arrow Protocol Consumption Adapter for Bitcoin.
//!
//! Bitcoin consumption uses the shared [`arrow_adapter`] contract directly
//! (no EVM dependency). **v0.1 is RecordOnly/DryRunOnly** — there is no live
//! UTXO selection, PSBT construction, signing, or broadcast. The consumer:
//!
//! 1. Rejects `ConsumerExecutionMode::Execute` immediately (fail-closed).
//! 2. Re-verifies envelope provenance via the shared `verify_envelope_provenance`.
//! 3. Rejects envelope replay via the [`ReplayStore`].
//! 4. Re-runs [`ValidationPipeline`] on the payload.
//! 5. Reduces via `arrow-reduce`.
//! 6. Derives a typed [`ActionIntent::btc_transfer`] only when the disposition
//!    permits settlement — and only from the typed [`SourceValueTransfer`], never
//!    free-form metadata.
//! 7. Calls [`ActionIntent::validate_destination`] (`"bitcoin"`, `"sats"`) as a
//!    fail-closed destination-model check; non-Bitcoin intents (e.g. `wei` unit)
//!    produce a named typed error rather than a panic or silent coercion.
//! 8. Returns a typed [`ConsumerReceipt`].

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

const ADAPTER_ID: &str = "arrow-consume-btc";

/// Outcome of consuming a Bitcoin event envelope.
#[derive(Debug, Clone)]
pub struct ConsumeOutcome {
    pub reduction: Reduction,
    pub intent: Option<ActionIntent>,
    pub destination_execution_id: Option<String>,
    pub settled: bool,
    pub receipt: Option<ConsumerReceipt>,
}

/// Derive a typed Bitcoin transfer intent from the normalized lifecycle.
///
/// Checks five independent guards before constructing an intent. All five are
/// required — `validate_destination` only inspects the intent we *already built*,
/// so it cannot catch source mismatches. Arrow core validation does not validate
/// adapter extensions, making these the real fail-closed boundary:
///
/// 1. `arrow_code` Origin field (position 2) == `BTC` — the canonical identity.
/// 2. `event.chain_id` == `BTC` — the Tier-2 chain identifier.
/// 3. `transfer.source_system` == `"bitcoin"`.
/// 4. `transfer.source_chain_id` == `"BTC"`.
/// 5. `transfer.amount.unit` == `"sats"`.
///
/// Guard 1 closes the gap left when chain_id is set to BTC but the Arrow code
/// Origin is a different chain (e.g. `1:ETH:FN:PR:1:A` + `chain_id=BTC`).
pub fn settlement_intent(event: &ArrowEvent) -> Result<ActionIntent> {
    // Guard 1: parse the canonical Arrow code and check the Origin field.
    // Using the core parser (not a manual split) so a structurally invalid
    // arrow_code is rejected here rather than producing a silent empty origin.
    let parsed = arrow_core::parser::message::parse(&event.arrow_code)
        .with_context(|| format!("malformed arrow_code `{}`", event.arrow_code))?;
    if parsed.origin != "BTC" {
        bail!(
            "wrong origin: Bitcoin consumer requires Arrow code origin BTC, \
             got `{}` (full code: `{}`)",
            parsed.origin,
            event.arrow_code
        );
    }

    // Guard 2: Tier-2 chain_id.
    match event.chain_id.as_deref() {
        Some("BTC") => {}
        other => bail!(
            "wrong origin: Bitcoin consumer requires chain_id=BTC, got {:?}; \
             refusing to derive a BTC intent from a non-Bitcoin event",
            other
        ),
    }

    let transfer = SourceValueTransfer::from_event(event).context(
        "typed source value transfer is required; refusing free-form-metadata settlement",
    )?;

    if transfer.source_system != "bitcoin" {
        bail!(
            "wrong origin: Bitcoin consumer requires source_system=bitcoin, got `{}`",
            transfer.source_system
        );
    }
    if transfer.source_chain_id != "BTC" {
        bail!(
            "wrong origin: Bitcoin consumer requires source_chain_id=BTC, got `{}`",
            transfer.source_chain_id
        );
    }
    if transfer.amount.unit != "sats" {
        bail!(
            "wrong destination: Bitcoin consumer requires `sats` unit, got `{}`; \
             this event was minted for a different chain",
            transfer.amount.unit
        );
    }
    let intent = ActionIntent::btc_transfer(
        format!(
            "intent:btc-transfer:{}:{}:{}:{}",
            transfer.source_chain_id, transfer.source_tx, transfer.to, transfer.amount.value
        ),
        "BTC",
        transfer.to,
        transfer.amount.value,
        event.event_id.clone(),
        event.proof_ref.clone(),
    );
    // Defence-in-depth: the intent we just built must be Bitcoin-shaped.
    intent
        .validate_destination("bitcoin", "sats")
        .map_err(|e| anyhow::anyhow!("intent destination mismatch: {e}"))?;
    Ok(intent)
}

/// Verify a signed envelope's provenance, reject replay, then consume the
/// payload in `RecordOnly` mode. No live Bitcoin transaction is submitted.
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
/// `ConsumerExecutionMode::Execute` is **explicitly rejected** — live Bitcoin
/// settlement (UTXO selection, PSBT signing, broadcast) is out of scope for v0.1.
pub fn verify_and_consume_with_actions(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    execution_store: &dyn DestinationExecutionStore,
    mode: ConsumerExecutionMode,
) -> Result<ConsumeOutcome> {
    if mode.executes_destination() {
        bail!(
            "live Bitcoin settlement is disabled in v0.1; \
             no UTXO selection, PSBT signing, or broadcast"
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

    fn btc_pending_event() -> ArrowEvent {
        let mut e = ArrowEvent {
            arrow_code: "1:BTC:PD:HL:0:B".to_string(),
            event_id: "evt-btc-pending-1".to_string(),
            lifecycle_id: "btc:a1b2c3d4".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "a1b2c3d4".to_string(),
            chain_id: Some("BTC".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("a1b2c3d4:0".to_string()),
            finality_basis: Some("probabilistic".to_string()),
            ..Default::default()
        };
        SourceValueTransfer::btc_native(
            "BTC",
            "a1b2c3d4",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf",
            "1BpEi6DfDAUFd153wiGrvkiboLLaqFZu",
            "5000000",
            Some(0),
        )
        .attach_to_event(&mut e)
        .unwrap();
        e
    }

    fn evm_event_with_wei() -> ArrowEvent {
        // A valid-looking ETH event — but unit is "wei", which the BTC consumer
        // must refuse with a typed wrong-chain error.
        let mut e = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-eth-wrong-chain".to_string(),
            lifecycle_id: "eth:0xdeadbeef".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
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
            envelope_id: "env-btc-consume-1".to_string(),
            sender_id: did.clone(),
            receiver_id: ReceiverId::One("did:arrow:consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-06-02T00:00:00Z".to_string(),
            nonce: "n-btc-replay-test".to_string(),
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
    fn pending_btc_event_yields_no_action_receipt() {
        let (envelope, did) = signed_envelope(btc_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let out = verify_and_consume(&envelope, &policy, &replay).unwrap();
        assert!(!out.settled, "pending BTC event must not settle");
        assert!(out.intent.is_none(), "pending event carries no intent");
        let receipt = out.receipt.expect("pending event should produce a receipt");
        assert_eq!(receipt.status, ConsumerReceiptStatus::NoAction);
    }

    #[test]
    fn replay_is_rejected() {
        let (envelope, did) = signed_envelope(btc_pending_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let first = verify_and_consume(&envelope, &policy, &replay).unwrap();
        assert!(!first.settled);
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(err.to_string().contains("replay detected"), "got: {err}");
    }

    #[test]
    fn execute_mode_is_explicitly_rejected() {
        let (envelope, did) = signed_envelope(btc_pending_event());
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
                .contains("live Bitcoin settlement is disabled"),
            "got: {err}"
        );
    }

    #[test]
    fn missing_typed_transfer_refused() {
        let mut ev = ArrowEvent {
            arrow_code: "1:BTC:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-btc-no-ext".to_string(),
            lifecycle_id: "btc:0xnone".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xnone".to_string(),
            chain_id: Some("BTC".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("0xnone:0".to_string()),
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
        // An EVM event (unit=wei) fed to the Bitcoin consumer must fail with a
        // typed wrong-chain error — not a panic or silent coercion.
        let (envelope, did) = signed_envelope(evm_event_with_wei());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(
            err.to_string().contains("origin BTC")
                || err.to_string().contains("chain_id")
                || err.to_string().contains("sats"),
            "error must identify this as a wrong-chain rejection; got: {err}"
        );
    }

    #[test]
    fn non_btc_event_with_forged_sats_transfer_is_refused() {
        // Regression for P1: a 1:ETH:FN:PR:1:A event carrying a forged sats
        // SourceValueTransfer must be refused. The unit check alone is not
        // sufficient — chain_id, source_system, and source_chain_id are all
        // required guards because validate_destination only inspects the intent
        // we already constructed from the transfer.
        let mut ev = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-eth-forged-sats".to_string(),
            lifecycle_id: "eth:0xforged".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xforged".to_string(),
            chain_id: Some("ETH".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("0xforged".to_string()),
            ..Default::default()
        };
        // Attach a sats transfer on an ETH event — the forged scenario.
        SourceValueTransfer::btc_native(
            "BTC",
            "0xforged",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf",
            "1BpEi6DfDAUFd153wiGrvkiboLLaqFZu",
            "5000000",
            Some(6),
        )
        .attach_to_event(&mut ev)
        .unwrap();
        let (envelope, did) = signed_envelope(ev);
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(
            err.to_string().contains("origin BTC") || err.to_string().contains("chain_id"),
            "must be refused at origin or chain_id check; got: {err}"
        );
    }

    #[test]
    fn eth_arrow_code_with_btc_chain_id_and_btc_transfer_is_refused() {
        // Regression for P1 (second layer): 1:ETH:FN:PR:1:A with chain_id=BTC
        // and a BTC-shaped SourceValueTransfer passes the chain_id + source
        // checks but must be caught by the arrow_code origin guard.
        let mut ev = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-eth-code-btc-chain".to_string(),
            lifecycle_id: "btc:deadbeef".to_string(),
            timestamp: "2026-06-02T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "deadbeef".to_string(),
            chain_id: Some("BTC".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("deadbeef:0".to_string()),
            ..Default::default()
        };
        SourceValueTransfer::btc_native(
            "BTC",
            "deadbeef",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf",
            "1BpEi6DfDAUFd153wiGrvkiboLLaqFZu",
            "5000000",
            Some(6),
        )
        .attach_to_event(&mut ev)
        .unwrap();
        let (envelope, did) = signed_envelope(ev);
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = verify_and_consume(&envelope, &policy, &replay).unwrap_err();
        assert!(
            err.to_string().contains("origin BTC") || err.to_string().contains("arrow_code"),
            "must be refused at the arrow_code origin check; got: {err}"
        );
    }
}
