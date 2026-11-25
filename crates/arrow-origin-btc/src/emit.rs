//! Envelope build/sign/POST for the Bitcoin adapter.
//!
//! The signing and transport path is chain-agnostic and lives in
//! [`arrow_origin_evm::emit`]; this re-exports it so Bitcoin callers share
//! the same surface as all other adapters.

pub use arrow_origin_evm::emit::{
    build_signed_envelope, emit_event, post_envelope, EmitOutcome, CHANNEL_BINDING,
};
