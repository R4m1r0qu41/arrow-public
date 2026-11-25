use crate::{
    errors::StateMachineError,
    types::{
        event::ArrowEvent,
        observation::{ChainObservation, ObservationState},
    },
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub chain_id: String,
    pub from: ObservationState,
    pub to: ObservationState,
    pub trigger_event_id: String,
}

pub struct ObservationStateMachine {
    states: HashMap<String, ChainObservation>,
}

impl ObservationStateMachine {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    pub fn apply(
        &mut self,
        event: &ArrowEvent,
    ) -> Result<Option<StateTransition>, StateMachineError> {
        let Some(chain_id) = event.chain_id.clone() else {
            return Ok(None);
        };
        let current = self
            .states
            .entry(chain_id.clone())
            .or_insert(ChainObservation {
                chain_id: chain_id.clone(),
                state: ObservationState::Active,
                last_transition_event_id: None,
                confidence_suspended: false,
            });
        let next = match event.reason_code.as_str() {
            "DIVERGENCE_DETECTED" | "OBSERVATION_CONDITIONAL" => ObservationState::Conditional,
            "OBSERVATION_HALTED" => ObservationState::Halted,
            "OBSERVATION_DEGRADED" => ObservationState::Degraded,
            "OBSERVATION_RESTORED" => ObservationState::Restored,
            "DIVERGENCE_RESOLVED" => ObservationState::Active,
            _ => return Ok(None),
        };
        if matches!(current.state, ObservationState::Halted)
            && matches!(
                next,
                ObservationState::Conditional | ObservationState::Degraded
            )
        {
            return Err(StateMachineError::InvalidTransition {
                chain_id,
                from: "Halted".to_string(),
                to: format!("{:?}", next),
                event_id: event.event_id.clone(),
            });
        }
        if event.reason_code == "DIVERGENCE_RESOLVED"
            && current.state != ObservationState::Conditional
        {
            return Err(StateMachineError::InvalidTransition {
                chain_id,
                from: format!("{:?}", current.state),
                to: "Active".to_string(),
                event_id: event.event_id.clone(),
            });
        }
        let transition = StateTransition {
            chain_id: chain_id.clone(),
            from: current.state.clone(),
            to: next.clone(),
            trigger_event_id: event.event_id.clone(),
        };
        current.state = next;
        current.last_transition_event_id = Some(event.event_id.clone());
        current.confidence_suspended = matches!(
            current.state,
            ObservationState::Conditional | ObservationState::Halted | ObservationState::Degraded
        );
        Ok(Some(transition))
    }

    pub fn state_for(&self, chain_id: &str) -> Option<&ChainObservation> {
        self.states.get(chain_id)
    }
}

impl Default for ObservationStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(chain_id: Option<&str>, reason_code: &str, event_id: &str) -> ArrowEvent {
        ArrowEvent {
            event_id: event_id.to_string(),
            chain_id: chain_id.map(str::to_string),
            reason_code: reason_code.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn ignores_events_without_chain_id() {
        let mut machine = ObservationStateMachine::default();

        let transition = machine
            .apply(&event(None, "DIVERGENCE_DETECTED", "evt-1"))
            .expect("event without chain id should not fail");

        assert!(transition.is_none());
        assert!(machine.state_for("ETH").is_none());
    }

    #[test]
    fn unknown_reason_code_creates_no_transition() {
        let mut machine = ObservationStateMachine::default();

        let transition = machine
            .apply(&event(Some("ETH"), "UNRELATED_REASON", "evt-1"))
            .expect("unknown reason should not fail");

        assert!(transition.is_none());
        let state = machine
            .state_for("ETH")
            .expect("chain state is initialized before reason filtering");
        assert_eq!(state.state, ObservationState::Active);
        assert_eq!(state.last_transition_event_id, None);
        assert!(!state.confidence_suspended);
    }

    #[test]
    fn divergence_detected_moves_active_chain_to_conditional() {
        let mut machine = ObservationStateMachine::default();

        let transition = machine
            .apply(&event(Some("ETH"), "DIVERGENCE_DETECTED", "evt-1"))
            .expect("conditional transition should succeed")
            .expect("recognized reason should produce transition");

        assert_eq!(transition.chain_id, "ETH");
        assert_eq!(transition.from, ObservationState::Active);
        assert_eq!(transition.to, ObservationState::Conditional);
        assert_eq!(transition.trigger_event_id, "evt-1");

        let state = machine
            .state_for("ETH")
            .expect("transition should initialize chain state");
        assert_eq!(state.chain_id, "ETH");
        assert_eq!(state.state, ObservationState::Conditional);
        assert_eq!(state.last_transition_event_id.as_deref(), Some("evt-1"));
        assert!(state.confidence_suspended);
    }

    #[test]
    fn conditional_chain_can_resolve_to_active() {
        let mut machine = ObservationStateMachine::default();
        machine
            .apply(&event(Some("ETH"), "OBSERVATION_CONDITIONAL", "evt-1"))
            .expect("conditional transition should succeed");

        let transition = machine
            .apply(&event(Some("ETH"), "DIVERGENCE_RESOLVED", "evt-2"))
            .expect("resolution from conditional should succeed")
            .expect("resolution should produce transition");

        assert_eq!(transition.from, ObservationState::Conditional);
        assert_eq!(transition.to, ObservationState::Active);

        let state = machine
            .state_for("ETH")
            .expect("state should remain tracked");
        assert_eq!(state.state, ObservationState::Active);
        assert_eq!(state.last_transition_event_id.as_deref(), Some("evt-2"));
        assert!(!state.confidence_suspended);
    }

    #[test]
    fn halted_chain_rejects_lower_severity_transitions() {
        let mut machine = ObservationStateMachine::default();
        machine
            .apply(&event(Some("ETH"), "OBSERVATION_HALTED", "evt-1"))
            .expect("halt transition should succeed");

        let err = machine
            .apply(&event(Some("ETH"), "OBSERVATION_DEGRADED", "evt-2"))
            .expect_err("halted chain should reject degraded transition");

        match err {
            StateMachineError::InvalidTransition {
                chain_id,
                from,
                to,
                event_id,
            } => {
                assert_eq!(chain_id, "ETH");
                assert_eq!(from, "Halted");
                assert_eq!(to, "Degraded");
                assert_eq!(event_id, "evt-2");
            }
        }

        let err = machine
            .apply(&event(Some("ETH"), "OBSERVATION_CONDITIONAL", "evt-3"))
            .expect_err("halted chain should reject conditional transition");

        match err {
            StateMachineError::InvalidTransition { to, event_id, .. } => {
                assert_eq!(to, "Conditional");
                assert_eq!(event_id, "evt-3");
            }
        }
    }

    #[test]
    fn divergence_resolved_is_valid_only_from_conditional() {
        let mut machine = ObservationStateMachine::default();

        let err = machine
            .apply(&event(Some("ETH"), "DIVERGENCE_RESOLVED", "evt-1"))
            .expect_err("active chain cannot resolve divergence");

        match err {
            StateMachineError::InvalidTransition {
                chain_id,
                from,
                to,
                event_id,
            } => {
                assert_eq!(chain_id, "ETH");
                assert_eq!(from, "Active");
                assert_eq!(to, "Active");
                assert_eq!(event_id, "evt-1");
            }
        }
    }

    #[test]
    fn degraded_halted_and_restored_states_update_confidence_suspension() {
        let mut machine = ObservationStateMachine::default();

        machine
            .apply(&event(Some("ETH"), "OBSERVATION_DEGRADED", "evt-1"))
            .expect("degraded transition should succeed");
        let state = machine.state_for("ETH").expect("state should be tracked");
        assert_eq!(state.state, ObservationState::Degraded);
        assert!(state.confidence_suspended);

        machine
            .apply(&event(Some("ETH"), "OBSERVATION_HALTED", "evt-2"))
            .expect("halted transition should succeed");
        let state = machine.state_for("ETH").expect("state should be tracked");
        assert_eq!(state.state, ObservationState::Halted);
        assert!(state.confidence_suspended);

        machine
            .apply(&event(Some("ETH"), "OBSERVATION_RESTORED", "evt-3"))
            .expect("restored transition should succeed");
        let state = machine.state_for("ETH").expect("state should be tracked");
        assert_eq!(state.state, ObservationState::Restored);
        assert!(!state.confidence_suspended);
    }
}
