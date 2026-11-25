#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationState {
    Active,
    Conditional,
    Halted,
    Degraded,
    Restored,
}

#[derive(Debug, Clone)]
pub struct ChainObservation {
    pub chain_id: String,
    pub state: ObservationState,
    pub last_transition_event_id: Option<String>,
    pub confidence_suspended: bool,
}
