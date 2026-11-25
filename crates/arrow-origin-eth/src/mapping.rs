//! Ethereum confirmation→Arrow-code mapping.
//!
//! Ethereum is an `EvmChainConfig::eth()` instance over the shared
//! [`arrow_origin_evm::mapping`] core. This module preserves the original
//! `arrow-origin-eth` public surface — [`ObservedTx`],
//! [`ETH_REORG_SAFETY_CONFIRMATIONS`], and [`map_to_arrow`] — by re-exporting and
//! thinly wrapping the library, so callers and tests are unaffected.

use arrow_core::types::event::ArrowEvent;
use arrow_origin_evm::config::EvmChainConfig;

pub use arrow_origin_evm::mapping::{ObservedTx, ETH_REORG_SAFETY_CONFIRMATIONS};

/// Map an observed Ethereum transaction to a protocol-valid [`ArrowEvent`].
///
/// `reorg_safety` is the confirmation depth at which the event is considered
/// final — pass [`ETH_REORG_SAFETY_CONFIRMATIONS`] for Ethereum mainnet. This
/// delegates to the shared EVM mapping with the Ethereum chain config.
///
/// Note: chain-id validation is cleared here so this function works with any
/// EVM node (incl. Anvil). For strict mainnet chain-id enforcement, use
/// [`arrow_origin_evm::mapping::try_map_to_arrow`] directly with a full config.
pub fn map_to_arrow(obs: &ObservedTx, reorg_safety: u64) -> ArrowEvent {
    let mut config = EvmChainConfig::eth();
    config.reorg_safety_confirmations = Some(reorg_safety);
    // Clear expected chain id for this compatibility wrapper — chain-id
    // enforcement is the responsibility of the full lifecycle harness.
    config.expected_eip155_chain_id = None;
    arrow_origin_evm::mapping::map_to_arrow(obs, &config)
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
            observed_chain_id: 1, // Ethereum mainnet
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
    fn confirmations_counts_head_inclusive() {
        // tx in block 100, head at 100 → 1 confirmation.
        assert_eq!(obs_with(Some(100), 100).confirmations(), 1);
        // tx in block 100, head at 111 → 12 confirmations.
        assert_eq!(obs_with(Some(100), 111).confirmations(), 12);
        // not yet mined.
        assert_eq!(obs_with(None, 100).confirmations(), 0);
    }

    #[test]
    fn zero_confirmations_is_pending() {
        let ev = map_to_arrow(&obs_with(None, 100), ETH_REORG_SAFETY_CONFIRMATIONS);
        assert_eq!(ev.arrow_code, "1:ETH:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("probabilistic"));
        assert_eq!(ev.finality_eta, Some(12));
        assert_validates(&ev);
    }

    #[test]
    fn eleven_confirmations_still_pending() {
        // tx in block 100, head 110 → 11 confirmations (< 12).
        let ev = map_to_arrow(&obs_with(Some(100), 110), ETH_REORG_SAFETY_CONFIRMATIONS);
        assert_eq!(ev.arrow_code, "1:ETH:PD:HL:0:B");
        assert_eq!(ev.finality_eta, Some(1));
        assert_validates(&ev);
    }

    #[test]
    fn twelve_confirmations_is_final() {
        // tx in block 100, head 111 → 12 confirmations (== threshold).
        let ev = map_to_arrow(&obs_with(Some(100), 111), ETH_REORG_SAFETY_CONFIRMATIONS);
        assert_eq!(ev.arrow_code, "1:ETH:FN:PR:1:A");
        assert!(ev.finality_basis.is_none());
        assert_validates(&ev);
    }

    #[test]
    fn deep_confirmations_is_final() {
        let ev = map_to_arrow(&obs_with(Some(100), 200), ETH_REORG_SAFETY_CONFIRMATIONS);
        assert_eq!(ev.arrow_code, "1:ETH:FN:PR:1:A");
        assert_validates(&ev);
    }
}
