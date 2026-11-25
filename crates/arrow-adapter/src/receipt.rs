use arrow_reduce::Reduction;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{intent::ActionIntent, provenance::ProvenanceVerification};

pub const CONSUMER_RECEIPT_VERSION: &str = "arrow-consumer-receipt-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerExecutionMode {
    Execute,
    DryRun,
    RecordOnly,
}

impl ConsumerExecutionMode {
    pub fn records_idempotency(self) -> bool {
        matches!(
            self,
            ConsumerExecutionMode::Execute | ConsumerExecutionMode::RecordOnly
        )
    }

    pub fn executes_destination(self) -> bool {
        matches!(self, ConsumerExecutionMode::Execute)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerReceiptStatus {
    NoAction,
    DryRun,
    RecordedOnly,
    Executed,
    /// The lifecycle was routed to a review/quarantine state instead of an
    /// autonomous destination action (RFC-005 §8.3 `quarantined`, CA-011 "routed
    /// for review"). Used by adapters whose host-system action is a human-review
    /// hand-off rather than a value transfer, e.g. the LEG consumer. A quarantined
    /// receipt carries no typed `action_intent`.
    Quarantined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerReceipt {
    pub receipt_version: String,
    pub consumer_adapter: String,
    pub mode: ConsumerExecutionMode,
    pub status: ConsumerReceiptStatus,
    pub envelope_id: String,
    pub event_id: String,
    pub lifecycle_id: String,
    pub disposition: String,
    pub action: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub action_intent: Option<ActionIntent>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub destination_execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub destination_tx: Option<String>,
    pub provenance: ProvenanceVerification,
    pub recorded_at: String,
}

impl ConsumerReceipt {
    pub fn new(
        consumer_adapter: impl Into<String>,
        mode: ConsumerExecutionMode,
        status: ConsumerReceiptStatus,
        envelope_id: impl Into<String>,
        event_id: impl Into<String>,
        reduction: &Reduction,
        provenance: ProvenanceVerification,
    ) -> Self {
        Self {
            receipt_version: CONSUMER_RECEIPT_VERSION.to_string(),
            consumer_adapter: consumer_adapter.into(),
            mode,
            status,
            envelope_id: envelope_id.into(),
            event_id: event_id.into(),
            lifecycle_id: reduction.lifecycle_id.clone(),
            disposition: format!("{:?}", reduction.disposition),
            action: reduction.action.clone(),
            reason: reduction.reason.clone(),
            action_intent: None,
            destination_execution_id: None,
            destination_tx: None,
            provenance,
            recorded_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn with_action(
        mut self,
        intent: ActionIntent,
        destination_execution_id: Option<String>,
        destination_tx: Option<String>,
    ) -> Self {
        self.action_intent = Some(intent);
        self.destination_execution_id = destination_execution_id;
        self.destination_tx = destination_tx;
        self
    }
}
