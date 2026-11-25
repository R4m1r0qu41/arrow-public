//! Envelope build/sign/POST for the Base adapter.
//!
//! The signing + transport path is chain-agnostic and lives in
//! [`arrow_origin_evm::emit`]; this re-exports it so Base callers share the
//! same surface as all other EVM-family adapters. Envelopes are emitted on
//! the `http-rest-local` channel binding (the legacy `http-rest` is inactive).

pub use arrow_origin_evm::emit::{
    build_signed_envelope, emit_event, post_envelope, EmitOutcome, CHANNEL_BINDING,
};
