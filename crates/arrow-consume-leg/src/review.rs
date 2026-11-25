//! The human-in-the-loop review queue.
//!
//! The LEG Consumption Adapter never performs a destination action itself. Its
//! "host-system action" is to enqueue a structured [`LegReviewItem`] for a human
//! operator, who alone decides whether to act on the asserted legal fact. This
//! module defines that queue, its idempotency key, and the fail-closed decision a
//! disposition maps to.

use std::collections::HashSet;
use std::sync::Mutex;

use arrow_reduce::Disposition;
use arrow_transport::envelope::TransportEnvelope;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReviewQueueError {
    #[error("review queue lock poisoned")]
    LockPoisoned,
    #[error("review queue I/O error: {0}")]
    Io(String),
}

/// What the adapter does with a verified, reduced LEG event. Fail-closed: only an
/// explicitly non-terminal, review-eligible disposition is ever held for a human;
/// everything else is denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegReviewDecision {
    /// Route to a human for a decision. No autonomous action is taken.
    HoldForReview,
    /// Terminal deny: a dominating block/reversal/closure, or any disposition not
    /// explicitly eligible for review. No action, and nothing queued.
    Deny,
}

impl LegReviewDecision {
    /// Map a reduced [`Disposition`] to a fail-closed review decision.
    ///
    /// A LEG event NEVER authorizes an autonomous action — not even
    /// `ApprovedForSettlement`, which for a legal assertion still means "a final
    /// legal authority asserted this; a human must decide what to do." Only the
    /// real, non-terminal dispositions are held for review; the terminal-negative
    /// ones, and any future/unknown variant, deny by default.
    pub fn from_disposition(disposition: Disposition) -> Self {
        match disposition {
            Disposition::ApprovedForSettlement
            | Disposition::Pending
            | Disposition::Quarantined => LegReviewDecision::HoldForReview,
            Disposition::Blocked | Disposition::Reversed | Disposition::Closed => {
                LegReviewDecision::Deny
            }
        }
    }
}

/// A structured item placed on the human review queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegReviewItem {
    /// Stable idempotency key for this review item (see [`review_id`]).
    pub review_id: String,
    pub envelope_id: String,
    pub event_id: String,
    pub lifecycle_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub leg_subtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub jurisdiction: Option<String>,
    /// Canonical Action the reduction settled on (`BL`/`HL`/`RW`/…).
    pub action: String,
    /// Derived disposition label (debug form of [`Disposition`]).
    pub disposition: String,
    /// Human-legible explanation carried from the reduction.
    pub reason: String,
    /// DID that signed the transport envelope (verified provenance).
    pub signer_did: String,
    pub queued_at: String,
}

/// Stable review-item idempotency key, distinct from envelope replay.
///
/// This intentionally excludes transport-envelope ids and nonces. Envelope replay
/// protection catches the same signed envelope; review idempotency must also
/// catch the same legal assertion rewrapped in a fresh envelope.
pub fn review_id(envelope: &TransportEnvelope) -> String {
    #[derive(Serialize)]
    struct Preimage<'a> {
        contract: &'static str,
        lifecycle_id: &'a str,
        object_id: &'a str,
        arrow_code: &'a str,
        proof_ref: Option<&'a str>,
    }
    let event = &envelope.payload;
    let preimage = Preimage {
        contract: "arrow-leg-review-id-v1",
        lifecycle_id: &event.lifecycle_id,
        object_id: &event.object_id,
        arrow_code: &event.arrow_code,
        proof_ref: event.proof_ref.as_deref(),
    };
    let bytes = serde_json::to_vec(&preimage).expect("serializing review-id preimage");
    format!("rev-{}", hex::encode(Sha256::digest(bytes)))
}

impl LegReviewItem {
    /// Build a review item from a verified envelope and its reduction.
    pub fn from_reduction(
        envelope: &TransportEnvelope,
        reduction: &arrow_reduce::Reduction,
        signer_did: impl Into<String>,
    ) -> Self {
        let event = &envelope.payload;
        Self {
            review_id: review_id(envelope),
            envelope_id: envelope.envelope_id.clone(),
            event_id: event.event_id.clone(),
            lifecycle_id: reduction.lifecycle_id.clone(),
            leg_subtype: event.leg_subtype.clone(),
            jurisdiction: event.jurisdiction.clone(),
            action: reduction.action.clone(),
            disposition: format!("{:?}", reduction.disposition),
            reason: reduction.reason.clone(),
            signer_did: signer_did.into(),
            queued_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Durable-ish sink for review items. Idempotent on [`LegReviewItem::review_id`].
pub trait ReviewQueue: Send + Sync {
    /// Enqueue `item`. Returns `true` if an item with the same `review_id` was
    /// already present (and the new one is therefore a no-op duplicate).
    fn enqueue(&self, item: &LegReviewItem) -> Result<bool, ReviewQueueError>;
}

/// In-memory review queue. Retains enqueued items for inspection (CLI / tests).
#[derive(Debug, Default)]
pub struct InMemoryReviewQueue {
    seen: Mutex<HashSet<String>>,
    items: Mutex<Vec<LegReviewItem>>,
}

impl InMemoryReviewQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the items enqueued so far.
    pub fn items(&self) -> Vec<LegReviewItem> {
        self.items.lock().expect("review queue lock").clone()
    }
}

impl ReviewQueue for InMemoryReviewQueue {
    fn enqueue(&self, item: &LegReviewItem) -> Result<bool, ReviewQueueError> {
        let mut seen = self
            .seen
            .lock()
            .map_err(|_| ReviewQueueError::LockPoisoned)?;
        if seen.contains(&item.review_id) {
            return Ok(true);
        }
        seen.insert(item.review_id.clone());
        self.items
            .lock()
            .map_err(|_| ReviewQueueError::LockPoisoned)?
            .push(item.clone());
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::types::event::ArrowEvent;
    use arrow_transport::envelope::ReceiverId;

    #[test]
    fn terminal_dispositions_deny() {
        assert_eq!(
            LegReviewDecision::from_disposition(Disposition::Blocked),
            LegReviewDecision::Deny
        );
        assert_eq!(
            LegReviewDecision::from_disposition(Disposition::Reversed),
            LegReviewDecision::Deny
        );
        assert_eq!(
            LegReviewDecision::from_disposition(Disposition::Closed),
            LegReviewDecision::Deny
        );
    }

    #[test]
    fn even_approved_is_held_for_review() {
        // Fail-closed, human-in-the-loop: a final legal authority does not buy an
        // autonomous action.
        assert_eq!(
            LegReviewDecision::from_disposition(Disposition::ApprovedForSettlement),
            LegReviewDecision::HoldForReview
        );
        assert_eq!(
            LegReviewDecision::from_disposition(Disposition::Pending),
            LegReviewDecision::HoldForReview
        );
        assert_eq!(
            LegReviewDecision::from_disposition(Disposition::Quarantined),
            LegReviewDecision::HoldForReview
        );
    }

    fn envelope(envelope_id: &str, nonce: &str, event: ArrowEvent) -> TransportEnvelope {
        TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: envelope_id.to_string(),
            sender_id: "did:key:zTest".to_string(),
            receiver_id: ReceiverId::One("did:arrow:leg-consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-05-31T00:00:00Z".to_string(),
            nonce: nonce.to_string(),
            sequence: None,
            signature: None,
            signing: None,
            correlation_id: None,
            reply_to: None,
            metadata: None,
            payload: event,
        }
    }

    #[test]
    fn review_id_survives_rewrapped_envelopes() {
        let event = ArrowEvent {
            arrow_code: "2:LEG:FN:PR:1:C".to_string(),
            event_id: "leg-evt-random".to_string(),
            lifecycle_id: "leg:case-1".to_string(),
            timestamp: "2026-05-31T00:00:00Z".to_string(),
            object_type: "legal_assertion".to_string(),
            object_id: "obj-case-1".to_string(),
            proof_ref: Some("proof-1".to_string()),
            ..Default::default()
        };
        let first = envelope("env-1", "n-11111111111111111111111111111111", event.clone());
        let second = envelope("env-2", "n-22222222222222222222222222222222", event);

        assert_eq!(review_id(&first), review_id(&second));
    }
}
