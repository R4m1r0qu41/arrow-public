//! Backward-compatible re-export of shared adapter provenance verification.

pub use arrow_adapter::provenance::{
    verify_envelope_provenance as verify_envelope, ProvenancePolicy, ProvenanceVerification,
};
