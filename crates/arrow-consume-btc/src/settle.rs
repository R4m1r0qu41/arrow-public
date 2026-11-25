//! Settlement stub for the Bitcoin adapter.
//!
//! Live Bitcoin settlement is **out of scope for v0.1** — no UTXO selection,
//! PSBT construction, signing, or broadcast. This module is a placeholder so
//! the crate surface matches other consumer adapters; it is never called since
//! the consumer hardwires `RecordOnly` and explicitly rejects `Execute` mode.
