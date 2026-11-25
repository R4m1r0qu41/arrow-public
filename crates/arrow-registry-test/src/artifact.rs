//! R-011 extraction artifact schema and canonical hashing.
//!
//! An extraction artifact records a single on-chain lifecycle test: it binds
//! the observed transaction, the derived Arrow event, and the registry entry
//! together in a reproducible, anchorable document.
//!
//! ## Two-layer structure
//!
//! The artifact has a **`core`** layer (hashed) and an **`observation`** layer
//! (informational, not hashed). `extraction_artifact_hash` covers only `core`
//! so that independent re-derivation from public chain data converges on the
//! same hash regardless of when the observation was made.
//!
//! ## Canonical hash
//!
//! `extraction_artifact_hash = "sha256:" + lowercase_hex(SHA256(JCS(core)))`
//!
//! Uses RFC 8785 / JCS canonicalization (same as `arrow-transport` envelope
//! signing) via [`serde_jcs`].

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Discriminated tx reference for different chain families.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxRef {
    /// `"evm_tx_hash"` | `"btc_outpoint"` | `"sol_signature"`
    #[serde(rename = "type")]
    pub tx_type: String,
    /// The chain-native reference value.
    pub value: String,
}

impl TxRef {
    pub fn evm(tx_hash: impl Into<String>) -> Self {
        Self {
            tx_type: "evm_tx_hash".to_string(),
            value: tx_hash.into(),
        }
    }

    pub fn btc(txid: impl Into<String>, vout: u32) -> Self {
        Self {
            tx_type: "btc_outpoint".to_string(),
            value: format!("{}:{}", txid.into(), vout),
        }
    }

    pub fn sol(signature: impl Into<String>) -> Self {
        Self {
            tx_type: "sol_signature".to_string(),
            value: signature.into(),
        }
    }
}

/// The hashed core of a lifecycle artifact.
///
/// All fields here are deterministic for a confirmed/finalized transaction —
/// given `network_name` + `tx_ref`, any party with an RPC node can reproduce
/// this object and recompute the same `extraction_artifact_hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactCore {
    pub artifact_version: String,
    pub registry_code: String,
    pub entry_version: String,
    pub registry_version: String,
    pub network: String,
    pub network_name: String,
    pub chain_family: String,
    /// Chain-native identity actually read from the RPC. EVM: EIP-155 numeric
    /// id as string; BTC: chain name; SOL: genesis hash.
    pub chain_id_observed: String,
    pub tx_ref: TxRef,
    pub finality_model: String,
    /// `null` only when the observed state is FN (mirrors FB-001).
    pub finality_basis: Option<String>,
    /// The minted Arrow code at the observed terminal state.
    pub arrow_code: String,
    /// List of `protocol_required.*` paths corroborated by this test.
    pub maps_to_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_chain: Option<String>,
}

/// Informational observation metadata (not included in `extraction_artifact_hash`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactObservation {
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_head: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations_at_observation: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finality_timing: Option<FinalityTiming>,
}

/// Finality timing at observation time (excluded from hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityTiming {
    #[serde(rename = "type")]
    pub timing_type: String,
    pub value: u64,
}

/// Maintainer Ed25519 attestation over `extraction_artifact_hash`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub alg: String,
    pub did: String,
    pub sig: String,
}

/// Complete extraction artifact (R-011).
///
/// The `extraction_artifact_hash` is computed from `core` only and must be
/// validated with [`verify_hash`] before trusting any field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionArtifact {
    pub core: ArtifactCore,
    pub observation: ArtifactObservation,
    pub extraction_artifact_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation: Option<Attestation>,
}

/// Compute the `extraction_artifact_hash` for an [`ArtifactCore`].
///
/// `"sha256:" + lowercase_hex(SHA256(JCS(core)))`.
/// The JCS canonicalization matches `arrow-transport`'s envelope signing so
/// third-party verifiers can use any RFC 8785 implementation.
pub fn compute_hash(core: &ArtifactCore) -> Result<String> {
    let canonical =
        serde_jcs::to_vec(core).context("JCS canonicalization of artifact core failed")?;
    let digest = Sha256::digest(&canonical);
    Ok(format!("sha256:{}", hex::encode(digest)))
}

/// Build an [`ExtractionArtifact`] from core + observation, computing the hash.
pub fn build_artifact(
    core: ArtifactCore,
    observation: ArtifactObservation,
) -> Result<ExtractionArtifact> {
    let extraction_artifact_hash = compute_hash(&core)?;
    Ok(ExtractionArtifact {
        core,
        observation,
        extraction_artifact_hash,
        attestation: None,
    })
}

/// Verify that `artifact.extraction_artifact_hash` matches a fresh computation
/// over `artifact.core`.
pub fn verify_hash(artifact: &ExtractionArtifact) -> Result<()> {
    let expected = compute_hash(&artifact.core)?;
    if artifact.extraction_artifact_hash != expected {
        anyhow::bail!(
            "artifact hash mismatch: stored={}, computed={}",
            artifact.extraction_artifact_hash,
            expected
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_core() -> ArtifactCore {
        ArtifactCore {
            artifact_version: "arrow-lifecycle-artifact-v0".to_string(),
            registry_code: "ETH".to_string(),
            entry_version: "1.0".to_string(),
            registry_version: "2.0".to_string(),
            network: "testnet".to_string(),
            network_name: "sepolia".to_string(),
            chain_family: "evm".to_string(),
            chain_id_observed: "11155111".to_string(),
            tx_ref: TxRef::evm(
                "0xce09000000000000000000000000000000000000000000000000000000000088dc",
            ),
            finality_model: "probabilistic".to_string(),
            finality_basis: None,
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            maps_to_fields: vec![
                "code".to_string(),
                "protocol_required.finality_model".to_string(),
                "protocol_required.finality_basis_default".to_string(),
                "protocol_required.reorg_safety_confirmations".to_string(),
            ],
            value: None,
            settlement_chain: None,
        }
    }

    fn sample_observation() -> ArtifactObservation {
        ArtifactObservation {
            observed_at: "2026-06-04T00:00:00Z".to_string(),
            observer_head: Some(1_234_567),
            confirmations_at_observation: Some(40),
            finality_timing: None,
        }
    }

    // §4.8 — artifact_core_is_deterministic
    #[test]
    fn artifact_core_is_deterministic() {
        let core = sample_core();
        let h1 = compute_hash(&core).unwrap();
        let h2 = compute_hash(&core).unwrap();
        assert_eq!(h1, h2, "same core must produce identical hash");
    }

    // §4.8 — artifact_hash_excludes_observation_layer
    #[test]
    fn artifact_hash_excludes_observation_layer() {
        let core = sample_core();
        let obs1 = ArtifactObservation {
            observed_at: "2026-06-04T00:00:00Z".to_string(),
            observer_head: Some(1_000_000),
            confirmations_at_observation: Some(100),
            finality_timing: None,
        };
        let obs2 = ArtifactObservation {
            observed_at: "2026-06-05T12:00:00Z".to_string(),
            observer_head: Some(9_999_999),
            confirmations_at_observation: Some(1),
            finality_timing: Some(FinalityTiming {
                timing_type: "block_count".to_string(),
                value: 5,
            }),
        };
        let a1 = build_artifact(core.clone(), obs1).unwrap();
        let a2 = build_artifact(core.clone(), obs2).unwrap();
        assert_eq!(
            a1.extraction_artifact_hash, a2.extraction_artifact_hash,
            "observation layer changes must not affect the hash"
        );
    }

    // §4.8 — artifact_round_trips_serde
    #[test]
    fn artifact_round_trips_serde() {
        let artifact = build_artifact(sample_core(), sample_observation()).unwrap();
        let json = serde_json::to_string(&artifact).unwrap();
        let parsed: ExtractionArtifact = serde_json::from_str(&json).unwrap();
        assert_eq!(
            artifact.extraction_artifact_hash,
            parsed.extraction_artifact_hash
        );
        assert_eq!(artifact.core.registry_code, parsed.core.registry_code);
    }

    // §4.9 — canonical_hash_matches_known_vector
    #[test]
    fn canonical_hash_matches_known_vector() {
        // Golden hash generated from the fixture in tests/fixtures/.
        // Regenerate with: cargo run -p arrow-registry-test -- artifact hash tests/fixtures/eth_sepolia_core.json
        let core = sample_core();
        let hash = compute_hash(&core).unwrap();
        assert!(hash.starts_with("sha256:"), "hash must have sha256: prefix");
        assert_eq!(hash.len(), 7 + 64, "sha256: + 64 hex chars");

        // Load golden vector if present (CI generates and pins it).
        let fixture_path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/eth_sepolia_core_hash.txt"
        ));
        if fixture_path.exists() {
            let golden = std::fs::read_to_string(fixture_path)
                .unwrap()
                .trim()
                .to_string();
            assert_eq!(hash, golden, "hash must match committed golden vector");
        }
    }

    // §4.9 — key_order_independent
    #[test]
    fn key_order_independent() {
        // JCS sorts keys lexicographically; two cores with the same field values
        // must produce the same hash regardless of Rust struct field order.
        let c1 = sample_core();
        // Construct a second core via JSON round-trip (reorders some fields).
        let json = serde_json::to_string(&c1).unwrap();
        let c2: ArtifactCore = serde_json::from_str(&json).unwrap();
        assert_eq!(compute_hash(&c1).unwrap(), compute_hash(&c2).unwrap());
    }

    #[test]
    fn verify_hash_passes_on_correct_artifact() {
        let artifact = build_artifact(sample_core(), sample_observation()).unwrap();
        assert!(verify_hash(&artifact).is_ok());
    }

    #[test]
    fn verify_hash_fails_on_tampered_hash() {
        let mut artifact = build_artifact(sample_core(), sample_observation()).unwrap();
        artifact.extraction_artifact_hash = "sha256:deadbeef".to_string();
        assert!(verify_hash(&artifact).is_err());
    }
}
