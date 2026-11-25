//! Bitcoin UTXO → Arrow-code mapping.
//!
//! Bitcoin uses a UTXO model: a transaction has inputs (spending previous outputs)
//! and multiple outputs. The observer reduces one tracked output to an
//! [`ObservedUtxo`]. Finality is probabilistic-depth: ≥ 6 confirmations
//! (registry `reorg_safety_confirmations = 6`) → `FN`; fewer → `PD` with
//! `finality_basis = probabilistic`.
//!
//! `proof_type = tx_hash` (the only registered tx proof type). `proof_ref` is
//! `"<txid>:<vout>"` to identify the specific output.
//!
//! | Condition                     | arrow_code          | extra fields                              |
//! |-------------------------------|---------------------|-------------------------------------------|
//! | `confirmations >= 6`          | `1:BTC:FN:PR:1:A`   | —                                         |
//! | `confirmations <  6`          | `1:BTC:PD:HL:0:B`   | `finality_basis=probabilistic`, ETA blocks|

use arrow_adapter::SourceValueTransfer;
use arrow_core::types::event::ArrowEvent;
use chrono::Utc;
use uuid::Uuid;

/// Canonical reorg-safety depth for Bitcoin mainnet, mirroring
/// `registries/chain_registry.json` → `BTC.protocol_required.reorg_safety_confirmations`.
pub const BTC_REORG_SAFETY_CONFIRMATIONS: u64 = 6;

/// A UTXO output observed on Bitcoin, reduced to the facts the mapping needs.
///
/// Bitcoin transactions have multiple outputs; this represents one tracked output
/// (recipient + amount). `from` is the primary input address (simplified for v0.1
/// — a production observer would enumerate all inputs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedUtxo {
    /// Transaction id (hex string, no `0x` prefix — Bitcoin convention).
    pub txid: String,
    /// Output index within the transaction.
    pub vout: u32,
    /// Primary sending address (simplified; first input in v0.1).
    pub from: String,
    /// Recipient address for this output.
    pub recipient: String,
    /// Output value in satoshis.
    pub amount_sats: u64,
    /// Confirmation depth at observation time (0 = unconfirmed/mempool).
    pub confirmations: u64,
    /// Network identity from `getblockchaininfo.chain`, e.g. `"main"`,
    /// `"test"`, `"signet"`, `"regtest"`. Bound into lifecycle artifacts.
    pub observed_network: Option<String>,
}

/// Map an observed Bitcoin UTXO to a protocol-valid [`ArrowEvent`].
///
/// Uses `BTC_REORG_SAFETY_CONFIRMATIONS` (6) for the finality threshold.
/// The returned event carries a typed [`SourceValueTransfer`] with `unit =
/// "sats"` — the only source fact a consumer needs to derive an action intent.
pub fn map_to_arrow(obs: &ObservedUtxo) -> ArrowEvent {
    let lifecycle_id = format!("btc:{}", obs.txid);
    let event_id = format!("evt-{}", Uuid::new_v4());
    // `<txid>:<vout>` identifies the exact output; `proof_type = tx_hash` is
    // the registered tx-proof type (utxo_ref is not in proof_types.json).
    let proof_ref = format!("{}:{}", obs.txid, obs.vout);

    let mut event = ArrowEvent {
        event_id,
        lifecycle_id,
        timestamp: Utc::now().to_rfc3339(),
        object_type: "transfer".to_string(),
        object_id: obs.txid.clone(),
        chain_id: Some("BTC".to_string()),
        proof_type: Some("tx_hash".to_string()),
        proof_ref: Some(proof_ref),
        metadata: Some(serde_json::json!({
            "from": obs.from,
            "recipient": obs.recipient,
            "amount_sats": obs.amount_sats,
            "vout": obs.vout,
            "confirmations": obs.confirmations,
        })),
        ..Default::default()
    };

    SourceValueTransfer::btc_native(
        "BTC",
        obs.txid.clone(),
        obs.from.clone(),
        obs.recipient.clone(),
        obs.amount_sats.to_string(),
        Some(obs.confirmations),
    )
    .attach_to_event(&mut event)
    .expect("serializing typed BTC source value transfer");

    if obs.confirmations >= BTC_REORG_SAFETY_CONFIRMATIONS {
        event.arrow_code = "1:BTC:FN:PR:1:A".to_string();
        // LFC-008 (RFC-010 Part 2A): a settled observation under a bounded
        // closure discipline declares when its lifecycle is due to CLOSE.
        // Settlement depth is already reached at emission, so closure is due
        // immediately: 0 blocks remaining. Deterministic — no clock needed.
        event.closure_eta = Some(0);
        event.closure_eta_type = Some("block_count".to_string());
    } else {
        event.arrow_code = "1:BTC:PD:HL:0:B".to_string();
        event.finality_basis = Some("probabilistic".to_string());
        let remaining = BTC_REORG_SAFETY_CONFIRMATIONS.saturating_sub(obs.confirmations);
        event.finality_eta = Some(remaining);
        event.finality_eta_type = Some("block_count".to_string());
    }

    event
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_adapter::SourceValueTransfer;
    use arrow_core::registry::loader::Registries;
    use arrow_core::validation::pipeline::ValidationPipeline;

    fn obs() -> ObservedUtxo {
        ObservedUtxo {
            txid: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
            vout: 1,
            from: "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf".to_string(),
            recipient: "1BpEi6DfDAUFd153wiGrvkiboLLaqFZu".to_string(),
            amount_sats: 5_000_000,
            confirmations: 0,
            observed_network: None,
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
    fn zero_confirmations_is_pending() {
        let ev = map_to_arrow(&obs());
        assert_eq!(ev.arrow_code, "1:BTC:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("probabilistic"));
        assert_eq!(ev.finality_eta, Some(6));
        assert_eq!(ev.finality_eta_type.as_deref(), Some("block_count"));
        assert_validates(&ev);
    }

    #[test]
    fn five_confirmations_still_pending() {
        let ev = map_to_arrow(&ObservedUtxo {
            confirmations: 5,
            ..obs()
        });
        assert_eq!(ev.arrow_code, "1:BTC:PD:HL:0:B");
        assert_eq!(ev.finality_eta, Some(1));
        assert_validates(&ev);
    }

    #[test]
    fn six_confirmations_is_final() {
        let ev = map_to_arrow(&ObservedUtxo {
            confirmations: 6,
            ..obs()
        });
        assert_eq!(ev.arrow_code, "1:BTC:FN:PR:1:A");
        assert!(ev.finality_basis.is_none());
        assert!(ev.finality_eta.is_none());
        assert_validates(&ev);
    }

    #[test]
    fn deep_confirmations_is_final() {
        let ev = map_to_arrow(&ObservedUtxo {
            confirmations: 100,
            ..obs()
        });
        assert_eq!(ev.arrow_code, "1:BTC:FN:PR:1:A");
        assert_validates(&ev);
    }

    #[test]
    fn utxo_carries_sats_unit_source_transfer() {
        let ev = map_to_arrow(&obs());
        let transfer =
            SourceValueTransfer::from_event(&ev).expect("typed transfer must be present");
        assert_eq!(transfer.source_system, "bitcoin");
        assert_eq!(transfer.amount.unit, "sats");
        assert_eq!(transfer.amount.value, "5000000");
        assert_eq!(transfer.asset.symbol.as_deref(), Some("BTC"));
        assert_eq!(transfer.asset.kind, "native");
    }

    #[test]
    fn proof_ref_is_txid_colon_vout() {
        let ev = map_to_arrow(&obs());
        let expected = format!(
            "{}:{}",
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2", 1
        );
        assert_eq!(ev.proof_ref.as_deref(), Some(expected.as_str()));
        assert_eq!(ev.proof_type.as_deref(), Some("tx_hash"));
    }

    #[test]
    fn lifecycle_id_is_btc_prefix_plus_txid() {
        let ev = map_to_arrow(&obs());
        assert_eq!(
            ev.lifecycle_id,
            "btc:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
        );
    }

    #[test]
    fn btc_final_event_validates_against_pipeline() {
        let ev = map_to_arrow(&ObservedUtxo {
            confirmations: 6,
            ..obs()
        });
        assert_validates(&ev);
    }
}
