//! Bitcoin Origin Adapter for the Arrow Protocol.
//!
//! Bitcoin is a UTXO-chain L1 with probabilistic finality at 6 confirmations.
//! This adapter is **not EVM-compatible** — it uses its own [`ObservedUtxo`]
//! observation type and derives finality from confirmation depth rather than
//! block confirmations against an EVM chain. The signing and transport path
//! re-uses the chain-agnostic [`emit`] module.
//!
//! All v0.1 events are minted as `1:BTC:PD:HL:0:B` with
//! `finality_basis = probabilistic` until 6 confirmations are observed, at which
//! point they become `1:BTC:FN:PR:1:A`. There is no challenge window: Bitcoin
//! finality is probabilistic-depth, not optimistic-rollup.
//!
//! - [`mapping`] — `ObservedUtxo` → [`arrow_core::types::event::ArrowEvent`].
//! - [`emit`] — wrap, sign, and POST the event to a node.
//! - [`observer`] — Bitcoin JSON-RPC reader producing [`mapping::ObservedUtxo`].

pub mod emit;
pub mod mapping;
pub mod observer;
