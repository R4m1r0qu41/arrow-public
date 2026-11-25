//! Envelope build/sign/POST for the Arbitrum adapter.
//!
//! The signing + transport path is chain-agnostic and lives in
//! [`arrow_origin_evm::emit`]; this re-exports it so Arbitrum callers share
//! the same surface as all other EVM-family adapters.

pub use arrow_origin_evm::emit::{
    build_signed_envelope, emit_event, post_envelope, EmitOutcome, CHANNEL_BINDING,
};
