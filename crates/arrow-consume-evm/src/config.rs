//! Per-chain configuration for the EVM-family Consumption Adapter.
//!
//! The consume/settle logic is identical across EVM destinations; only the
//! adapter identity, the destination chain id stamped into the typed
//! [`arrow_adapter::ActionIntent`], and the expected source unit differ. This
//! struct carries those, so a concrete consumer (e.g. `arrow-consume-eth`) is a
//! thin shell.

/// Static description of one EVM-compatible settlement destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmConsumerConfig {
    /// Stable adapter id recorded on every [`arrow_adapter::ConsumerReceipt`],
    /// e.g. `arrow-consume-eth`.
    pub adapter_id: String,
    /// Destination chain id stamped into the typed action intent, e.g. `ETH`.
    pub destination_chain_id: String,
    /// Expected source-transfer unit this consumer can settle, e.g. `wei`.
    pub expected_source_unit: String,
}

impl EvmConsumerConfig {
    /// Ethereum settlement destination (the original `arrow-consume-eth`).
    pub fn eth() -> Self {
        Self {
            adapter_id: "arrow-consume-eth".to_string(),
            destination_chain_id: "ETH".to_string(),
            expected_source_unit: "wei".to_string(),
        }
    }

    /// Base settlement destination. RecordOnly in v0.1 — live settlement
    /// requires the challenge window to elapse and a settlement watcher.
    pub fn base() -> Self {
        Self {
            adapter_id: "arrow-consume-base".to_string(),
            destination_chain_id: "BASE".to_string(),
            expected_source_unit: "wei".to_string(),
        }
    }

    /// Arbitrum One settlement destination. RecordOnly in v0.1.
    pub fn arb() -> Self {
        Self {
            adapter_id: "arrow-consume-arb".to_string(),
            destination_chain_id: "ARB".to_string(),
            expected_source_unit: "wei".to_string(),
        }
    }
}
