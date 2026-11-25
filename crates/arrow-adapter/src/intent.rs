use std::collections::HashMap;

use arrow_core::types::event::ArrowEvent;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ACTION_INTENT_EXTENSION_KEY: &str = "arrow.adapter.action_intent.v1";
pub const ACTION_INTENT_VERSION: &str = "1.0";
pub const SOURCE_VALUE_TRANSFER_EXTENSION_KEY: &str = "arrow.adapter.source_value_transfer.v1";
pub const SOURCE_VALUE_TRANSFER_VERSION: &str = "1.0";

/// Typed downstream action intent produced by Consumer Adapters.
///
/// Origins should emit normalized source facts and avoid deciding where the
/// event will be consumed. Consumers create an `ActionIntent` only when applying
/// an authorized destination action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionIntent {
    pub intent_version: String,
    pub intent_id: String,
    pub action: ActionKind,
    pub destination: Destination,
    pub asset: Asset,
    pub amount: IntentAmount,
    pub source_event_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_proof_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    TransferValue,
    ContractCall,
    RecordOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Destination {
    /// Destination execution environment, e.g. `evm`.
    pub system: String,
    /// Destination chain or registry id, e.g. `ETH`.
    pub chain_id: String,
    /// Destination account/contract address in the destination system.
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// `native`, `erc20`, `spl`, etc.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub contract: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentAmount {
    /// Decimal string to avoid precision loss for chain-native integer amounts.
    pub value: String,
    /// Unit in the destination system, e.g. `wei`.
    pub unit: String,
}

/// Normalized source-side value transfer fact emitted by Origin Adapters.
///
/// This is not a destination action. It describes what was honestly observed in
/// the source domain so consumers can parse the lifecycle without reaching back
/// into Ethereum-specific raw objects or free-form metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceValueTransfer {
    pub transfer_version: String,
    pub source_system: String,
    pub source_chain_id: String,
    pub source_tx: String,
    pub from: String,
    pub to: String,
    pub asset: Asset,
    pub amount: IntentAmount,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub confirmations: Option<u64>,
}

/// Returned by [`ActionIntent::validate_destination`] when the intent's
/// destination system or amount unit does not match what a consumer expects.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DestinationMismatchError {
    #[error("intent destination system `{got}` does not match expected `{expected}`")]
    SystemMismatch { expected: String, got: String },
    #[error("intent amount unit `{got}` does not match expected `{expected}`")]
    UnitMismatch { expected: String, got: String },
}

#[derive(Debug, Error)]
pub enum ActionIntentError {
    #[error("event `{0}` has no extensions; typed action intent is required")]
    MissingExtensions(String),
    #[error(
        "event `{0}` is missing typed action intent extension `{ACTION_INTENT_EXTENSION_KEY}`"
    )]
    MissingIntent(String),
    #[error("event `{event_id}` has malformed action intent: {source}")]
    Malformed {
        event_id: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("event `{event_id}` action intent version `{version}` is unsupported")]
    UnsupportedVersion { event_id: String, version: String },
}

#[derive(Debug, Error)]
pub enum SourceValueTransferError {
    #[error("event `{0}` has no extensions; typed source value transfer is required")]
    MissingExtensions(String),
    #[error("event `{0}` is missing typed source value transfer extension `{SOURCE_VALUE_TRANSFER_EXTENSION_KEY}`")]
    MissingTransfer(String),
    #[error("event `{event_id}` has malformed source value transfer: {source}")]
    Malformed {
        event_id: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("event `{event_id}` source value transfer version `{version}` is unsupported")]
    UnsupportedVersion { event_id: String, version: String },
}

impl ActionIntent {
    pub fn evm_value_transfer(
        intent_id: impl Into<String>,
        destination_chain_id: impl Into<String>,
        to: impl Into<String>,
        value_wei: impl Into<String>,
        source_event_id: impl Into<String>,
        source_proof_ref: Option<String>,
    ) -> Self {
        Self {
            intent_version: ACTION_INTENT_VERSION.to_string(),
            intent_id: intent_id.into(),
            action: ActionKind::TransferValue,
            destination: Destination {
                system: "evm".to_string(),
                chain_id: destination_chain_id.into(),
                address: to.into(),
            },
            asset: Asset {
                kind: "native".to_string(),
                symbol: Some("ETH".to_string()),
                contract: None,
            },
            amount: IntentAmount {
                value: value_wei.into(),
                unit: "wei".to_string(),
            },
            source_event_id: source_event_id.into(),
            source_proof_ref,
            memo: None,
        }
    }

    /// Bitcoin native-asset transfer intent. `destination.system = "bitcoin"`,
    /// `amount.unit = "sats"`. Used by `arrow-consume-btc`; never executed in v0.1
    /// (RecordOnly/DryRunOnly — no UTXO selection, signing, or broadcast).
    pub fn btc_transfer(
        intent_id: impl Into<String>,
        destination_chain_id: impl Into<String>,
        to: impl Into<String>,
        amount_sats: impl Into<String>,
        source_event_id: impl Into<String>,
        source_proof_ref: Option<String>,
    ) -> Self {
        Self {
            intent_version: ACTION_INTENT_VERSION.to_string(),
            intent_id: intent_id.into(),
            action: ActionKind::TransferValue,
            destination: Destination {
                system: "bitcoin".to_string(),
                chain_id: destination_chain_id.into(),
                address: to.into(),
            },
            asset: Asset {
                kind: "native".to_string(),
                symbol: Some("BTC".to_string()),
                contract: None,
            },
            amount: IntentAmount {
                value: amount_sats.into(),
                unit: "sats".to_string(),
            },
            source_event_id: source_event_id.into(),
            source_proof_ref,
            memo: None,
        }
    }

    /// Solana native-asset transfer intent. `destination.system = "solana"`,
    /// `amount.unit = "lamports"`. Used by `arrow-consume-sol`; never executed in
    /// v0.1 (RecordOnly/DryRunOnly — no instruction building, signing, or send).
    pub fn sol_transfer(
        intent_id: impl Into<String>,
        destination_chain_id: impl Into<String>,
        to: impl Into<String>,
        amount_lamports: impl Into<String>,
        source_event_id: impl Into<String>,
        source_proof_ref: Option<String>,
    ) -> Self {
        Self {
            intent_version: ACTION_INTENT_VERSION.to_string(),
            intent_id: intent_id.into(),
            action: ActionKind::TransferValue,
            destination: Destination {
                system: "solana".to_string(),
                chain_id: destination_chain_id.into(),
                address: to.into(),
            },
            asset: Asset {
                kind: "native".to_string(),
                symbol: Some("SOL".to_string()),
                contract: None,
            },
            amount: IntentAmount {
                value: amount_lamports.into(),
                unit: "lamports".to_string(),
            },
            source_event_id: source_event_id.into(),
            source_proof_ref,
            memo: None,
        }
    }

    /// Fail-closed destination-model check called by consumers before any action.
    ///
    /// Returns a typed [`DestinationMismatchError`] when this intent's
    /// `destination.system` or `amount.unit` does not match the consumer's
    /// expected model. Consumers call this so incompatible intents fail with a
    /// named error rather than a panic or silent coercion (cross-product suite).
    pub fn validate_destination(
        &self,
        expected_system: &str,
        expected_unit: &str,
    ) -> Result<(), DestinationMismatchError> {
        if self.destination.system != expected_system {
            return Err(DestinationMismatchError::SystemMismatch {
                expected: expected_system.to_string(),
                got: self.destination.system.clone(),
            });
        }
        if self.amount.unit != expected_unit {
            return Err(DestinationMismatchError::UnitMismatch {
                expected: expected_unit.to_string(),
                got: self.amount.unit.clone(),
            });
        }
        Ok(())
    }

    pub fn attach_to_event(&self, event: &mut ArrowEvent) -> Result<(), serde_json::Error> {
        let mut extensions = event.extensions.take().unwrap_or_default();
        extensions.insert(
            ACTION_INTENT_EXTENSION_KEY.to_string(),
            serde_json::to_value(self)?,
        );
        event.extensions = Some(extensions);
        Ok(())
    }

    pub fn from_event(event: &ArrowEvent) -> Result<Self, ActionIntentError> {
        let extensions = event
            .extensions
            .as_ref()
            .ok_or_else(|| ActionIntentError::MissingExtensions(event.event_id.clone()))?;
        let value = extensions
            .get(ACTION_INTENT_EXTENSION_KEY)
            .ok_or_else(|| ActionIntentError::MissingIntent(event.event_id.clone()))?;
        let intent: ActionIntent = serde_json::from_value(value.clone()).map_err(|source| {
            ActionIntentError::Malformed {
                event_id: event.event_id.clone(),
                source,
            }
        })?;
        if intent.intent_version != ACTION_INTENT_VERSION {
            return Err(ActionIntentError::UnsupportedVersion {
                event_id: event.event_id.clone(),
                version: intent.intent_version,
            });
        }
        Ok(intent)
    }

    pub fn extension_map(&self) -> Result<HashMap<String, serde_json::Value>, serde_json::Error> {
        let mut map = HashMap::new();
        map.insert(
            ACTION_INTENT_EXTENSION_KEY.to_string(),
            serde_json::to_value(self)?,
        );
        Ok(map)
    }
}

impl SourceValueTransfer {
    pub fn evm_native(
        source_chain_id: impl Into<String>,
        source_tx: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        value_wei: impl Into<String>,
        confirmations: Option<u64>,
    ) -> Self {
        Self {
            transfer_version: SOURCE_VALUE_TRANSFER_VERSION.to_string(),
            source_system: "evm".to_string(),
            source_chain_id: source_chain_id.into(),
            source_tx: source_tx.into(),
            from: from.into(),
            to: to.into(),
            asset: Asset {
                kind: "native".to_string(),
                symbol: Some("ETH".to_string()),
                contract: None,
            },
            amount: IntentAmount {
                value: value_wei.into(),
                unit: "wei".to_string(),
            },
            confirmations,
        }
    }

    /// Bitcoin native-asset source fact. `source_system = "bitcoin"`,
    /// `asset.symbol = Some("BTC")`, `amount.unit = "sats"`.
    pub fn btc_native(
        source_chain_id: impl Into<String>,
        source_tx: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        amount_sats: impl Into<String>,
        confirmations: Option<u64>,
    ) -> Self {
        Self {
            transfer_version: SOURCE_VALUE_TRANSFER_VERSION.to_string(),
            source_system: "bitcoin".to_string(),
            source_chain_id: source_chain_id.into(),
            source_tx: source_tx.into(),
            from: from.into(),
            to: to.into(),
            asset: Asset {
                kind: "native".to_string(),
                symbol: Some("BTC".to_string()),
                contract: None,
            },
            amount: IntentAmount {
                value: amount_sats.into(),
                unit: "sats".to_string(),
            },
            confirmations,
        }
    }

    /// Solana native-asset source fact. `source_system = "solana"`,
    /// `asset.symbol = Some("SOL")`, `amount.unit = "lamports"`.
    pub fn sol_native(
        source_chain_id: impl Into<String>,
        source_tx: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        amount_lamports: impl Into<String>,
        confirmations: Option<u64>,
    ) -> Self {
        Self {
            transfer_version: SOURCE_VALUE_TRANSFER_VERSION.to_string(),
            source_system: "solana".to_string(),
            source_chain_id: source_chain_id.into(),
            source_tx: source_tx.into(),
            from: from.into(),
            to: to.into(),
            asset: Asset {
                kind: "native".to_string(),
                symbol: Some("SOL".to_string()),
                contract: None,
            },
            amount: IntentAmount {
                value: amount_lamports.into(),
                unit: "lamports".to_string(),
            },
            confirmations,
        }
    }

    pub fn attach_to_event(&self, event: &mut ArrowEvent) -> Result<(), serde_json::Error> {
        let mut extensions = event.extensions.take().unwrap_or_default();
        extensions.insert(
            SOURCE_VALUE_TRANSFER_EXTENSION_KEY.to_string(),
            serde_json::to_value(self)?,
        );
        event.extensions = Some(extensions);
        Ok(())
    }

    pub fn from_event(event: &ArrowEvent) -> Result<Self, SourceValueTransferError> {
        let extensions = event
            .extensions
            .as_ref()
            .ok_or_else(|| SourceValueTransferError::MissingExtensions(event.event_id.clone()))?;
        let value = extensions
            .get(SOURCE_VALUE_TRANSFER_EXTENSION_KEY)
            .ok_or_else(|| SourceValueTransferError::MissingTransfer(event.event_id.clone()))?;
        let transfer: SourceValueTransfer =
            serde_json::from_value(value.clone()).map_err(|source| {
                SourceValueTransferError::Malformed {
                    event_id: event.event_id.clone(),
                    source,
                }
            })?;
        if transfer.transfer_version != SOURCE_VALUE_TRANSFER_VERSION {
            return Err(SourceValueTransferError::UnsupportedVersion {
                event_id: event.event_id.clone(),
                version: transfer.transfer_version,
            });
        }
        Ok(transfer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event_id: &str) -> ArrowEvent {
        ArrowEvent {
            event_id: event_id.to_string(),
            ..Default::default()
        }
    }

    fn evm_intent() -> ActionIntent {
        ActionIntent::evm_value_transfer(
            "intent-1",
            "ETH",
            "0x2222222222222222222222222222222222222222",
            "100",
            "evt-1",
            Some("0xabc".to_string()),
        )
    }

    fn evm_transfer() -> SourceValueTransfer {
        SourceValueTransfer::evm_native(
            "ETH",
            "0xabc",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "100",
            Some(12),
        )
    }

    #[test]
    fn action_intent_from_event_requires_extensions() {
        let err = ActionIntent::from_event(&event("evt-no-extensions"))
            .expect_err("missing extensions must fail closed");

        assert!(matches!(
            err,
            ActionIntentError::MissingExtensions(event_id) if event_id == "evt-no-extensions"
        ));
    }

    #[test]
    fn action_intent_from_event_requires_typed_extension_key() {
        let mut event = event("evt-missing-intent");
        evm_transfer().attach_to_event(&mut event).unwrap();

        let err = ActionIntent::from_event(&event)
            .expect_err("missing action intent key must fail closed");

        assert!(matches!(
            err,
            ActionIntentError::MissingIntent(event_id) if event_id == "evt-missing-intent"
        ));
    }

    #[test]
    fn action_intent_from_event_rejects_malformed_extension() {
        let mut event = event("evt-malformed-intent");
        event.extensions = Some(HashMap::from([(
            ACTION_INTENT_EXTENSION_KEY.to_string(),
            serde_json::json!({"intent_version": ACTION_INTENT_VERSION}),
        )]));

        let err =
            ActionIntent::from_event(&event).expect_err("partial action intent must be malformed");

        assert!(matches!(
            err,
            ActionIntentError::Malformed { event_id, .. } if event_id == "evt-malformed-intent"
        ));
    }

    #[test]
    fn action_intent_from_event_rejects_unsupported_version() {
        let mut intent = evm_intent();
        intent.intent_version = "2.0".to_string();
        let mut event = event("evt-unsupported-intent");
        intent.attach_to_event(&mut event).unwrap();

        let err = ActionIntent::from_event(&event)
            .expect_err("unsupported action intent version must fail closed");

        assert!(matches!(
            err,
            ActionIntentError::UnsupportedVersion { event_id, version }
                if event_id == "evt-unsupported-intent" && version == "2.0"
        ));
    }

    #[test]
    fn source_value_transfer_from_event_requires_typed_extension_key() {
        let mut event = event("evt-missing-transfer");
        evm_intent().attach_to_event(&mut event).unwrap();

        let err = SourceValueTransfer::from_event(&event)
            .expect_err("missing source transfer key must fail closed");

        assert!(matches!(
            err,
            SourceValueTransferError::MissingTransfer(event_id) if event_id == "evt-missing-transfer"
        ));
    }

    #[test]
    fn source_value_transfer_from_event_rejects_malformed_extension() {
        let mut event = event("evt-malformed-transfer");
        event.extensions = Some(HashMap::from([(
            SOURCE_VALUE_TRANSFER_EXTENSION_KEY.to_string(),
            serde_json::json!({"transfer_version": SOURCE_VALUE_TRANSFER_VERSION}),
        )]));

        let err = SourceValueTransfer::from_event(&event)
            .expect_err("partial source transfer must be malformed");

        assert!(matches!(
            err,
            SourceValueTransferError::Malformed { event_id, .. }
                if event_id == "evt-malformed-transfer"
        ));
    }

    #[test]
    fn source_value_transfer_from_event_rejects_unsupported_version() {
        let mut transfer = evm_transfer();
        transfer.transfer_version = "2.0".to_string();
        let mut event = event("evt-unsupported-transfer");
        transfer.attach_to_event(&mut event).unwrap();

        let err = SourceValueTransfer::from_event(&event)
            .expect_err("unsupported source transfer version must fail closed");

        assert!(matches!(
            err,
            SourceValueTransferError::UnsupportedVersion { event_id, version }
                if event_id == "evt-unsupported-transfer" && version == "2.0"
        ));
    }

    #[test]
    fn validate_destination_rejects_wrong_amount_unit() {
        let mut intent = evm_intent();
        intent.amount.unit = "gwei".to_string();

        let err = intent
            .validate_destination("evm", "wei")
            .expect_err("wrong amount unit must be a typed mismatch");

        assert_eq!(
            err,
            DestinationMismatchError::UnitMismatch {
                expected: "wei".to_string(),
                got: "gwei".to_string(),
            }
        );
    }

    #[test]
    fn validate_destination_rejects_wrong_destination_system() {
        let mut intent = evm_intent();
        intent.destination.system = "bitcoin".to_string();

        let err = intent
            .validate_destination("evm", "wei")
            .expect_err("wrong destination system must be a typed mismatch");

        assert_eq!(
            err,
            DestinationMismatchError::SystemMismatch {
                expected: "evm".to_string(),
                got: "bitcoin".to_string(),
            }
        );
    }
}
