//! Base chain reader.
//!
//! `BaseObserver` is the shared [`arrow_origin_evm::observer::EvmObserver`]
//! under its chain-specific alias; the read-only JSON-RPC logic works against
//! any EVM-compatible endpoint.

pub use arrow_origin_evm::observer::EvmObserver as BaseObserver;
