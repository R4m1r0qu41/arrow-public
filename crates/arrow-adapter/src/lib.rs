//! Shared adapter boundary for Arrow origin and consumer adapters.
//!
//! This crate keeps adapter-to-adapter semantics out of concrete chain adapters:
//! typed action intents, normalized interchange, consumer receipts, provenance
//! verification, destination idempotency, dry-run/record-only modes, and reusable
//! conformance assertions.

pub mod closure;
pub mod conformance;
pub mod contract;
pub mod idempotency;
pub mod intent;
pub mod provenance;
pub mod receipt;
pub mod traits;

pub use contract::{AdapterInfo, AdapterInterchangeContract, AdapterKind};
pub use idempotency::{
    destination_execution_id, DestinationExecutionStore, InMemoryDestinationExecutionStore,
};
pub use intent::{
    ActionIntent, ActionKind, Asset, Destination, DestinationMismatchError, IntentAmount,
    SourceValueTransfer,
};
pub use provenance::{verify_envelope_provenance, ProvenancePolicy, ProvenanceVerification};
pub use receipt::{ConsumerExecutionMode, ConsumerReceipt, ConsumerReceiptStatus};
pub use traits::{ConsumerAdapter, OriginAdapter};
