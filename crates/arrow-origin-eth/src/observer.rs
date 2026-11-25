//! Ethereum chain reader.
//!
//! `EthObserver` is the shared [`arrow_origin_evm::observer::EvmObserver`] under
//! its historical name; the read-only JSON-RPC logic is chain-agnostic and lives
//! in the EVM-family library.

pub use arrow_origin_evm::observer::EvmObserver as EthObserver;
