//! Arbitrum One Origin Adapter for the Arrow Protocol.
//!
//! A thin [`arrow_origin_evm::EvmChainConfig::arb()`] shell over the shared
//! EVM-family library. Arbitrum One is an optimistic L2 (BoLD) on Ethereum;
//! the observation and finality logic is the same as Base — only the origin
//! code (`ARB`) and chain id (`ARB`) differ. All v0.1 events are minted as
//! `1:ARB:PD:HL:0:B` with `finality_basis = optimistic_window` until a
//! settlement watcher (later phase) promotes them after the 7-day window.
//!
//! - [`mapping`] — `ObservedTx` → [`arrow_core::types::event::ArrowEvent`]
//!   via fail-closed [`try_map_to_arrow`](mapping::try_map_to_arrow).
//! - [`emit`] — wrap, sign, and POST the event to a node.
//! - [`observer`] — EVM JSON-RPC chain reader aliased as `ArbObserver`.

pub mod emit;
pub mod mapping;
pub mod observer;
