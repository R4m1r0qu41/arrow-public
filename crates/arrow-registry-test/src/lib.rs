//! Arrow Protocol on-chain lifecycle test harness.
//!
//! Implements RFC-006 graduation prerequisites:
//! - R-011 extraction artifact generation and canonical hashing
//! - On-chain lifecycle testing (observe → derive → artifact)
//! - Registry anchoring (calldata + OpenTimestamps)
//! - Testnet network endpoint / identity expectations

pub mod anchor;
pub mod artifact;
pub mod lifecycle;
pub mod networks;
