//! Per-chain configuration for the EVM-family origin/consumer adapters.
//!
//! One Origin Adapter and one Consumer Adapter implementation serves every
//! EVM-compatible chain in the Chain Registry. The differences between chains
//! (Arrow Origin code, registry chain id, L1 reorg-safety depth vs. L2 challenge
//! window, the Confidence ceiling the verification regime permits) are *data*,
//! not code. This struct carries that data; the mapping and consume logic read
//! it instead of hard-coding `"ETH"` and `12`.
//!
//! Grounding: every field mirrors a `registries/chain_registry.json`
//! `protocol_required`/`advisory` value or a `registries/origins.json` entry.
//! Adding a new EVM chain is a new `EvmChainConfig`, not a new code path.

use thiserror::Error;

/// Verification regime an EVM chain's events are minted under (mirrors
/// `registries/verification_regimes.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvmRegime {
    /// L1 canonical state is the proof (`native_canonical`, max Confidence A).
    NativeCanonical,
    /// L2 rollup inheriting L1 security after a challenge window
    /// (`optimistic_settled`, max Confidence A post-settlement, B pre).
    OptimisticSettled,
}

impl EvmRegime {
    /// Registered `verification_regime` code.
    pub fn code(self) -> &'static str {
        match self {
            EvmRegime::NativeCanonical => "native_canonical",
            EvmRegime::OptimisticSettled => "optimistic_settled",
        }
    }
}

/// A config that cannot be mapped to a coherent finality model.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvmConfigError {
    /// The config is neither a valid L1 (layer 1 with `reorg_safety_confirmations`)
    /// nor a valid optimistic L2 (layer 2 with `challenge_window_seconds`).
    #[error(
        "EVM chain `{origin_code}` (layer {layer}) is not a valid finality config: \
         an L1 must set reorg_safety_confirmations (and no challenge window); \
         an optimistic L2 (layer 2) must set challenge_window_seconds"
    )]
    InvalidFinalityConfig { origin_code: String, layer: u8 },
    /// The observed EIP-155 chain id does not match what the config expects.
    /// Fails closed: no event is minted when the RPC is connected to the wrong network.
    #[error(
        "chain id mismatch for `{origin_code}`: \
         expected EIP-155 id {expected}, observed {observed}"
    )]
    ChainIdMismatch {
        origin_code: String,
        expected: u64,
        observed: u64,
    },
    /// An optimistic-L2 observation lacks its inclusion-block timestamp, so the
    /// challenge-window close (`finality_eta`) cannot be computed. FB-001
    /// requires a dated window for `finality_basis = optimistic_window`; a
    /// fail-closed producer must not mint an event the validator rejects.
    #[error(
        "cannot mint optimistic-L2 pending event for `{origin_code}`: \
         block_timestamp is unavailable, so the challenge-window close \
         (finality_eta) cannot be computed (FB-001 requires a dated window)"
    )]
    MissingBlockTimestamp { origin_code: String },
}

/// Static, registry-derived description of one EVM-compatible chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmChainConfig {
    /// Canonical 3-char Arrow Origin code (`registries/origins.json` key), e.g.
    /// `ETH`, `ARB`, `BSE`. This is the Origin field (position 2) of the minted
    /// `arrow_code`.
    pub origin_code: String,
    /// Chain Registry `code` (`registries/chain_registry.json`), e.g. `ETH`,
    /// `BASE`. Distinct from `origin_code` for chains whose registry code is not
    /// a valid 3-char Origin (e.g. `BASE`). Becomes the event `chain_id` and the
    /// normalized `SourceValueTransfer.source_chain_id`.
    pub chain_id: String,
    /// Lowercase prefix for the stable per-transaction `lifecycle_id`
    /// (`{prefix}:{tx_hash}`), e.g. `eth`, `base`.
    pub lifecycle_prefix: String,
    /// Chain Registry `layer`: 1 (settlement L1) or 2 (rollup).
    pub layer: u8,
    /// Chain Registry `settlement_chain` (L2 only), e.g. `Some("ETH")`.
    pub settlement_chain: Option<String>,
    /// Verification regime the chain mints under.
    pub regime: EvmRegime,
    /// L1 reorg-safety confirmation depth
    /// (`protocol_required.reorg_safety_confirmations`). `Some` for L1.
    pub reorg_safety_confirmations: Option<u64>,
    /// L2 fraud-proof challenge window in seconds
    /// (`protocol_required.challenge_window_seconds`). `Some` for L2.
    pub challenge_window_seconds: Option<u64>,
    /// Expected EIP-155 chain id for the RPC endpoint.
    /// When `Some`, the observer asserts the live chain returns this id and
    /// fails closed on mismatch — prevents accidental cross-network minting.
    pub expected_eip155_chain_id: Option<u64>,
}

impl EvmChainConfig {
    /// Ethereum mainnet — L1, `native_canonical`, 12-confirmation reorg safety.
    /// Mirrors `chain_registry.json` → `ETH` and `origins.json` → `ETH`.
    pub fn eth() -> Self {
        Self {
            origin_code: "ETH".to_string(),
            chain_id: "ETH".to_string(),
            lifecycle_prefix: "eth".to_string(),
            layer: 1,
            settlement_chain: None,
            regime: EvmRegime::NativeCanonical,
            reorg_safety_confirmations: Some(12),
            challenge_window_seconds: None,
            expected_eip155_chain_id: Some(1),
        }
    }

    /// Base — Coinbase optimistic L2 on Ethereum, 7-day challenge window.
    /// Mirrors `chain_registry.json` → `BASE` and `origins.json` → `BSE`.
    /// `BASE` is 4 chars and cannot be a 3-char Arrow Origin; the registered
    /// code is `BSE`.
    pub fn base() -> Self {
        Self {
            origin_code: "BSE".to_string(),
            chain_id: "BASE".to_string(),
            lifecycle_prefix: "base".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: Some(8453),
        }
    }

    /// Arbitrum One — optimistic L2 (BoLD) on Ethereum, 7-day challenge window.
    /// Mirrors `chain_registry.json` → `ARB` and `origins.json` → `ARB`.
    pub fn arb() -> Self {
        Self {
            origin_code: "ARB".to_string(),
            chain_id: "ARB".to_string(),
            lifecycle_prefix: "arb".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: Some(42161),
        }
    }

    /// Base Sepolia testnet — optimistic L2 for testnet lifecycle testing.
    /// `derivation_eligible: false`; chain id 84532.
    pub fn base_sepolia() -> Self {
        Self {
            origin_code: "BSE".to_string(),
            chain_id: "BASE".to_string(),
            lifecycle_prefix: "base-sepolia".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            // Testnet challenge window is near-zero; stored for structural
            // completeness but must not be cited as mainnet proof.
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: Some(84532),
        }
    }

    /// Arbitrum Sepolia testnet — optimistic L2 for testnet lifecycle testing.
    /// `derivation_eligible: false`; chain id 421614.
    pub fn arb_sepolia() -> Self {
        Self {
            origin_code: "ARB".to_string(),
            chain_id: "ARB".to_string(),
            lifecycle_prefix: "arb-sepolia".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: Some(604800),
            expected_eip155_chain_id: Some(421614),
        }
    }

    /// True when this chain reaches finality by confirmation depth (L1).
    pub fn is_layer1(&self) -> bool {
        self.layer == 1
            && self.reorg_safety_confirmations.is_some()
            && self.challenge_window_seconds.is_none()
    }

    /// True when this chain settles through an L2 challenge window.
    pub fn is_optimistic_l2(&self) -> bool {
        self.layer == 2 && self.challenge_window_seconds.is_some()
    }

    /// Fail-closed check that the config maps to exactly one coherent finality
    /// model. Because the fields are public, manually-built configs (e.g. the
    /// Base/Arbitrum wrappers) must call this — or [`crate::mapping::try_map_to_arrow`],
    /// which calls it — rather than risk a panic in mapping.
    pub fn validate(&self) -> Result<(), EvmConfigError> {
        if self.is_layer1() || self.is_optimistic_l2() {
            Ok(())
        } else {
            Err(EvmConfigError::InvalidFinalityConfig {
                origin_code: self.origin_code.clone(),
                layer: self.layer,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eth_config_is_valid_l1() {
        let cfg = EvmChainConfig::eth();
        assert!(cfg.validate().is_ok());
        assert!(cfg.is_layer1());
        assert!(!cfg.is_optimistic_l2());
    }

    #[test]
    fn base_config_is_valid_optimistic_l2() {
        let cfg = EvmChainConfig::base();
        assert!(cfg.validate().is_ok());
        assert!(!cfg.is_layer1());
        assert!(cfg.is_optimistic_l2());
        assert_eq!(cfg.origin_code, "BSE");
        assert_eq!(cfg.chain_id, "BASE");
        assert_eq!(cfg.challenge_window_seconds, Some(604800));
    }

    #[test]
    fn arb_config_is_valid_optimistic_l2() {
        let cfg = EvmChainConfig::arb();
        assert!(cfg.validate().is_ok());
        assert!(!cfg.is_layer1());
        assert!(cfg.is_optimistic_l2());
        assert_eq!(cfg.origin_code, "ARB");
        assert_eq!(cfg.chain_id, "ARB");
        assert_eq!(cfg.challenge_window_seconds, Some(604800));
    }

    #[test]
    fn l1_without_reorg_safety_is_invalid() {
        let mut cfg = EvmChainConfig::eth();
        cfg.reorg_safety_confirmations = None;
        assert_eq!(
            cfg.validate(),
            Err(EvmConfigError::InvalidFinalityConfig {
                origin_code: "ETH".to_string(),
                layer: 1,
            })
        );
    }

    #[test]
    fn l2_without_challenge_window_is_invalid() {
        let cfg = EvmChainConfig {
            origin_code: "BSE".to_string(),
            chain_id: "BASE".to_string(),
            lifecycle_prefix: "base".to_string(),
            layer: 2,
            settlement_chain: Some("ETH".to_string()),
            regime: EvmRegime::OptimisticSettled,
            reorg_safety_confirmations: None,
            challenge_window_seconds: None,
            expected_eip155_chain_id: None,
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn base_sepolia_config_valid_optimistic_l2() {
        let cfg = EvmChainConfig::base_sepolia();
        assert!(cfg.validate().is_ok());
        assert!(cfg.is_optimistic_l2());
        assert_eq!(cfg.expected_eip155_chain_id, Some(84532));
    }

    #[test]
    fn arb_sepolia_config_valid_optimistic_l2() {
        let cfg = EvmChainConfig::arb_sepolia();
        assert!(cfg.validate().is_ok());
        assert!(cfg.is_optimistic_l2());
        assert_eq!(cfg.expected_eip155_chain_id, Some(421614));
    }
}
