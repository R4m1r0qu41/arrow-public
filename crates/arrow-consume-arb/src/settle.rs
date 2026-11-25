//! Settlement re-export for the Arbitrum adapter.
//!
//! Live settlement is **out of scope for v0.1** — the BoLD challenge window
//! must elapse before any Arbitrum transfer is safe to execute. This module
//! re-exports the shared EVM settlement path for future use; it is never
//! called in v0.1 (RecordOnly mode).

pub use arrow_consume_evm::settle::{send_settlement, SettleParams};
