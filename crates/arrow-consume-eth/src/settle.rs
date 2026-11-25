//! Destination-chain (chain B) settlement.
//!
//! The EVM JSON-RPC settlement path is chain-agnostic and lives in
//! [`arrow_consume_evm::settle`]; this module re-exports it so the historical
//! `arrow_consume_eth::settle::*` paths keep working.

pub use arrow_consume_evm::settle::{send_settlement, SettleParams};
