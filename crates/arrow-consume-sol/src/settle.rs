//! Settlement stub for the Solana adapter.
//!
//! Live Solana settlement is **out of scope for v0.1** — no instruction
//! building, keypair signing, or transaction send. This module is a placeholder
//! so the crate surface matches other consumer adapters; it is never called
//! because the consumer hardwires `RecordOnly` and explicitly rejects `Execute`.
