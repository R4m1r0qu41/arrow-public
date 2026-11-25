//! Arrow Identity — DID-based identity resolution and signature verification.
//!
//! This crate provides:
//! - DID resolution (`did:key` offline, `did:web` via HTTP)
//! - Ed25519 signature verification
//! - A generic `verify_signature` function over arbitrary bytes
//!
//! It operates on raw bytes and signing metadata — **not** on transport-specific
//! types. The integration layer (arrow-node, arrow-cli) bridges Arrow envelopes
//! to identity verification.

pub mod crypto;
pub mod error;
pub mod resolver;
pub mod types;
pub mod verify;
