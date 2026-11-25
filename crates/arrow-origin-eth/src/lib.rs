//! Ethereum Origin Adapter for the Arrow Protocol.
//!
//! Observes a transaction on an EVM chain, maps confirmation depth to Arrow
//! finality/confidence, mints a protocol-valid [`arrow_core::types::event::ArrowEvent`],
//! and (via [`emit`]) signs it into a transport envelope and POSTs it to an Arrow node.
//!
//! - [`mapping`] — the pure, chain-free confirmation→Arrow-code core.
//! - [`observer`] — the `alloy`-backed chain reader.
//! - [`emit`] — wrap, sign, and POST the event to a node.

pub mod emit;
pub mod mapping;
pub mod observer;
