//! EVM-family Origin Adapter library for the Arrow Protocol.
//!
//! One implementation serves every EVM-compatible chain in the Chain Registry.
//! A concrete chain adapter (e.g. `arrow-origin-eth`, and the Base/Arbitrum
//! wrappers) is a thin shell over an [`EvmChainConfig`]:
//!
//! - [`config`] — the per-chain, registry-derived [`EvmChainConfig`].
//! - [`mapping`] — the pure, chain-free confirmation→Arrow-code core.
//! - [`observer`] — the JSON-RPC chain reader ([`EvmObserver`]).
//! - [`emit`] — wrap, sign, and POST the event to a node.

pub mod config;
pub mod emit;
pub mod mapping;
pub mod observer;

pub use config::{EvmChainConfig, EvmConfigError, EvmRegime};
pub use mapping::{map_to_arrow, try_map_to_arrow, ObservedTx, ETH_REORG_SAFETY_CONFIRMATIONS};
pub use observer::EvmObserver;
