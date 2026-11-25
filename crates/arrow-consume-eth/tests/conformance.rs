use arrow_adapter::{
    conformance::{
        assert_event_carries_typed_source_transfer, assert_idempotency_actions,
        assert_receipt_conforms,
    },
    ConsumerExecutionMode, ConsumerReceiptStatus, InMemoryDestinationExecutionStore,
};
use arrow_consume_eth::{verify_and_consume_with_actions, OperatorConfig, ProvenancePolicy};
use arrow_identity::crypto::ed25519::generate_test_keypair;
use arrow_identity::resolver::did_key::did_key_from_public;
use arrow_origin_eth::emit::build_signed_envelope;
use arrow_origin_eth::mapping::{map_to_arrow, ObservedTx, ETH_REORG_SAFETY_CONFIRMATIONS};
use arrow_transport::replay::InMemoryReplayStore;
use base64::Engine;

fn write_dev_key(path: &std::path::Path) -> String {
    let (sk, pk) = generate_test_keypair();
    let did = did_key_from_public(&pk);
    let suffix = did.strip_prefix("did:key:").unwrap();
    let key_ref = format!("{did}#{suffix}");
    let sk_b64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(sk);
    let json = serde_json::json!({
        "did": did,
        "key_ref": key_ref,
        "algorithm": "Ed25519",
        "private_key_b64": sk_b64,
    });
    std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    did
}

#[test]
fn eth_origin_to_eth_consumer_conforms_to_adapter_boundary() {
    let observed = ObservedTx {
        tx_hash: "0xabc123".to_string(),
        block_number: Some(100),
        head_block: 111,
        from: "0x1111111111111111111111111111111111111111".to_string(),
        to: Some("0x2222222222222222222222222222222222222222".to_string()),
        value_wei: "1000000000000000000".to_string(),
        observed_chain_id: 1,
        block_timestamp: None,
    };
    let event = map_to_arrow(&observed, ETH_REORG_SAFETY_CONFIRMATIONS);

    let key_path = std::env::temp_dir().join("arrow-adapter-conformance-key.json");
    let sender_did = write_dev_key(&key_path);
    let envelope = build_signed_envelope(event, &sender_did, "did:arrow:consumer", Some(&key_path))
        .expect("build signed envelope");
    let _ = std::fs::remove_file(&key_path);

    let source_transfer = assert_event_carries_typed_source_transfer(&envelope);
    assert_eq!(source_transfer.source_system, "evm");
    assert_eq!(source_transfer.amount.value, "1000000000000000000");

    let policy = ProvenancePolicy::new(vec![sender_did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let execution_store = InMemoryDestinationExecutionStore::new();
    let operator = OperatorConfig {
        rpc_url: String::new(),
        from: String::new(),
        ..Default::default()
    };

    let outcome = verify_and_consume_with_actions(
        &envelope,
        &policy,
        &replay,
        &execution_store,
        &operator,
        ConsumerExecutionMode::RecordOnly,
    )
    .expect("record-only consume");

    let receipt = outcome.receipt.as_ref().expect("record-only receipt");
    assert_receipt_conforms(
        receipt,
        ConsumerExecutionMode::RecordOnly,
        ConsumerReceiptStatus::RecordedOnly,
    );
    let intent = receipt.action_intent.as_ref().expect("typed action intent");
    assert_eq!(
        intent.destination.address,
        "0x2222222222222222222222222222222222222222"
    );

    let fresh_replay = InMemoryReplayStore::with_24h_retention();
    let fresh_execution_store = InMemoryDestinationExecutionStore::new();
    assert_idempotency_actions(&envelope, intent, &fresh_replay, &fresh_execution_store);
}
