//! Closure emission — the origin watcher's side of Universal Eventual
//! Closure (RFC-010 Part 2A; architect rulings 2026-07-07).
//!
//! The **origin adapter / settlement watcher is the closure emitter of
//! record** for chain lifecycles, as Lex.exe is for LEG: consumers never
//! self-close lifecycles they merely consume. When a lifecycle's closure
//! horizon is reached the watcher emits the canonical terminal event
//! `6:{ORIGIN}:FN:CL:1:{confidence}` referencing the observation it
//! terminates. A lifecycle whose window lapsed **without** settlement is
//! closed as non-consummated (reason code `NON_CONSUMMATION`) — abandoned
//! lifecycles must not linger as dormant attack surface.
//!
//! Everything here is deterministic construction from caller-supplied
//! inputs (the watcher supplies its own observation time, like every
//! producer); validation of the result remains the pipeline's job.

use arrow_core::types::event::ArrowEvent;

/// Deterministic inputs for one closure emission.
#[derive(Debug, Clone)]
pub struct ClosureSpec<'a> {
    /// The event being terminated — the settled observation (consummated
    /// path) or the lapsed pending observation (non-consummated path).
    pub observation: &'a ArrowEvent,
    /// Watcher-assigned globally-unique event id for the closure.
    pub event_id: String,
    /// Watcher-supplied emission time (RFC3339).
    pub timestamp: String,
    /// Confidence of the closure claim — bounded by the origin regime's
    /// SEM-015 ceiling exactly like any other event.
    pub confidence: String,
    /// True: the lifecycle settled and its reversal surface is exhausted.
    /// False: the window lapsed unsettled — closure records
    /// non-consummation (`reason_code = NON_CONSUMMATION`).
    pub consummated: bool,
}

/// Extract the Origin (field 2) from a canonical arrow code.
fn origin_of(code: &str) -> Option<&str> {
    code.split(':').nth(1)
}

/// Build the canonical closure event for a lifecycle.
///
/// The result is a plain `ArrowEvent`; emit it through the same signed
/// transport as any other event and validate it with lifecycle context —
/// LFC-007 will reject it if the lifecycle is somehow already closed.
pub fn build_closure_event(spec: &ClosureSpec) -> ArrowEvent {
    let origin = origin_of(&spec.observation.arrow_code).unwrap_or("");
    ArrowEvent {
        arrow_code: format!("6:{origin}:FN:CL:1:{}", spec.confidence),
        event_id: spec.event_id.clone(),
        lifecycle_id: spec.observation.lifecycle_id.clone(),
        timestamp: spec.timestamp.clone(),
        object_type: spec.observation.object_type.clone(),
        object_id: spec.observation.object_id.clone(),
        chain_id: spec.observation.chain_id.clone(),
        subject_id: spec.observation.subject_id.clone(),
        subject_type: spec.observation.subject_type.clone(),
        reason_code: if spec.consummated {
            String::new()
        } else {
            "NON_CONSUMMATION".to_string()
        },
        // The closure names the event it terminates (SEM-014 symmetry:
        // terminal bookkeeping always cites its subject).
        related_events: vec![spec.observation.event_id.clone()],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::registry::loader::Registries;
    use arrow_core::validation::pipeline::{LifecycleContext, ValidationPipeline};

    fn settled_observation() -> ArrowEvent {
        ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            event_id: "evt-obs-settled".to_string(),
            lifecycle_id: "eth:0xclose1".to_string(),
            timestamp: "2026-07-07T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xclose1".to_string(),
            chain_id: Some("ETH".to_string()),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            ..Default::default()
        }
    }

    fn lapsed_pending_observation() -> ArrowEvent {
        ArrowEvent {
            arrow_code: "1:ARB:PD:HL:0:B".to_string(),
            event_id: "evt-obs-lapsed".to_string(),
            lifecycle_id: "arb:0xclose2".to_string(),
            timestamp: "2026-06-01T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xclose2".to_string(),
            chain_id: Some("ARB".to_string()),
            finality_basis: Some("optimistic_window".to_string()),
            finality_eta: Some(1_780_963_200),
            finality_eta_type: Some("unix_ts".to_string()),
            ..Default::default()
        }
    }

    fn ctx_with(prior: &ArrowEvent) -> LifecycleContext {
        let mut ctx = LifecycleContext::default();
        ctx.processed_event_ids.insert(prior.event_id.clone());
        ctx.lifecycle_events
            .entry(prior.lifecycle_id.clone())
            .or_default()
            .push(prior.clone());
        ctx
    }

    #[test]
    fn consummated_closure_validates_with_context() {
        let obs = settled_observation();
        let closure = build_closure_event(&ClosureSpec {
            observation: &obs,
            event_id: "evt-obs-settled-cl".to_string(),
            timestamp: "2026-07-08T00:00:00Z".to_string(),
            confidence: "A".to_string(),
            consummated: true,
        });
        assert_eq!(closure.arrow_code, "6:ETH:FN:CL:1:A");
        assert_eq!(closure.related_events, vec!["evt-obs-settled"]);

        let registries = Registries::load_embedded().unwrap();
        let pipeline = ValidationPipeline::new(&registries);
        let result = pipeline.validate_event_with_context(&closure, &ctx_with(&obs));
        assert!(
            result.passed,
            "closure must validate: {:?}",
            result.violations
        );
    }

    #[test]
    fn non_consummated_closure_carries_reason_and_validates() {
        let obs = lapsed_pending_observation();
        let closure = build_closure_event(&ClosureSpec {
            observation: &obs,
            event_id: "evt-obs-lapsed-cl".to_string(),
            timestamp: "2026-06-10T00:00:00Z".to_string(),
            confidence: "A".to_string(),
            consummated: false,
        });
        assert_eq!(closure.arrow_code, "6:ARB:FN:CL:1:A");
        assert_eq!(closure.reason_code, "NON_CONSUMMATION");

        let registries = Registries::load_embedded().unwrap();
        let pipeline = ValidationPipeline::new(&registries);
        let result = pipeline.validate_event_with_context(&closure, &ctx_with(&obs));
        assert!(
            result.passed,
            "non-consummated closure must validate: {:?}",
            result.violations
        );
    }

    #[test]
    fn second_closure_is_rejected_by_lfc_007() {
        let obs = settled_observation();
        let first = build_closure_event(&ClosureSpec {
            observation: &obs,
            event_id: "evt-cl-1".to_string(),
            timestamp: "2026-07-08T00:00:00Z".to_string(),
            confidence: "A".to_string(),
            consummated: true,
        });
        let mut ctx = ctx_with(&obs);
        ctx.processed_event_ids.insert(first.event_id.clone());
        ctx.lifecycle_events
            .entry(first.lifecycle_id.clone())
            .or_default()
            .push(first.clone());

        let second = build_closure_event(&ClosureSpec {
            observation: &obs,
            event_id: "evt-cl-2".to_string(),
            timestamp: "2026-07-09T00:00:00Z".to_string(),
            confidence: "A".to_string(),
            consummated: true,
        });
        let registries = Registries::load_embedded().unwrap();
        let pipeline = ValidationPipeline::new(&registries);
        let result = pipeline.validate_event_with_context(&second, &ctx);
        assert!(!result.passed, "second closure must reject");
        assert!(result.violations.iter().any(|v| v.rule_id == "LFC-007"));
    }
}
