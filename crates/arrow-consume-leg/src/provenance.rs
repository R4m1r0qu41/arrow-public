//! Backward-compatible re-export of shared adapter provenance verification.
//!
//! The LEG consumer uses the exact same fail-closed provenance check as the EVM
//! consumer: a non-empty sender allow-list, a receiver binding, and a verified
//! Ed25519 envelope signature.

pub use arrow_adapter::provenance::{
    verify_envelope_provenance as verify_envelope, ProvenancePolicy, ProvenanceVerification,
};
