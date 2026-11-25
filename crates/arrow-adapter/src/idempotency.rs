use std::collections::HashSet;
use std::sync::Mutex;

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::intent::{ActionIntent, ActionKind, Asset, Destination, IntentAmount};

#[derive(Debug, Error)]
pub enum DestinationExecutionError {
    #[error("destination execution store lock poisoned")]
    LockPoisoned,
    #[error("destination execution store I/O error: {0}")]
    Io(String),
}

/// Idempotency state for destination-side execution, distinct from envelope replay.
pub trait DestinationExecutionStore: Send + Sync {
    /// Returns `true` if `destination_execution_id` was already recorded.
    /// If not seen, records it and returns `false`.
    fn check_and_record_execution(
        &self,
        destination_execution_id: &str,
    ) -> Result<bool, DestinationExecutionError>;
}

#[derive(Debug, Default)]
pub struct InMemoryDestinationExecutionStore {
    seen: Mutex<HashSet<String>>,
}

impl InMemoryDestinationExecutionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DestinationExecutionStore for InMemoryDestinationExecutionStore {
    fn check_and_record_execution(
        &self,
        destination_execution_id: &str,
    ) -> Result<bool, DestinationExecutionError> {
        let mut guard = self
            .seen
            .lock()
            .map_err(|_| DestinationExecutionError::LockPoisoned)?;
        if guard.contains(destination_execution_id) {
            return Ok(true);
        }
        guard.insert(destination_execution_id.to_string());
        Ok(false)
    }
}

#[derive(Serialize)]
struct DestinationExecutionPreimage<'a> {
    contract: &'static str,
    intent_id: &'a str,
    action: &'a ActionKind,
    destination: &'a Destination,
    asset: &'a Asset,
    amount: &'a IntentAmount,
}

/// Stable destination-execution id used to prevent duplicated host-system action.
///
/// This intentionally excludes transport-envelope ids and nonces. Envelope replay
/// protection catches the same signed envelope; destination idempotency must also
/// catch the same logical action rewrapped in a fresh envelope.
pub fn destination_execution_id(intent: &ActionIntent) -> String {
    let preimage = DestinationExecutionPreimage {
        contract: "arrow-destination-execution-id-v1",
        intent_id: &intent.intent_id,
        action: &intent.action,
        destination: &intent.destination,
        asset: &intent.asset,
        amount: &intent.amount,
    };
    let bytes = serde_json::to_vec(&preimage).expect("serializing idempotency preimage");
    let digest = Sha256::digest(bytes);
    format!("dx-{}", hex::encode(digest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionIntent;

    #[test]
    fn destination_execution_id_is_stable_for_same_logical_action() {
        let intent = ActionIntent::evm_value_transfer(
            "intent:evm-transfer:ETH:0xabc:0x222:100",
            "ETH",
            "0x222",
            "100",
            "evt-random-1",
            Some("0xabc".to_string()),
        );
        let mut reminted = intent.clone();
        reminted.source_event_id = "evt-random-2".to_string();

        assert_eq!(
            destination_execution_id(&intent),
            destination_execution_id(&reminted)
        );
    }
}
