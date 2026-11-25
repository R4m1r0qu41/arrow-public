//! Base (BSE) confirmation → Arrow-code mapping.
//!
//! Base is an [`arrow_origin_evm::EvmChainConfig::base()`] instance over the
//! shared EVM mapping core. This module exposes [`try_map_to_arrow`] (fail-
//! closed) — the public API for L2 wrappers whose configs are built at runtime.
//!
//! All Base events in v0.1 are minted as `1:BSE:PD:HL:0:B` with
//! `finality_basis = optimistic_window`. Block depth alone does not prove the
//! 7-day challenge window has elapsed. Confidence is capped at B pre-settlement.

use arrow_core::types::event::ArrowEvent;
use arrow_origin_evm::config::EvmChainConfig;
pub use arrow_origin_evm::mapping::ObservedTx;
pub use arrow_origin_evm::EvmConfigError;

/// Map an observed Base transaction to a protocol-valid [`ArrowEvent`].
///
/// Delegates to [`arrow_origin_evm::mapping::try_map_to_arrow`] (fail-closed)
/// with the [`EvmChainConfig::base()`] config. Returns a pending event
/// (`1:BSE:PD:HL:0:B`) — block depth on Base cannot prove the 7-day L1
/// challenge window has closed.
pub fn try_map_to_arrow(obs: &ObservedTx) -> Result<ArrowEvent, EvmConfigError> {
    arrow_origin_evm::mapping::try_map_to_arrow(obs, &EvmChainConfig::base())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::registry::loader::Registries;
    use arrow_core::validation::pipeline::ValidationPipeline;

    fn obs() -> ObservedTx {
        ObservedTx {
            tx_hash: "0xbase123".to_string(),
            block_number: Some(1_000),
            head_block: 999_999,
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: Some("0x2222222222222222222222222222222222222222".to_string()),
            value_wei: "2000000000000000000".to_string(),
            observed_chain_id: 8453, // Base mainnet
            block_timestamp: Some(1_700_000_000),
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
    fn try_map_to_arrow_yields_pending_with_optimistic_window_basis() {
        let ev = try_map_to_arrow(&obs()).expect("base() config is always valid");
        assert_eq!(ev.arrow_code, "1:BSE:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("optimistic_window"));
        assert_eq!(ev.chain_id.as_deref(), Some("BASE"));
        assert_eq!(ev.lifecycle_id, "base:0xbase123");
    }

    #[test]
    fn origin_code_is_bse() {
        let ev = try_map_to_arrow(&obs()).expect("base() config is always valid");
        let parts: Vec<&str> = ev.arrow_code.splitn(6, ':').collect();
        assert_eq!(parts[1], "BSE", "Origin field must be BSE for Base events");
    }

    #[test]
    fn pre_settlement_confidence_is_capped_at_b() {
        let ev = try_map_to_arrow(&obs()).expect("base() config is always valid");
        assert!(
            ev.arrow_code.ends_with(":B"),
            "Confidence must be B pre-settlement; got {}",
            ev.arrow_code
        );
    }

    #[test]
    fn bse_event_validates_against_pipeline() {
        let ev = try_map_to_arrow(&obs()).expect("base() config is always valid");
        assert_validates(&ev);
    }

    #[test]
    fn deeply_confirmed_base_tx_is_still_pending() {
        // Even with millions of L2 confirmations, block depth does not prove the
        // challenge window elapsed — the event stays pending.
        let deep = ObservedTx {
            tx_hash: "0xbase456".to_string(),
            block_number: Some(1),
            head_block: 1_000_000,
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: Some("0x2222222222222222222222222222222222222222".to_string()),
            value_wei: "1".to_string(),
            observed_chain_id: 8453,
            block_timestamp: Some(1_700_000_000),
        };
        let ev = try_map_to_arrow(&deep).expect("base() config is always valid");
        assert_eq!(ev.arrow_code, "1:BSE:PD:HL:0:B");
    }
}
