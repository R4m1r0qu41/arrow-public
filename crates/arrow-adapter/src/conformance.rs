use arrow_transport::{envelope::TransportEnvelope, replay::ReplayStore};

use crate::{
    idempotency::{destination_execution_id, DestinationExecutionStore},
    intent::{ActionIntent, SourceValueTransfer, SOURCE_VALUE_TRANSFER_EXTENSION_KEY},
    receipt::{
        ConsumerExecutionMode, ConsumerReceipt, ConsumerReceiptStatus, CONSUMER_RECEIPT_VERSION,
    },
};

pub fn assert_event_carries_typed_source_transfer(
    envelope: &TransportEnvelope,
) -> SourceValueTransfer {
    let extensions = envelope
        .payload
        .extensions
        .as_ref()
        .expect("event must carry extensions");
    assert!(
        extensions.contains_key(SOURCE_VALUE_TRANSFER_EXTENSION_KEY),
        "event must carry `{SOURCE_VALUE_TRANSFER_EXTENSION_KEY}`"
    );
    SourceValueTransfer::from_event(&envelope.payload)
        .expect("typed source value transfer must deserialize")
}

pub fn assert_receipt_conforms(
    receipt: &ConsumerReceipt,
    expected_mode: ConsumerExecutionMode,
    expected_status: ConsumerReceiptStatus,
) {
    assert_eq!(receipt.receipt_version, CONSUMER_RECEIPT_VERSION);
    assert_eq!(receipt.mode, expected_mode);
    assert_eq!(receipt.status, expected_status);
    assert!(
        !receipt.envelope_id.trim().is_empty(),
        "receipt must name the envelope"
    );
    assert!(
        !receipt.lifecycle_id.trim().is_empty(),
        "receipt must name the lifecycle"
    );
    assert!(
        !receipt.provenance.signer_did.trim().is_empty(),
        "receipt must include verified provenance"
    );
    if matches!(
        expected_status,
        ConsumerReceiptStatus::DryRun
            | ConsumerReceiptStatus::RecordedOnly
            | ConsumerReceiptStatus::Executed
    ) {
        assert!(
            receipt.action_intent.is_some(),
            "actionable receipts must include typed action intent"
        );
    }
}

pub fn assert_idempotency_actions(
    envelope: &TransportEnvelope,
    intent: &ActionIntent,
    replay: &dyn ReplayStore,
    execution_store: &dyn DestinationExecutionStore,
) {
    assert!(
        !replay
            .check_and_record(&envelope.sender_id, &envelope.nonce)
            .expect("first replay record should succeed"),
        "first replay record must be new"
    );
    assert!(
        replay
            .check_and_record(&envelope.sender_id, &envelope.nonce)
            .expect("second replay record should succeed"),
        "second replay record must be detected"
    );

    let execution_id = destination_execution_id(intent);
    assert!(
        !execution_store
            .check_and_record_execution(&execution_id)
            .expect("first destination execution record should succeed"),
        "first destination execution record must be new"
    );
    assert!(
        execution_store
            .check_and_record_execution(&execution_id)
            .expect("second destination execution record should succeed"),
        "second destination execution record must be detected"
    );
}
