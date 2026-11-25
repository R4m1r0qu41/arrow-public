//! Settlement re-export for the Base adapter.
//!
//! Live settlement is **out of scope for v0.1** — the challenge window must
//! elapse and be confirmed by a settlement watcher before any Base transfer
//! is safe to execute. This module re-exports the shared EVM settlement path
//! for future use; it is never called in v0.1 (RecordOnly mode).

pub use arrow_consume_evm::settle::{send_settlement, SettleParams};
