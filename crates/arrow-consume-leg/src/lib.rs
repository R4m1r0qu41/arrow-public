//! Fail-closed, human-in-the-loop LEG Consumption Adapter for the Arrow Protocol.
//!
//! The consumption side of the legal-attestation pipeline. It takes a **signed**
//! Arrow transport envelope carrying an `origin = LEG` event, re-verifies its
//! provenance, re-validates it, reduces it to a disposition, and — crucially —
//! routes it to a **human review queue** instead of performing any autonomous
//! action.
//!
//! This is the deliberate counterpart to the EVM consumer (`arrow-consume-eth`),
//! which settles on a destination chain when the disposition permits. A legal
//! assertion is different in kind: Arrow classifies and records legal context but
//! never determines applicable law or executes a legal order. So this adapter has
//! **no autonomous execution path at all** — its "host-system action" is to enqueue
//! a structured [`review::LegReviewItem`] for a human operator.
//!
//! # Fail-closed trust boundaries
//!
//! - **Provenance is re-verified (fail-closed).** [`verify_and_review`] re-checks
//!   the envelope Ed25519 signature, a non-empty sender allow-list, and the
//!   receiver binding ([`provenance`]) before anything else. An empty allow-list
//!   trusts no one.
//! - **The payload is re-validated.** The full Arrow [`ValidationPipeline`] runs
//!   over the event; an invalid event never reaches reduction. This is where the
//!   live LEG rules (MSG-006, LEG-004…LEG-010) are enforced, including the LEG-007
//!   supranational sanctions force-block and the LEG-009/LEG-010 jurisdiction
//!   rules.
//! - **Scope is enforced.** A non-`LEG` event is refused outright — this consumer
//!   only handles legal-origin events.
//! - **Replay is rejected.** The envelope sender/nonce is recorded before any
//!   record/execute run, and the review item is idempotent on a dedicated id.
//! - **Default deny.** A disposition maps to [`review::LegReviewDecision`]: only an
//!   explicitly non-terminal, review-eligible disposition is held for a human;
//!   blocks, reversals, closures, and any unrecognized state deny by default. Even
//!   an `ApprovedForSettlement` legal assertion is *held for review*, never acted
//!   on — a final legal authority does not buy an autonomous action.
//!
//! The live LEG rules are enforced here via the embedded registries, including
//! jurisdiction presence (LEG-009) and `leg_supra_jurisdictions` membership
//! (LEG-010).

pub mod provenance;
pub mod review;

use anyhow::{bail, Context, Result};
use arrow_adapter::{ConsumerExecutionMode, ConsumerReceipt, ConsumerReceiptStatus};
use arrow_core::parser::message::parse;
use arrow_core::registry::loader::Registries;
use arrow_core::types::event::ArrowEvent;
use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_reduce::{reduce, Reduction};

pub use arrow_transport::{envelope::TransportEnvelope, replay::ReplayStore};
pub use provenance::{ProvenancePolicy, ProvenanceVerification};
pub use review::{review_id, InMemoryReviewQueue, LegReviewDecision, LegReviewItem, ReviewQueue};

/// Outcome of consuming a signed LEG envelope.
#[derive(Debug, Clone)]
pub struct LegConsumeOutcome {
    /// The reduction that drove the decision.
    pub reduction: Reduction,
    /// The fail-closed routing decision.
    pub decision: LegReviewDecision,
    /// The review item, present whenever the decision is `HoldForReview`.
    pub review_item: Option<LegReviewItem>,
    /// Whether this call enqueued the review item (false for deny, dry-run, or a
    /// duplicate that was already queued).
    pub queued: bool,
    /// The typed consumer receipt.
    pub receipt: ConsumerReceipt,
}

/// Re-validate every event against the Arrow validation pipeline.
///
/// The consumer MUST NOT trust the input: an event that never passed a node could
/// otherwise drive a human-review decision based on an unvalidated legal claim.
pub fn validate_log(events: &[ArrowEvent]) -> Result<()> {
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

/// The Arrow Origin parsed from an event's canonical code (position 2).
fn origin_of(event: &ArrowEvent) -> Result<String> {
    let msg = parse(&event.arrow_code)
        .map_err(|e| anyhow::anyhow!("cannot parse arrow_code `{}`: {e}", event.arrow_code))?;
    Ok(msg.origin)
}

/// Verify a signed LEG envelope's provenance, reject replay, re-validate, reduce,
/// and route the result to the human review queue.
///
/// `mode` selects the side effects: [`ConsumerExecutionMode::DryRun`] computes the
/// decision without recording replay state or enqueuing; `RecordOnly` and
/// `Execute` both record replay/idempotency and enqueue a `HoldForReview` item
/// (there is no further autonomous action to distinguish them — the queue *is* the
/// action). A denied event is never queued under any mode.
pub fn verify_and_review(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    queue: &dyn ReviewQueue,
    mode: ConsumerExecutionMode,
) -> Result<LegConsumeOutcome> {
    // 1. Provenance — fail-closed: signature + allow-list + receiver binding.
    let provenance =
        provenance::verify_envelope(envelope, policy).context("provenance verification failed")?;

    // 2. Replay — reject a repeated signed envelope before any side effect.
    if mode.records_idempotency()
        && replay.check_and_record(&envelope.sender_id, &envelope.nonce)?
    {
        bail!(
            "replay detected: nonce `{}` from `{}` was already consumed",
            envelope.nonce,
            envelope.sender_id
        );
    }

    // 3. Trust boundary — never reduce an unvalidated event.
    validate_log(std::slice::from_ref(&envelope.payload))?;

    // 4. Scope — this consumer only handles legal-origin events.
    let origin = origin_of(&envelope.payload)?;
    if origin != "LEG" {
        bail!(
            "arrow-consume-leg only handles origin = LEG; refusing origin `{origin}` (event `{}`)",
            envelope.payload.event_id
        );
    }

    // 5. Reduce to a disposition.
    let reduction = reduce(std::slice::from_ref(&envelope.payload))?;
    let decision = LegReviewDecision::from_disposition(reduction.disposition);

    // 6. Route. Fail-closed: deny is terminal; hold-for-review enqueues for a human.
    match decision {
        LegReviewDecision::Deny => {
            let receipt = ConsumerReceipt::new(
                "arrow-consume-leg",
                mode,
                ConsumerReceiptStatus::NoAction,
                envelope.envelope_id.clone(),
                envelope.payload.event_id.clone(),
                &reduction,
                provenance,
            );
            Ok(LegConsumeOutcome {
                reduction,
                decision,
                review_item: None,
                queued: false,
                receipt,
            })
        }
        LegReviewDecision::HoldForReview => {
            let item = LegReviewItem::from_reduction(envelope, &reduction, &provenance.signer_did);

            // A held-for-review item is always `Quarantined` (RFC-005 §8.3,
            // CA-011 "routed for review"): the host-system action is a human-review
            // hand-off, never an autonomous destination action, so the receipt
            // carries no typed action intent regardless of mode. `mode` and
            // `queued` carry the dry-run/record/execute distinction.

            // DryRun computes the decision but commits nothing to the queue.
            if !mode.records_idempotency() {
                let receipt = ConsumerReceipt::new(
                    "arrow-consume-leg",
                    mode,
                    ConsumerReceiptStatus::Quarantined,
                    envelope.envelope_id.clone(),
                    envelope.payload.event_id.clone(),
                    &reduction,
                    provenance,
                );
                return Ok(LegConsumeOutcome {
                    reduction,
                    decision,
                    review_item: Some(item),
                    queued: false,
                    receipt,
                });
            }

            // Record-only and execute both enqueue for a human. The queue is the
            // only "action" — there is no autonomous downstream execution.
            if queue.enqueue(&item)? {
                bail!(
                    "review replay detected: item `{}` was already enqueued",
                    item.review_id
                );
            }
            let receipt = ConsumerReceipt::new(
                "arrow-consume-leg",
                mode,
                ConsumerReceiptStatus::Quarantined,
                envelope.envelope_id.clone(),
                envelope.payload.event_id.clone(),
                &reduction,
                provenance,
            );
            Ok(LegConsumeOutcome {
                reduction,
                decision,
                review_item: Some(item),
                queued: true,
                receipt,
            })
        }
    }
}

/// Parse an event file that holds either a single `ArrowEvent` object or a JSON
/// array of them.
pub fn parse_event_log(raw: &str) -> Result<Vec<ArrowEvent>> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let events = if value.is_array() {
        serde_json::from_value(value)?
    } else {
        vec![serde_json::from_value(value)?]
    };
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_adapter::ConsumerReceiptStatus;
    use arrow_identity::{
        crypto::ed25519::{generate_test_keypair, sign_ed25519},
        resolver::did_key::did_key_from_public,
    };
    use arrow_origin_leg::attestation::{map_to_arrow, LegalAttestation};
    use arrow_transport::{
        canonical::envelope_canonical_bytes,
        envelope::{EnvelopeSigningMetadata, ReceiverId},
        replay::InMemoryReplayStore,
        signature::{CANONICALIZATION, SIGNED_FIELDS},
    };
    use base64::Engine;

    fn signed_envelope(event: ArrowEvent) -> (TransportEnvelope, String) {
        let (sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let suffix = did.strip_prefix("did:key:").unwrap();
        let key_ref = format!("{did}#{suffix}");
        let mut envelope = TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: "env-leg-1".to_string(),
            sender_id: did.clone(),
            receiver_id: ReceiverId::One("did:arrow:leg-consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-05-31T00:00:00Z".to_string(),
            nonce: "n-leg-test".to_string(),
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

    fn judicial_event() -> ArrowEvent {
        map_to_arrow(&LegalAttestation::final_judicial_order(
            "MX",
            "case-2026-001",
            "did:arrow:oracle:mx-1",
        ))
    }

    fn sanctions_event() -> ArrowEvent {
        map_to_arrow(&LegalAttestation::supranational_sanctions(
            "UNSC",
            "unscr-2026-42",
            "did:arrow:oracle:un-1",
        ))
    }

    #[test]
    fn judicial_order_is_held_for_review_never_auto_acted() {
        let (envelope, did) = signed_envelope(judicial_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:leg-consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let queue = InMemoryReviewQueue::new();

        let out = verify_and_review(
            &envelope,
            &policy,
            &replay,
            &queue,
            ConsumerExecutionMode::Execute,
        )
        .unwrap();

        assert_eq!(out.decision, LegReviewDecision::HoldForReview);
        assert!(
            out.queued,
            "a valid judicial order must be queued for a human"
        );
        // Routed for review, not executed: the host-system action is a human
        // hand-off, so the receipt is Quarantined and carries no action intent.
        assert_eq!(out.receipt.status, ConsumerReceiptStatus::Quarantined);
        assert!(out.receipt.action_intent.is_none());
        // The "action" is a queued review item, not a destination transaction.
        assert!(out.receipt.destination_tx.is_none());
        assert_eq!(queue.items().len(), 1);
        assert_eq!(queue.items()[0].leg_subtype.as_deref(), Some("JUD"));
    }

    #[test]
    fn sanctions_block_is_denied_and_not_queued() {
        // SUPRA + SA + Finality=1 reduces to a dominating BL → fail-closed deny.
        let (envelope, did) = signed_envelope(sanctions_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:leg-consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let queue = InMemoryReviewQueue::new();

        let out = verify_and_review(
            &envelope,
            &policy,
            &replay,
            &queue,
            ConsumerExecutionMode::Execute,
        )
        .unwrap();

        assert_eq!(out.decision, LegReviewDecision::Deny);
        assert!(!out.queued);
        assert_eq!(out.receipt.status, ConsumerReceiptStatus::NoAction);
        assert!(queue.items().is_empty());
    }

    #[test]
    fn dry_run_computes_decision_without_queuing() {
        let (envelope, did) = signed_envelope(judicial_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:leg-consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let queue = InMemoryReviewQueue::new();

        let out = verify_and_review(
            &envelope,
            &policy,
            &replay,
            &queue,
            ConsumerExecutionMode::DryRun,
        )
        .unwrap();

        assert_eq!(out.decision, LegReviewDecision::HoldForReview);
        assert!(!out.queued, "dry-run must not enqueue");
        // Decision is the same (Quarantined); mode=DryRun and queued=false convey
        // that nothing was committed to the queue.
        assert_eq!(out.receipt.status, ConsumerReceiptStatus::Quarantined);
        assert_eq!(out.receipt.mode, ConsumerExecutionMode::DryRun);
        assert!(queue.items().is_empty());
    }

    #[test]
    fn unknown_sender_is_rejected_fail_closed() {
        let (envelope, _did) = signed_envelope(judicial_event());
        let policy = ProvenancePolicy::new(
            vec!["did:key:zSomeoneElse".to_string()],
            "did:arrow:leg-consumer",
        );
        let replay = InMemoryReplayStore::with_24h_retention();
        let queue = InMemoryReviewQueue::new();
        let err = verify_and_review(
            &envelope,
            &policy,
            &replay,
            &queue,
            ConsumerExecutionMode::Execute,
        )
        .unwrap_err();
        assert!(err.to_string().contains("provenance"), "got: {err}");
    }

    #[test]
    fn replayed_envelope_is_rejected() {
        let (envelope, did) = signed_envelope(judicial_event());
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:leg-consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let queue = InMemoryReviewQueue::new();

        verify_and_review(
            &envelope,
            &policy,
            &replay,
            &queue,
            ConsumerExecutionMode::Execute,
        )
        .unwrap();
        let err = verify_and_review(
            &envelope,
            &policy,
            &replay,
            &queue,
            ConsumerExecutionMode::Execute,
        )
        .unwrap_err();
        assert!(err.to_string().contains("replay detected"), "got: {err}");
    }

    #[test]
    fn non_leg_origin_is_refused() {
        // An ETH-origin event must not be consumable by the LEG adapter.
        let mut ev = judicial_event();
        ev.arrow_code = "1:ETH:FN:PR:1:A".to_string();
        // Satisfy LFC-008 so the refusal under test is the origin check,
        // not the closure-expectation gate.
        ev.closure_eta = Some(0);
        ev.closure_eta_type = Some("block_count".to_string());
        ev.leg_subtype = None;
        ev.leg_jurisdiction = None;
        let (envelope, did) = signed_envelope(ev);
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:leg-consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let queue = InMemoryReviewQueue::new();
        let err = verify_and_review(
            &envelope,
            &policy,
            &replay,
            &queue,
            ConsumerExecutionMode::DryRun,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("only handles origin = LEG"),
            "got: {err}"
        );
    }
}
