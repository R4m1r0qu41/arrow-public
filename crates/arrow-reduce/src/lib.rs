//! Arrow Protocol reduction layer.
//!
//! Implements `R(event_log) → disposition + canonical Action` from RFC-003 §7: a
//! deterministic function that folds a validated, append-only log of Arrow events
//! for a single lifecycle into a current **disposition** — "given everything
//! attested so far, what is this lifecycle's current standing, and what may act on
//! it?"
//!
//! The reduction reads, per event, the canonical six-field message
//! (`Class:Origin:State:Action:Finality:Confidence`) and produces a [`Disposition`]
//! label plus the canonical [`Reduction::action`] that a Consumption Adapter acts on.
//!
//! ## Precedence (interim)
//!
//! RFC-003 §7.4 leaves the normative precedence rules to a future RFC. This crate
//! implements a deliberately **safety-first** interim policy:
//!
//! - an active `BL` (Block) anywhere in the log dominates everything → [`Disposition::Blocked`];
//! - otherwise an asserted `RV` (Reversed) dominates → [`Disposition::Reversed`];
//! - otherwise the standing is that of the **latest** event in the log.
//!
//! Settlement is gated: [`Disposition::ApprovedForSettlement`] requires a
//! finality-locked (`Finality = 1`) event whose Action is `PR` and whose Confidence
//! permits action (not the `E` floor). A settled-but-challengeable `FN+0` event
//! reduces to [`Disposition::Pending`] — real, but not yet safe to act on.

use arrow_core::parser::message::parse;
use arrow_core::types::event::ArrowEvent;
use arrow_core::types::message::ArrowMessage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Derived lifecycle standing. Illustrative vocabulary per RFC-003 §3 — these are
/// derived labels, not Arrow State values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    /// Finality-locked, action-permitting: safe for a Consumption Adapter to settle.
    ApprovedForSettlement,
    /// Real but not yet actionable — `FN+0` (settled-but-challengeable), `Hold`, or
    /// a `Trigger` awaiting its downstream condition.
    Pending,
    /// Requires review before any downstream action (`RW`, or an action-incapable
    /// Confidence floor).
    Quarantined,
    /// Hard-blocked — an active `BL` dominates the lifecycle.
    Blocked,
    /// Reversed by a subsequent authority action (`RV`).
    Reversed,
    /// Lifecycle closed; no further actions permitted (`CL`).
    Closed,
}

impl Disposition {
    /// Whether this disposition authorises a Consumption Adapter to settle.
    pub fn permits_settlement(self) -> bool {
        matches!(self, Disposition::ApprovedForSettlement)
    }
}

/// The result of reducing one lifecycle's event log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reduction {
    /// The lifecycle these events belong to.
    pub lifecycle_id: String,
    /// Derived standing.
    pub disposition: Disposition,
    /// Canonical Action that a Consumption Adapter acts on (`PR`/`BL`/`HL`/`RW`/`TR`/`CL`).
    pub action: String,
    /// Human-legible explanation of why this disposition was reached.
    pub reason: String,
    /// Number of events folded.
    pub event_count: usize,
}

/// Errors that prevent reduction.
#[derive(Debug, Error)]
pub enum ReduceError {
    /// The log was empty — nothing to reduce.
    #[error("cannot reduce an empty event log")]
    EmptyLog,
    /// Events from more than one lifecycle were supplied.
    #[error("mixed lifecycles in log: expected `{expected}`, found `{found}`")]
    MixedLifecycles { expected: String, found: String },
    /// An event carried an unparseable `arrow_code`.
    #[error("event `{event_id}` has invalid arrow_code `{code}`: {source}")]
    BadCode {
        event_id: String,
        code: String,
        #[source]
        source: arrow_core::errors::ParseError,
    },
}

/// Reduce a single lifecycle's append-only event log to its current disposition.
///
/// `events` are assumed to be chronologically ordered and to share one
/// `lifecycle_id`. Each event must already have passed Arrow validation (RFC-003
/// §7.2 — reduction operates over a *validated* log).
pub fn reduce(events: &[ArrowEvent]) -> Result<Reduction, ReduceError> {
    let first = events.first().ok_or(ReduceError::EmptyLog)?;
    let lifecycle_id = first.lifecycle_id.clone();

    // Parse every event up front; enforce single-lifecycle.
    let mut parsed: Vec<(&ArrowEvent, ArrowMessage)> = Vec::with_capacity(events.len());
    for ev in events {
        if ev.lifecycle_id != lifecycle_id {
            return Err(ReduceError::MixedLifecycles {
                expected: lifecycle_id,
                found: ev.lifecycle_id.clone(),
            });
        }
        let msg = parse(&ev.arrow_code).map_err(|source| ReduceError::BadCode {
            event_id: ev.event_id.clone(),
            code: ev.arrow_code.clone(),
            source,
        })?;
        parsed.push((ev, msg));
    }

    // Safety-first precedence: a Block dominates, then a Reversal, else the latest.
    if let Some((_, msg)) = parsed.iter().find(|(_, m)| m.action.as_str() == "BL") {
        return Ok(Reduction {
            lifecycle_id,
            disposition: Disposition::Blocked,
            action: msg.action.as_str().to_string(),
            reason: "an active BL (Block) dominates the lifecycle".to_string(),
            event_count: events.len(),
        });
    }
    if let Some((_, msg)) = parsed.iter().find(|(_, m)| m.state.as_str() == "RV") {
        return Ok(Reduction {
            lifecycle_id,
            disposition: Disposition::Reversed,
            action: msg.action.as_str().to_string(),
            reason: "the lifecycle has been reversed (RV)".to_string(),
            event_count: events.len(),
        });
    }

    let (_, latest) = parsed
        .last()
        .expect("non-empty log guaranteed by EmptyLog check");
    let (disposition, reason) = standing_of(latest);

    Ok(Reduction {
        lifecycle_id,
        disposition,
        action: latest.action.as_str().to_string(),
        reason,
        event_count: events.len(),
    })
}

/// Derive the standing of a single event from its canonical message fields.
fn standing_of(msg: &ArrowMessage) -> (Disposition, String) {
    let action = msg.action.as_str();
    let confidence = msg.confidence.as_str();
    let action_capable = confidence != "E";

    match action {
        "CL" => (Disposition::Closed, "lifecycle closed (CL)".to_string()),
        "RW" => (
            Disposition::Quarantined,
            "review warranted before downstream action (RW)".to_string(),
        ),
        "HL" => (Disposition::Pending, "held pending review (HL)".to_string()),
        "TR" => (
            Disposition::Pending,
            "trigger awaiting downstream condition (TR)".to_string(),
        ),
        "PR" => {
            if !action_capable {
                (
                    Disposition::Quarantined,
                    "Action PR but Confidence floor (E) cannot authorise action".to_string(),
                )
            } else if msg.finality == 1 {
                (
                    Disposition::ApprovedForSettlement,
                    format!("finality-locked (FN+1) proceed at Confidence {confidence}"),
                )
            } else {
                (
                    Disposition::Pending,
                    "settled but challengeable (FN+0) — not yet finality-locked".to_string(),
                )
            }
        }
        other => (
            Disposition::Quarantined,
            format!("unrecognised action `{other}` — quarantined for review"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(arrow_code: &str, lifecycle_id: &str) -> ArrowEvent {
        ArrowEvent {
            arrow_code: arrow_code.to_string(),
            event_id: format!("evt-{arrow_code}"),
            lifecycle_id: lifecycle_id.to_string(),
            timestamp: "2026-05-31T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xabc".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn final_proceed_is_approved() {
        let r = reduce(&[event("1:ETH:FN:PR:1:A", "life-1")]).unwrap();
        assert_eq!(r.disposition, Disposition::ApprovedForSettlement);
        assert!(r.disposition.permits_settlement());
        assert_eq!(r.action, "PR");
    }

    #[test]
    fn pending_finality_is_not_settleable() {
        let r = reduce(&[event("1:ETH:PD:HL:0:B", "life-1")]).unwrap();
        assert_eq!(r.disposition, Disposition::Pending);
        assert!(!r.disposition.permits_settlement());
    }

    #[test]
    fn fn0_proceed_is_pending_not_approved() {
        // settled but challengeable — must not be settled yet.
        let r = reduce(&[event("1:ETH:FN:PR:0:B", "life-1")]).unwrap();
        assert_eq!(r.disposition, Disposition::Pending);
    }

    #[test]
    fn block_dominates_a_prior_proceed() {
        let log = [
            event("1:ETH:FN:PR:1:A", "life-1"),
            event("4:PCY:FN:BL:1:B", "life-1"),
        ];
        let r = reduce(&log).unwrap();
        assert_eq!(r.disposition, Disposition::Blocked);
        assert!(!r.disposition.permits_settlement());
    }

    #[test]
    fn reversal_dominates_when_no_block() {
        let log = [
            event("1:ETH:FN:PR:1:A", "life-1"),
            event("1:ETH:RV:RW:1:B", "life-1"),
        ];
        let r = reduce(&log).unwrap();
        assert_eq!(r.disposition, Disposition::Reversed);
    }

    #[test]
    fn latest_event_wins_absent_block_or_reversal() {
        // pending first, then a later finality-locked proceed.
        let log = [
            event("1:ETH:PD:HL:0:B", "life-1"),
            event("1:ETH:FN:PR:1:A", "life-1"),
        ];
        let r = reduce(&log).unwrap();
        assert_eq!(r.disposition, Disposition::ApprovedForSettlement);
    }

    #[test]
    fn empty_log_errors() {
        assert!(matches!(reduce(&[]), Err(ReduceError::EmptyLog)));
    }

    #[test]
    fn mixed_lifecycles_error() {
        let log = [
            event("1:ETH:FN:PR:1:A", "life-1"),
            event("1:ETH:FN:PR:1:A", "life-2"),
        ];
        assert!(matches!(
            reduce(&log),
            Err(ReduceError::MixedLifecycles { .. })
        ));
    }
}
