//! Pure mapping from an observed EVM transaction to an Arrow Event.
//!
//! This module has **no chain dependency** — it operates on the plain
//! [`ObservedTx`] struct and an [`EvmChainConfig`] so the
//! confirmation→finality/confidence logic is fully unit-testable without a
//! network. It is the deterministic, chain-parameterized heart of every
//! EVM-family Origin Adapter (ETH, and the Base/Arbitrum wrappers).
//!
//! ## The mapping
//!
//! **L1** (`reorg_safety_confirmations` set): given confirmation depth and the
//! registry's reorg-safety threshold N, an observed transaction becomes:
//!
//! | Condition            | arrow_code            | extra fields                               |
//! |----------------------|-----------------------|--------------------------------------------|
//! | `confirmations >= N` | `1:{ORIGIN}:FN:PR:1:A`| —                                          |
//! | `confirmations <  N` | `1:{ORIGIN}:PD:HL:0:B`| `finality_basis=probabilistic`, ETA blocks |
//!
//! **L2 optimistic rollup** (`challenge_window_seconds` set): block depth alone
//! never proves L1-settled finality, so this conservatively mints a pending
//! event (`1:{ORIGIN}:PD:HL:0:B`) with `finality_basis=optimistic_window` until
//! a settlement watcher (a later phase) can confirm the window elapsed. It never
//! claims finality beyond the registered challenge model.
//!
//! Grounding: Class `1`=OBSERVE; State `FN`/`PD`. **SEM-013** forbids an
//! executing Action (PR/TR/CL) when Finality=0, so the pending event uses Action
//! `HL` (Hold). **FB-001** requires `finality_basis` when Finality=0. Confidence
//! is bounded by the chain's verification regime.

use arrow_adapter::SourceValueTransfer;
use arrow_core::types::event::ArrowEvent;
use chrono::Utc;
use uuid::Uuid;

use crate::config::{EvmChainConfig, EvmConfigError, EvmRegime};

/// Canonical reorg-safety depth for Ethereum mainnet, mirroring
/// `registries/chain_registry.json` → `ETH.protocol_required.reorg_safety_confirmations`.
/// Retained here for callers that pass the threshold explicitly.
pub const ETH_REORG_SAFETY_CONFIRMATIONS: u64 = 12;

/// A transaction observed on an EVM chain, reduced to the facts the mapping needs.
///
/// Produced by [`crate::observer::EvmObserver`] in the binary path; constructed
/// directly in unit tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedTx {
    /// `0x`-prefixed transaction hash.
    pub tx_hash: String,
    /// Block height the transaction was included in. `None` while still pending
    /// in the mempool (not yet mined).
    pub block_number: Option<u64>,
    /// Current chain head height at observation time.
    pub head_block: u64,
    /// `0x`-prefixed sender address.
    pub from: String,
    /// `0x`-prefixed recipient address. `None` for contract-creation txs.
    pub to: Option<String>,
    /// Transferred value in wei, as a decimal string (avoids u256/precision loss
    /// in normalized adapter interchange).
    pub value_wei: String,
    /// EIP-155 chain id as returned by `eth_chainId`. Used to bind chain
    /// identity into the lifecycle artifact and to validate the RPC is on the
    /// expected network before minting.
    pub observed_chain_id: u64,
    /// Unix-second timestamp of the inclusion block, from `eth_getBlockByNumber`.
    /// `None` for pending (unmined) transactions. Used to compute the L2
    /// challenge-window close timestamp for `finality_eta`.
    pub block_timestamp: Option<u64>,
}

impl ObservedTx {
    /// Confirmation depth: `head − included_block + 1`, or `0` if not yet mined.
    ///
    /// A transaction sitting in the block at the current head counts as one
    /// confirmation.
    pub fn confirmations(&self) -> u64 {
        match self.block_number {
            Some(b) if self.head_block >= b => self.head_block - b + 1,
            _ => 0,
        }
    }
}

/// Map an observed transaction to a protocol-valid [`ArrowEvent`] for `config`,
/// failing closed on an incoherent config.
///
/// The Origin field, chain id, finality threshold, and regime all come from
/// `config` — there is nothing chain-specific in this function. Because
/// `EvmChainConfig` fields are public, this validates the config first and
/// returns [`EvmConfigError`] rather than panicking; prefer it over
/// [`map_to_arrow`] for dynamically- or manually-built configs (e.g. the
/// Base/Arbitrum wrappers).
pub fn try_map_to_arrow(
    obs: &ObservedTx,
    config: &EvmChainConfig,
) -> Result<ArrowEvent, EvmConfigError> {
    config.validate()?;

    // Fail closed: if the config expects a specific EIP-155 chain id and the
    // RPC returned a different one, reject the observation before minting.
    if let Some(expected_id) = config.expected_eip155_chain_id {
        if obs.observed_chain_id != expected_id {
            return Err(EvmConfigError::ChainIdMismatch {
                origin_code: config.origin_code.clone(),
                expected: expected_id,
                observed: obs.observed_chain_id,
            });
        }
    }

    let confirmations = obs.confirmations();

    // Stable lifecycle per transaction; a fresh event_id per observation.
    let lifecycle_id = format!("{}:{}", config.lifecycle_prefix, obs.tx_hash);
    let event_id = format!("evt-{}", Uuid::new_v4());

    let mut event = ArrowEvent {
        event_id,
        lifecycle_id,
        timestamp: Utc::now().to_rfc3339(),
        object_type: "transfer".to_string(),
        object_id: obs.tx_hash.clone(),
        chain_id: Some(config.chain_id.clone()),
        proof_type: Some("tx_hash".to_string()),
        proof_ref: Some(obs.tx_hash.clone()),
        metadata: Some(serde_json::json!({
            "from": obs.from,
            "to": obs.to,
            "value_wei": obs.value_wei,
            "confirmations": confirmations,
            "observed_head": obs.head_block,
            "included_block": obs.block_number,
        })),
        ..Default::default()
    };

    // Normalized, source-independent value-transfer fact (native asset). All
    // EVM-family chains in the registry use an ETH-denominated native asset.
    if let Some(to) = &obs.to {
        let transfer = SourceValueTransfer::evm_native(
            config.chain_id.clone(),
            obs.tx_hash.clone(),
            obs.from.clone(),
            to.clone(),
            obs.value_wei.clone(),
            Some(confirmations),
        );
        transfer
            .attach_to_event(&mut event)
            .expect("serializing typed source value transfer");
    }

    // Confidence ceiling per regime: native_canonical reaches A; an L2 only
    // reaches A *after settlement* (not derivable from block depth here).
    let final_confidence = match config.regime {
        EvmRegime::NativeCanonical => 'A',
        EvmRegime::OptimisticSettled => 'A',
    };
    let origin = &config.origin_code;

    if config.is_optimistic_l2() {
        // L2: block depth does not prove the challenge window elapsed. Mint a
        // pending event; a settlement watcher (later phase) promotes to final.
        event.arrow_code = format!("1:{origin}:PD:HL:0:B");
        event.finality_basis = Some("optimistic_window".to_string());
        // Compute the deterministic challenge-window close time. FB-001 requires
        // a dated window for optimistic_window (requires_finality_eta), so an
        // observation without its inclusion-block timestamp fails closed: no
        // event is minted that the validator would reject.
        let (ts, window) = match (obs.block_timestamp, config.challenge_window_seconds) {
            (Some(ts), Some(window)) => (ts, window),
            _ => {
                return Err(EvmConfigError::MissingBlockTimestamp {
                    origin_code: config.origin_code.clone(),
                })
            }
        };
        event.finality_eta = Some(ts + window);
        event.finality_eta_type = Some("unix_ts".to_string());
    } else {
        // L1: finality by confirmation depth. `validate()` above guarantees a
        // non-L2 config carries `reorg_safety_confirmations`.
        let reorg_safety = config.reorg_safety_confirmations.ok_or_else(|| {
            EvmConfigError::InvalidFinalityConfig {
                origin_code: config.origin_code.clone(),
                layer: config.layer,
            }
        })?;
        if confirmations >= reorg_safety {
            event.arrow_code = format!("1:{origin}:FN:PR:1:{final_confidence}");
            // LFC-008 (RFC-010 Part 2A): a settled observation under a bounded
            // closure discipline declares when its lifecycle is due to CLOSE.
            // Settlement depth is already reached at emission, so closure is due
            // immediately: 0 blocks remaining. Deterministic — no clock needed.
            event.closure_eta = Some(0);
            event.closure_eta_type = Some("block_count".to_string());
        } else {
            event.arrow_code = format!("1:{origin}:PD:HL:0:B");
            event.finality_basis = Some("probabilistic".to_string());
            let remaining = reorg_safety.saturating_sub(confirmations);
            event.finality_eta = Some(remaining);
            event.finality_eta_type = Some("block_count".to_string());
        }
    }

    Ok(event)
}

/// Convenience wrapper over [`try_map_to_arrow`] for callers with a
/// statically-known-valid config (e.g. [`EvmChainConfig::eth`]).
///
/// # Panics
/// Panics if `config` is not a coherent finality config. Use
/// [`try_map_to_arrow`] for configs constructed at runtime.
pub fn map_to_arrow(obs: &ObservedTx, config: &EvmChainConfig) -> ArrowEvent {
    try_map_to_arrow(obs, config)
        .expect("map_to_arrow called with an invalid EvmChainConfig; use try_map_to_arrow")
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::registry::loader::Registries;
    use arrow_core::validation::pipeline::ValidationPipeline;

    fn obs_with(block: Option<u64>, head: u64) -> ObservedTx {
        ObservedTx {
            tx_hash: "0xabc123".to_string(),
            block_number: block,
            head_block: head,
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: Some("0x2222222222222222222222222222222222222222".to_string()),
            value_wei: "1000000000000000000".to_string(),
            // Mainnet chain id by default; tests override where needed.
            observed_chain_id: 1,
            block_timestamp: None,
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
    fn eth_config_final_matches_reference_code() {
        let ev = map_to_arrow(&obs_with(Some(100), 111), &EvmChainConfig::eth());
        assert_eq!(ev.arrow_code, "1:ETH:FN:PR:1:A");
        assert_eq!(ev.chain_id.as_deref(), Some("ETH"));
        assert_eq!(ev.lifecycle_id, "eth:0xabc123");
        assert!(ev.finality_basis.is_none());
        assert_validates(&ev);
    }

    #[test]
    fn eth_config_pending_matches_reference_code() {
        let ev = map_to_arrow(&obs_with(None, 100), &EvmChainConfig::eth());
        assert_eq!(ev.arrow_code, "1:ETH:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("probabilistic"));
        assert_eq!(ev.finality_eta, Some(12));
        assert_validates(&ev);
    }

    #[test]
    fn optimistic_l2_is_pending_with_optimistic_window_basis() {
        // A hypothetical L2 config: deeply confirmed on the L2, but block depth
        // never proves the L1 challenge window elapsed → conservative pending.
        let cfg = EvmChainConfig {
            origin_code: "ARB".to_string(),
            chain_id: "ARB".to_string(),
            lifecycle_prefix: "arb".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: None,
        };
        let obs = ObservedTx {
            block_timestamp: Some(1_700_000_000),
            observed_chain_id: 42161,
            ..obs_with(Some(100), 100_000)
        };
        let ev = try_map_to_arrow(&obs, &cfg).unwrap();
        assert_eq!(ev.arrow_code, "1:ARB:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("optimistic_window"));
        assert_validates(&ev);
    }

    #[test]
    fn try_map_to_arrow_fails_closed_on_invalid_config() {
        // Layer 1 with no reorg-safety depth and no challenge window: neither a
        // valid L1 nor a valid L2 → typed error, never a panic.
        let mut cfg = EvmChainConfig::eth();
        cfg.reorg_safety_confirmations = None;
        let err = try_map_to_arrow(&obs_with(Some(100), 111), &cfg).unwrap_err();
        assert_eq!(
            err,
            EvmConfigError::InvalidFinalityConfig {
                origin_code: "ETH".to_string(),
                layer: 1,
            }
        );
    }

    // §4.1 — chain id validation tests

    #[test]
    fn chain_id_mismatch_fails_closed() {
        // Config expects mainnet (1); observed chain returns Sepolia (11155111).
        // No event must be minted.
        let obs = ObservedTx {
            observed_chain_id: 11155111,
            ..obs_with(Some(100), 111)
        };
        let err = try_map_to_arrow(&obs, &EvmChainConfig::eth()).unwrap_err();
        assert_eq!(
            err,
            EvmConfigError::ChainIdMismatch {
                origin_code: "ETH".to_string(),
                expected: 1,
                observed: 11155111,
            }
        );
    }

    #[test]
    fn arrow_event_chain_id_remains_registry_code() {
        // The Arrow event chain_id field is the registry code ("ETH"), never the
        // numeric EIP-155 id, regardless of what the RPC returned.
        let ev = map_to_arrow(&obs_with(Some(100), 111), &EvmChainConfig::eth());
        assert_eq!(ev.chain_id.as_deref(), Some("ETH"));
    }

    #[test]
    fn no_expected_chain_id_skips_check() {
        // A config with no expected_eip155_chain_id accepts any observed chain id.
        let mut cfg = EvmChainConfig::eth();
        cfg.expected_eip155_chain_id = None;
        let obs = ObservedTx {
            observed_chain_id: 99999,
            ..obs_with(Some(100), 111)
        };
        assert!(try_map_to_arrow(&obs, &cfg).is_ok());
    }

    // §4.3 — optimistic ETA tests

    #[test]
    fn optimistic_l2_sets_window_close_eta() {
        let cfg = EvmChainConfig {
            origin_code: "BSE".to_string(),
            chain_id: "BASE".to_string(),
            lifecycle_prefix: "base".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: None,
        };
        let obs = ObservedTx {
            block_timestamp: Some(1_700_000_000),
            observed_chain_id: 8453,
            ..obs_with(Some(100), 1000)
        };
        let ev = try_map_to_arrow(&obs, &cfg).unwrap();
        assert_eq!(ev.arrow_code, "1:BSE:PD:HL:0:B");
        assert_eq!(ev.finality_eta, Some(1_700_000_000 + 604800));
        assert_eq!(ev.finality_eta_type.as_deref(), Some("unix_ts"));
        assert_eq!(ev.finality_basis.as_deref(), Some("optimistic_window"));
    }

    #[test]
    fn optimistic_l2_without_timestamp_fails_closed() {
        // When block_timestamp is None the challenge-window close cannot be
        // dated, so the producer must fail closed (FB-001: optimistic_window
        // requires finality_eta) instead of minting an invalid event.
        let cfg = EvmChainConfig {
            origin_code: "ARB".to_string(),
            chain_id: "ARB".to_string(),
            lifecycle_prefix: "arb".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: None,
        };
        let obs = ObservedTx {
            block_timestamp: None,
            observed_chain_id: 42161,
            ..obs_with(None, 1000)
        };
        let err = try_map_to_arrow(&obs, &cfg).unwrap_err();
        assert_eq!(
            err,
            EvmConfigError::MissingBlockTimestamp {
                origin_code: "ARB".to_string()
            }
        );
    }

    #[test]
    fn optimistic_l2_never_final_from_depth() {
        // Even with head 1_000_000 blocks ahead, an L2 must stay pending.
        let cfg = EvmChainConfig {
            origin_code: "BSE".to_string(),
            chain_id: "BASE".to_string(),
            lifecycle_prefix: "base".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: None,
        };
        let obs = ObservedTx {
            observed_chain_id: 8453,
            block_timestamp: Some(1_700_000_000),
            ..obs_with(Some(100), 1_100_000)
        };
        let ev = try_map_to_arrow(&obs, &cfg).unwrap();
        assert_eq!(
            ev.arrow_code, "1:BSE:PD:HL:0:B",
            "L2 must never reach FN from block depth"
        );
    }
}
