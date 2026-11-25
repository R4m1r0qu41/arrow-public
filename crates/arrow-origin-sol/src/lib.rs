//! Solana Origin Adapter for the Arrow Protocol.
//!
//! Solana is a PoH+Tower-BFT L1 with commitment-based finality — not a
//! confirmation-depth chain and not an EVM-compatible chain. Finality is
//! derived from the transaction's **commitment level**:
//!
//! | Commitment   | arrow_code          | finality_basis  |
//! |--------------|---------------------|-----------------|
//! | `finalized`  | `1:SOL:FN:PR:1:A`  | —               |
//! | `confirmed`  | `1:SOL:PD:HL:0:B`  | `probabilistic` |
//! | `processed`  | `1:SOL:PD:HL:0:B`  | `probabilistic` |
//!
//! `native_canonical` regime (editor-reclassified from the erroneous
//! `optimistic_settled`): SOL is an L1 — the optimistic-rollup regime only
//! applies to ETH L2 chains. Maximum Confidence is **A** at `finalized`;
//! `confirmed`/`processed` are capped at B.
//!
//! - [`mapping`] — `ObservedTransaction` → [`arrow_core::types::event::ArrowEvent`].
//! - [`emit`] — wrap, sign, and POST the event to a node.
//! - [`observer`] — Solana JSON-RPC reader producing [`mapping::ObservedTransaction`].

pub mod emit;
pub mod mapping;
pub mod observer;
