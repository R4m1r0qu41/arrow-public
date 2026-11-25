//! Envelope build/sign/POST for the Ethereum adapter.
//!
//! The signing + transport path is chain-agnostic and lives in
//! [`arrow_origin_evm::emit`]; this module re-exports it so the historical
//! `arrow_origin_eth::emit::*` paths keep working.

pub use arrow_origin_evm::emit::{
    build_signed_envelope, emit_event, post_envelope, EmitOutcome, CHANNEL_BINDING,
};
