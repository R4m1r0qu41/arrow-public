//! Arbitrum One (ARB) confirmation → Arrow-code mapping.
//!
//! Arbitrum is an [`arrow_origin_evm::EvmChainConfig::arb()`] instance over
//! the shared EVM mapping core. This module exposes [`try_map_to_arrow`]
//! (fail-closed) — the public API for L2 wrappers whose configs are built
//! at runtime.
//!
//! All Arbitrum events in v0.1 are minted as `1:ARB:PD:HL:0:B` with
//! `finality_basis = optimistic_window`. Block depth on Arbitrum cannot prove
//! the 7-day BoLD challenge window has elapsed. Confidence is capped at B
//! pre-settlement.

use arrow_core::types::event::ArrowEvent;
use arrow_origin_evm::config::EvmChainConfig;
pub use arrow_origin_evm::mapping::ObservedTx;
pub use arrow_origin_evm::EvmConfigError;

/// Map an observed Arbitrum transaction to a protocol-valid [`ArrowEvent`].
///
/// Delegates to [`arrow_origin_evm::mapping::try_map_to_arrow`] (fail-closed)
/// with the [`EvmChainConfig::arb()`] config. Returns a pending event
/// (`1:ARB:PD:HL:0:B`) — block depth on Arbitrum cannot prove the 7-day
/// BoLD challenge window has closed.
pub fn try_map_to_arrow(obs: &ObservedTx) -> Result<ArrowEvent, EvmConfigError> {
    arrow_origin_evm::mapping::try_map_to_arrow(obs, &EvmChainConfig::arb())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::registry::loader::Registries;
    use arrow_core::validation::pipeline::ValidationPipeline;

    fn obs() -> ObservedTx {
        ObservedTx {
            tx_hash: "0xarb123".to_string(),
            block_number: Some(5_000),
            head_block: 1_999_999,
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: Some("0x2222222222222222222222222222222222222222".to_string()),
            value_wei: "500000000000000000".to_string(),
            observed_chain_id: 42161, // Arbitrum One mainnet
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
        let ev = try_map_to_arrow(&obs()).expect("arb() config is always valid");
        assert_eq!(ev.arrow_code, "1:ARB:PD:HL:0:B");
        assert_eq!(ev.finality_basis.as_deref(), Some("optimistic_window"));
        assert_eq!(ev.chain_id.as_deref(), Some("ARB"));
        assert_eq!(ev.lifecycle_id, "arb:0xarb123");
    }

    #[test]
    fn origin_code_is_arb() {
        let ev = try_map_to_arrow(&obs()).expect("arb() config is always valid");
        let parts: Vec<&str> = ev.arrow_code.splitn(6, ':').collect();
        assert_eq!(
            parts[1], "ARB",
            "Origin field must be ARB for Arbitrum events"
        );
    }

    #[test]
    fn pre_settlement_confidence_is_capped_at_b() {
        let ev = try_map_to_arrow(&obs()).expect("arb() config is always valid");
        assert!(
            ev.arrow_code.ends_with(":B"),
            "Confidence must be B pre-settlement; got {}",
            ev.arrow_code
        );
    }

    #[test]
    fn arb_event_validates_against_pipeline() {
        let ev = try_map_to_arrow(&obs()).expect("arb() config is always valid");
        assert_validates(&ev);
    }

    #[test]
    fn deeply_confirmed_arb_tx_is_still_pending() {
        // Even millions of Arbitrum L2 confirmations cannot prove the BoLD
        // challenge window elapsed — the event stays pending.
        let deep = ObservedTx {
            tx_hash: "0xarb456".to_string(),
            block_number: Some(1),
            head_block: 10_000_000,
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: Some("0x2222222222222222222222222222222222222222".to_string()),
            value_wei: "1".to_string(),
            observed_chain_id: 42161,
            block_timestamp: Some(1_700_000_000),
        };
        let ev = try_map_to_arrow(&deep).expect("arb() config is always valid");
        assert_eq!(ev.arrow_code, "1:ARB:PD:HL:0:B");
    }
}
