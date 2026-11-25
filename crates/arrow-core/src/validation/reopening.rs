//! Shared post-closure reopening predicates (LFC-007 / RFC-010 Part 2).
//!
//! ONE home for the shapes that may follow a closure, consumed by both the
//! Arrow gate (LFC-007 in the event pipeline) and downstream consumers
//! (Tarquin LC-013/LC-015). Keeping the predicate here means the gate and
//! the consumer cannot drift when the dormant subtype clause is promoted
//! (SEM-021 / `control_subtypes.json`): both sides change together.
//!
//! Interim origin constraint (post-review hardening, 2026-07-07): until
//! SEM-021 lands, a reopening MUST share the closure's Origin — except a
//! supersession from `PCY`, the protocol/governance origin, whose whole
//! source domain is the control plane Class 4 speaks for. Without this, ANY
//! Class 4 event from ANY origin citing the closure could reopen any
//! lifecycle — far wider than the reopening-is-rare model assumes.

use crate::types::event::ArrowEvent;

/// The protocol/governance origin: the one source domain whose CONTROL
/// events may supersede closures it did not observe.
pub const GOVERNANCE_ORIGIN: &str = "PCY";

/// Parse the (class, origin, state) triple of an event's arrow code once.
/// Returns None for unparseable codes (which fail validation elsewhere).
fn shape_of(e: &ArrowEvent) -> Option<(u8, String, String)> {
    crate::parser::message::parse(&e.arrow_code).ok().map(|m| {
        (
            m.class.as_u8(),
            m.origin.as_str().to_string(),
            m.state.as_str().to_string(),
        )
    })
}

/// A Class 6 closure (necessarily `6:*:FN:CL:1:*` once validated).
pub fn is_closure(e: &ArrowEvent) -> bool {
    shape_of(e).map(|(c, _, _)| c == 6).unwrap_or(false)
}

/// Reopening shape 1 — Class 1, State RV reversal.
pub fn is_reversal(e: &ArrowEvent) -> bool {
    shape_of(e)
        .map(|(c, _, st)| c == 1 && st == "RV")
        .unwrap_or(false)
}

/// Reopening shape 2 — Class 4 (CONTROL) supersession. The
/// `control_subtype = SUPERSESSION` clause is dormant until SEM-021.
pub fn is_supersession(e: &ArrowEvent) -> bool {
    shape_of(e).map(|(c, _, _)| c == 4).unwrap_or(false)
}

/// Full LFC-007 reopening test: is `e` a valid follow-up to `closure`?
///
/// Valid iff: it is one of the two reopening shapes, it cites the closure's
/// `event_id` specifically in `related_events`, and it satisfies the interim
/// origin constraint (same Origin as the closure; a supersession may instead
/// come from [`GOVERNANCE_ORIGIN`]).
pub fn is_valid_reopening(e: &ArrowEvent, closure: &ArrowEvent) -> bool {
    let Some((class, origin, state)) = shape_of(e) else {
        return false;
    };
    let closure_origin = match shape_of(closure) {
        Some((_, o, _)) => o,
        None => return false,
    };
    let reversal = class == 1 && state == "RV";
    let supersession = class == 4;
    if !reversal && !supersession {
        return false;
    }
    if !e.related_events.contains(&closure.event_id) {
        return false;
    }
    origin == closure_origin || (supersession && origin == GOVERNANCE_ORIGIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(code: &str, id: &str, related: &[&str]) -> ArrowEvent {
        ArrowEvent {
            arrow_code: code.to_string(),
            event_id: id.to_string(),
            lifecycle_id: "lfc".to_string(),
            timestamp: "2026-07-07T00:00:00Z".to_string(),
            object_type: "t".to_string(),
            object_id: "o".to_string(),
            related_events: related.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn same_origin_reversal_reopens() {
        let cl = ev("6:ETH:FN:CL:1:A", "cl-1", &[]);
        assert!(is_valid_reopening(
            &ev("1:ETH:RV:HL:1:A", "rv-1", &["cl-1"]),
            &cl
        ));
    }

    #[test]
    fn cross_origin_reversal_is_refused() {
        let cl = ev("6:ETH:FN:CL:1:A", "cl-1", &[]);
        assert!(!is_valid_reopening(
            &ev("1:BSE:RV:HL:1:B", "rv-x", &["cl-1"]),
            &cl
        ));
    }

    #[test]
    fn governance_supersession_is_permitted_cross_origin() {
        let cl = ev("6:ETH:FN:CL:1:A", "cl-1", &[]);
        assert!(is_valid_reopening(
            &ev("4:PCY:FN:HL:1:B", "sup-1", &["cl-1"]),
            &cl
        ));
    }

    #[test]
    fn non_governance_cross_origin_supersession_is_refused() {
        let cl = ev("6:ETH:FN:CL:1:A", "cl-1", &[]);
        assert!(!is_valid_reopening(
            &ev("4:CST:FN:HL:1:C", "sup-x", &["cl-1"]),
            &cl
        ));
    }

    #[test]
    fn uncited_reopening_is_refused() {
        let cl = ev("6:ETH:FN:CL:1:A", "cl-1", &[]);
        assert!(!is_valid_reopening(
            &ev("1:ETH:RV:HL:1:A", "rv-1", &[]),
            &cl
        ));
    }
}
