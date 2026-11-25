//! Base (Coinbase L2) Origin Adapter for the Arrow Protocol.
//!
//! A thin [`arrow_origin_evm::EvmChainConfig::base()`] shell over the shared
//! EVM-family library. Base is an optimistic rollup on Ethereum; the
//! observation and finality logic is the same as Arbitrum — only the origin
//! code (`BSE`) and chain id (`BASE`) differ. All v0.1 events are minted as
//! `1:BSE:PD:HL:0:B` with `finality_basis = optimistic_window` until a
//! settlement watcher (later phase) promotes them after the 7-day window.
//!
//! - [`mapping`] — `ObservedTx` → [`arrow_core::types::event::ArrowEvent`]
//!   via fail-closed [`try_map_to_arrow`](mapping::try_map_to_arrow).
//! - [`emit`] — wrap, sign, and POST the event to a node.
//! - [`observer`] — EVM JSON-RPC chain reader aliased as `BaseObserver`.

pub mod emit;
pub mod mapping;
pub mod observer;
