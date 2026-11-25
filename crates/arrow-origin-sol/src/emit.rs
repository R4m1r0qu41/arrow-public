//! Envelope build/sign/POST for the Solana adapter.
//!
//! The signing and transport path is chain-agnostic; re-exported from
//! [`arrow_origin_evm::emit`] so all adapters share the same surface.

pub use arrow_origin_evm::emit::{
    build_signed_envelope, emit_event, post_envelope, EmitOutcome, CHANNEL_BINDING,
};
