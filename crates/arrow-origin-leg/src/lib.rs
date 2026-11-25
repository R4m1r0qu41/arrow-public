//! Legal-attestation Origin Adapter for the Arrow Protocol.
//!
//! Takes a structured legal attestation (the LexExe→Arrow boundary object), maps
//! it to a protocol-valid `origin = LEG` [`arrow_core::types::event::ArrowEvent`],
//! and (via [`emit`]) signs it into a transport envelope and POSTs it to an Arrow
//! node.
//!
//! - [`attestation`] — the pure, network-free attestation→Arrow-code core. Derives
//!   the canonical LEG Action (LEG-006/LEG-007), sets `leg_subtype` and
//!   `leg_jurisdiction`, and satisfies FB-001.
//! - [`emit`] — wrap, sign, and POST the event to a node.
//!
//! This adapter **classifies and records**; it never determines applicable law or
//! authorizes a downstream action. Acting on a minted LEG event is delegated, by
//! design, to the fail-closed human-in-the-loop consumer (`arrow-consume-leg`).
//!
//! The minted event carries a `leg_jurisdiction` block consumed by live
//! validation rules: **LEG-009** (`leg_jurisdiction` required) and **LEG-010**
//! (`supra` registry membership against
//! `registries/leg_supra_jurisdictions.json`).

pub mod attestation;
pub mod emit;

pub use attestation::{
    derive_action, map_to_arrow, ActionDerivation, LegalAttestation, LEG_SUBTYPES,
};
pub use emit::{build_signed_envelope, post_envelope, EmitOutcome};
