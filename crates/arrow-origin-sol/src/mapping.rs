//! Solana transaction → Arrow-code mapping.
//!
//! Solana uses **commitment levels** for finality, not block confirmation depth.
//! The registry (`chain_registry.json`) sets `reorg_safety_confirmations = null`
//! for SOL — do not apply a confirmation threshold. Drive finality from the
//! commitment level returned by the JSON-RPC call.
//!
//! `proof_type = tx_hash` (the registered tx-proof type). `proof_ref` is the
//! **base58 transaction signature** — Solana's unique tx identifier.
//!
//! The `finality_basis` field is set to `probabilistic` for pre-finalized events
//! (FB-001 requires `finality_basis` when Finality=0). There is no registered
//! "commitment" basis; `probabilistic` is the correct registered choice.
//!
//! | Commitment   | arrow_code         | finality_basis  | Confidence |
//! |--------------|--------------------|-----------------|------------|
//! | `finalized`  | `1:SOL:FN:PR:1:A`  | —               | A          |
//! | `confirmed`  | `1:SOL:PD:HL:0:B`  | `probabilistic` | B (capped) |
//! | `processed`  | `1:SOL:PD:HL:0:B`  | `probabilistic` | B (capped) |

use arrow_adapter::SourceValueTransfer;
use arrow_core::types::event::ArrowEvent;
use chrono::Utc;
use uuid::Uuid;

/// Solana transaction commitment level at observation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolCommitment {
    /// Transaction has been processed but may still be rolled back.
    Processed,
    /// Transaction has been confirmed by a supermajority of stake.
    Confirmed,
    /// Transaction is finalized — included in a rooted block and will not be
    /// rolled back. This is the only level that produces `FN` in Arrow.
    Finalized,
}

impl SolCommitment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Processed => "processed",
            Self::Confirmed => "confirmed",
            Self::Finalized => "finalized",
        }
    }

    pub fn is_final(self) -> bool {
        matches!(self, Self::Finalized)
    }
}

/// A Solana transaction observed on-chain, reduced to the facts the mapping needs.
///
/// Solana uses account-based + instruction semantics. For a native SOL System
/// Program transfer, the relevant facts are sender, recipient, and lamports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedTransaction {
    /// Base58-encoded transaction signature (Solana's tx identifier).
    pub signature: String,
    /// Slot the transaction was included in (informational; not used for finality).
    pub slot: u64,
    /// Commitment level at observation time.
    pub commitment: SolCommitment,
    /// Sender public key (base58).
    pub from: String,
    /// Recipient public key (base58).
    pub to: String,
    /// Transferred amount in lamports.
    pub lamports: u64,
    /// Genesis hash returned by `getGenesisHash`, identifying the Solana cluster
    /// (mainnet-beta, devnet, testnet). Bound into lifecycle artifacts (§4.6).
    pub genesis_hash: Option<String>,
}

/// Map an observed Solana transaction to a protocol-valid [`ArrowEvent`].
///
/// Finality is derived from [`SolCommitment`]: only `Finalized` produces `FN`.
/// Pre-finalized events (`Confirmed`/`Processed`) are capped at Confidence B.
pub fn map_to_arrow(obs: &ObservedTransaction) -> ArrowEvent {
    let lifecycle_id = format!("sol:{}", obs.signature);
    let event_id = format!("evt-{}", Uuid::new_v4());

    let mut event = ArrowEvent {
        event_id,
        lifecycle_id,
        timestamp: Utc::now().to_rfc3339(),
        object_type: "transfer".to_string(),
        object_id: obs.signature.clone(),
        chain_id: Some("SOL".to_string()),
        proof_type: Some("tx_hash".to_string()),
        // proof_ref holds the base58 signature — the canonical Solana tx id.
        proof_ref: Some(obs.signature.clone()),
        metadata: Some(serde_json::json!({
            "from": obs.from,
            "to": obs.to,
            "lamports": obs.lamports,
            "slot": obs.slot,
            "commitment": obs.commitment.as_str(),
        })),
        ..Default::default()
    };

    SourceValueTransfer::sol_native(
        "SOL",
        obs.signature.clone(),
        obs.from.clone(),
        obs.to.clone(),
        obs.lamports.to_string(),
        // confirmations is not meaningful for Solana; slot depth carries no
        // protocol guarantee — commitment level is the authoritative signal.
        None,
    )
    .attach_to_event(&mut event)
    .expect("serializing typed SOL source value transfer");

    if obs.commitment.is_final() {
        event.arrow_code = "1:SOL:FN:PR:1:A".to_string();
        // LFC-008 (RFC-010 Part 2A): a settled observation under a bounded
        // closure discipline declares when its lifecycle is due to CLOSE.
        // Settlement depth is already reached at emission, so closure is due
        // immediately: 0 blocks remaining. Deterministic — no clock needed.
        event.closure_eta = Some(0);
        event.closure_eta_type = Some("block_count".to_string());
    } else {
        event.arrow_code = "1:SOL:PD:HL:0:B".to_string();
        event.finality_basis = Some("probabilistic".to_string());
        // No block-count ETA for Solana — finality is commitment-based.
    }

    event
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_adapter::SourceValueTransfer;
    use arrow_core::registry::loader::Registries;
    use arrow_core::validation::pipeline::ValidationPipeline;

    fn obs(commitment: SolCommitment) -> ObservedTransaction {
        ObservedTransaction {
            signature: "5j7s8K9mN2pQ3rT4uV5wX6yZ7aB8cD9eF0gH1iJ2kL3mN4oP5qR6sT7uV8wX9yZ"
                .to_string(),
            slot: 280_000_000,
            commitment,
            from: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".to_string(),
            to: "6FEgANEPqTsHxfyoTKtGBpBqSH6QqMG1V3HxVBFLbXPa".to_string(),
            lamports: 1_000_000_000,
            genesis_hash: None,
        }
    }

    fn assert_validates(event: &ArrowEvent) {
        let registries = Registries::load_embedded().expect("load embedded registries");
        let pipeline = ValidationPipeline::new(&registries);
        let result = pipeline.validate_event(event);
        let errors: Vec<_> = result.violations.iter().filter(|v| v.is_error()).collect();
        assert!(
            result.passed && errors.is_empty(),
            "event failed validation: arrow_code={}, errors={:#?}",
            event.arrow_code,
            errors
        );
    }

    #[test]
    fn finalized_commitment_is_final() {
        let ev = map_to_arrow(&obs(SolCommitment::Finalized));
        assert_eq!(ev.arrow_code, "1:SOL:FN:PR:1:A");
        assert!(ev.finality_basis.is_none());
        assert!(ev.finality_eta.is_none());
        assert_validates(&ev);
    }

    #[test]
    fn confirmed_commitment_is_pending() {
        let ev = map_to_arrow(&obs(SolCommitment::Confirmed));
        assert_eq!(ev.arrow_code, "1:SOL:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("probabilistic"));
        assert!(ev.finality_eta.is_none(), "Solana has no block-count ETA");
        assert_validates(&ev);
    }

    #[test]
    fn processed_commitment_is_pending() {
        let ev = map_to_arrow(&obs(SolCommitment::Processed));
        assert_eq!(ev.arrow_code, "1:SOL:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("probabilistic"));
        assert_validates(&ev);
    }

    #[test]
    fn sol_event_carries_lamports_unit_source_transfer() {
        let ev = map_to_arrow(&obs(SolCommitment::Finalized));
        let transfer =
            SourceValueTransfer::from_event(&ev).expect("typed transfer must be present");
        assert_eq!(transfer.source_system, "solana");
        assert_eq!(transfer.amount.unit, "lamports");
        assert_eq!(transfer.amount.value, "1000000000");
        assert_eq!(transfer.asset.symbol.as_deref(), Some("SOL"));
        assert_eq!(transfer.asset.kind, "native");
    }

    #[test]
    fn proof_ref_is_base58_signature() {
        let ev = map_to_arrow(&obs(SolCommitment::Finalized));
        assert_eq!(
            ev.proof_ref.as_deref(),
            Some("5j7s8K9mN2pQ3rT4uV5wX6yZ7aB8cD9eF0gH1iJ2kL3mN4oP5qR6sT7uV8wX9yZ")
        );
        assert_eq!(ev.proof_type.as_deref(), Some("tx_hash"));
    }

    #[test]
    fn lifecycle_id_is_sol_prefix_plus_signature() {
        let ev = map_to_arrow(&obs(SolCommitment::Finalized));
        assert_eq!(
            ev.lifecycle_id,
            "sol:5j7s8K9mN2pQ3rT4uV5wX6yZ7aB8cD9eF0gH1iJ2kL3mN4oP5qR6sT7uV8wX9yZ"
        );
    }

    #[test]
    fn finalized_confidence_is_a_not_b() {
        let ev = map_to_arrow(&obs(SolCommitment::Finalized));
        assert!(
            ev.arrow_code.ends_with(":A"),
            "finalized SOL must have Confidence A; got {}",
            ev.arrow_code
        );
    }

    #[test]
    fn pre_final_confidence_is_capped_at_b() {
        for c in [SolCommitment::Confirmed, SolCommitment::Processed] {
            let ev = map_to_arrow(&obs(c));
            assert!(
                ev.arrow_code.ends_with(":B"),
                "pre-final SOL ({}) must be capped at Confidence B; got {}",
                c.as_str(),
                ev.arrow_code
            );
        }
    }
}
