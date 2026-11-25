use arrow_transport::envelope::TransportEnvelope;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::intent::{ActionIntent, SourceValueTransfer, SourceValueTransferError};

pub const ADAPTER_INTERCHANGE_VERSION: &str = "arrow-adapter-interchange-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub adapter_id: String,
    pub adapter_kind: AdapterKind,
    pub protocol: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    Origin,
    Consumer,
}

/// Normalized Arrow Adapter Interchange Contract.
///
/// The envelope remains the signed transport object. Origin adapters attach
/// normalized source facts; consumer adapters may later attach a typed action
/// intent when crossing into a destination-specific execution boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInterchangeContract {
    pub contract_version: String,
    pub origin_adapter: AdapterInfo,
    pub issued_at: String,
    pub envelope: TransportEnvelope,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_value_transfer: Option<SourceValueTransfer>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub action_intent: Option<ActionIntent>,
}

impl AdapterInterchangeContract {
    pub fn from_envelope(
        origin_adapter: AdapterInfo,
        envelope: TransportEnvelope,
    ) -> Result<Self, SourceValueTransferError> {
        let source_value_transfer = Some(SourceValueTransfer::from_event(&envelope.payload)?);
        Ok(Self {
            contract_version: ADAPTER_INTERCHANGE_VERSION.to_string(),
            origin_adapter,
            issued_at: Utc::now().to_rfc3339(),
            envelope,
            source_value_transfer,
            action_intent: None,
        })
    }

    pub fn from_envelope_without_action(
        origin_adapter: AdapterInfo,
        envelope: TransportEnvelope,
    ) -> Self {
        Self {
            contract_version: ADAPTER_INTERCHANGE_VERSION.to_string(),
            origin_adapter,
            issued_at: Utc::now().to_rfc3339(),
            envelope,
            source_value_transfer: None,
            action_intent: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::types::event::ArrowEvent;
    use arrow_transport::envelope::ReceiverId;
    use chrono::DateTime;

    fn adapter_info() -> AdapterInfo {
        AdapterInfo {
            adapter_id: "origin-evm-test".to_string(),
            adapter_kind: AdapterKind::Origin,
            protocol: "evm".to_string(),
            version: "0.1.0".to_string(),
        }
    }

    fn envelope_with_payload(payload: ArrowEvent) -> TransportEnvelope {
        TransportEnvelope {
            envelope_version: "arrow-transport-envelope-v1".to_string(),
            envelope_id: "env-1".to_string(),
            sender_id: "origin-evm-test".to_string(),
            receiver_id: ReceiverId::One("consumer-evm-test".to_string()),
            channel_binding: "http-rest".to_string(),
            sent_at: "2026-06-07T00:00:00Z".to_string(),
            nonce: "nonce-1".to_string(),
            sequence: Some(1),
            signature: None,
            signing: None,
            correlation_id: Some("corr-1".to_string()),
            reply_to: None,
            metadata: None,
            payload,
        }
    }

    fn event_with_source_transfer() -> (ArrowEvent, SourceValueTransfer) {
        let mut event = ArrowEvent {
            event_id: "evt-1".to_string(),
            chain_id: Some("ETH".to_string()),
            ..Default::default()
        };
        let transfer = SourceValueTransfer::evm_native(
            "ETH",
            "0xsource",
            "0xfrom",
            "0xto",
            "1000000000000000000",
            Some(12),
        );
        transfer
            .attach_to_event(&mut event)
            .expect("source transfer should serialize");
        (event, transfer)
    }

    #[test]
    fn adapter_kind_serializes_as_snake_case() {
        assert_eq!(serde_json::to_value(AdapterKind::Origin).unwrap(), "origin");
        assert_eq!(
            serde_json::to_value(AdapterKind::Consumer).unwrap(),
            "consumer"
        );
    }

    #[test]
    fn from_envelope_extracts_typed_source_value_transfer() {
        let origin_adapter = adapter_info();
        let (event, transfer) = event_with_source_transfer();
        let envelope = envelope_with_payload(event);

        let contract = AdapterInterchangeContract::from_envelope(origin_adapter.clone(), envelope)
            .expect("typed source transfer should produce contract");

        assert_eq!(contract.contract_version, ADAPTER_INTERCHANGE_VERSION);
        assert_eq!(contract.origin_adapter, origin_adapter);
        assert_eq!(contract.envelope.envelope_id, "env-1");
        assert_eq!(contract.envelope.payload.event_id, "evt-1");
        assert_eq!(contract.source_value_transfer, Some(transfer));
        assert_eq!(contract.action_intent, None);
        DateTime::parse_from_rfc3339(&contract.issued_at).expect("issued_at should be RFC3339");
    }

    #[test]
    fn from_envelope_requires_typed_source_value_transfer() {
        let envelope = envelope_with_payload(ArrowEvent {
            event_id: "evt-1".to_string(),
            ..Default::default()
        });

        let err = AdapterInterchangeContract::from_envelope(adapter_info(), envelope)
            .expect_err("missing source transfer should fail closed");

        assert!(matches!(
            err,
            SourceValueTransferError::MissingExtensions(event_id) if event_id == "evt-1"
        ));
    }

    #[test]
    fn from_envelope_without_action_preserves_envelope_without_requiring_transfer() {
        let origin_adapter = adapter_info();
        let envelope = envelope_with_payload(ArrowEvent {
            event_id: "evt-1".to_string(),
            ..Default::default()
        });

        let contract = AdapterInterchangeContract::from_envelope_without_action(
            origin_adapter.clone(),
            envelope,
        );

        assert_eq!(contract.contract_version, ADAPTER_INTERCHANGE_VERSION);
        assert_eq!(contract.origin_adapter, origin_adapter);
        assert_eq!(contract.envelope.envelope_id, "env-1");
        assert_eq!(contract.envelope.payload.event_id, "evt-1");
        assert_eq!(contract.source_value_transfer, None);
        assert_eq!(contract.action_intent, None);
        DateTime::parse_from_rfc3339(&contract.issued_at).expect("issued_at should be RFC3339");
    }
}
