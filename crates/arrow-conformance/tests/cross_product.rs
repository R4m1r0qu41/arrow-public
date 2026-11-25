//! Cross-adapter conformance suite.
//!
//! Proves the N+N adapter architecture holds across the full 6×6 origin ×
//! consumer matrix. Every cell has an explicit expected outcome:
//!
//! - **Compatible final pairs** produce a conforming receipt (DryRun /
//!   RecordedOnly / Quarantined) — no panic, no silent coercion.
//! - **Incompatible final/actionable pairs** fail with a **named, typed error**
//!   — no panic, no silent settlement.
//! - **Pending pairs** (L2 events, pre-finalized SOL) produce a safe `NoAction`
//!   receipt in any consumer — settlement is never attempted because the
//!   disposition does not permit it. Exception: the LEG consumer rejects
//!   non-LEG origins before checking disposition, so L2→LEG errors even for
//!   pending events.
//!
//! Origins: ETH (L1 final), BSE/ARB (L2 pending), BTC (UTXO final),
//!          SOL (Solana finalized), LEG (judicial order — JUD+FN=1→PR→
//!          ApprovedForSettlement).
//! Consumers: ETH, BASE, ARB (EVM-family), BTC, SOL, LEG.

// ── Imports ──────────────────────────────────────────────────────────────────

use arrow_adapter::{ConsumerExecutionMode, ConsumerReceiptStatus, SourceValueTransfer};
use arrow_consume_leg::{InMemoryReviewQueue, LegReviewDecision};
use arrow_core::types::event::ArrowEvent;
use arrow_identity::crypto::ed25519::{generate_test_keypair, sign_ed25519};
use arrow_identity::resolver::did_key::did_key_from_public;
use arrow_origin_leg::attestation::{map_to_arrow as leg_map, LegalAttestation};
use arrow_transport::{
    canonical::envelope_canonical_bytes,
    envelope::{EnvelopeSigningMetadata, ReceiverId, TransportEnvelope},
    replay::InMemoryReplayStore,
    signature::{CANONICALIZATION, SIGNED_FIELDS},
};
use base64::Engine;
use uuid::Uuid;

// ── Test fixtures ─────────────────────────────────────────────────────────────

fn signed_envelope(event: ArrowEvent) -> (TransportEnvelope, String) {
    let (sk, pk) = generate_test_keypair();
    let did = did_key_from_public(&pk);
    let suffix = did.strip_prefix("did:key:").unwrap();
    let key_ref = format!("{did}#{suffix}");
    let mut env = TransportEnvelope {
        envelope_version: "1.0".to_string(),
        envelope_id: format!("env-conformance-{}", Uuid::new_v4()),
        sender_id: did.clone(),
        receiver_id: ReceiverId::One("did:arrow:consumer".to_string()),
        channel_binding: "http-rest-local".to_string(),
        sent_at: "2026-06-03T00:00:00Z".to_string(),
        nonce: format!("n-{}", Uuid::new_v4()),
        sequence: None,
        signature: None,
        signing: None,
        correlation_id: None,
        reply_to: None,
        metadata: None,
        payload: event,
    };
    let canonical = envelope_canonical_bytes(&env).unwrap();
    let sig_b64 =
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(sign_ed25519(&sk, &canonical));
    env.signing = Some(EnvelopeSigningMetadata {
        alg: "Ed25519".to_string(),
        key_ref,
        sig_b64,
        created: None,
        canonicalization: CANONICALIZATION.to_string(),
        signed_fields: SIGNED_FIELDS.to_string(),
    });
    (env, did)
}

// ── Origin event factories ────────────────────────────────────────────────────

fn eth_final_event() -> ArrowEvent {
    let mut e = ArrowEvent {
        arrow_code: "1:ETH:FN:PR:1:A".to_string(),
        // LFC-008: settled Class 1 under bounded discipline declares closure
        // due immediately (0 blocks remaining).
        closure_eta: Some(0),
        closure_eta_type: Some("block_count".to_string()),
        event_id: "evt-conf-eth-final".to_string(),
        lifecycle_id: "eth:0xconf01".to_string(),
        timestamp: "2026-06-03T00:00:00Z".to_string(),
        object_type: "transfer".to_string(),
        object_id: "0xconf01".to_string(),
        chain_id: Some("ETH".to_string()),
        proof_type: Some("tx_hash".to_string()),
        proof_ref: Some("0xconf01".to_string()),
        ..Default::default()
    };
    SourceValueTransfer::evm_native(
        "ETH",
        "0xconf01",
        "0x1111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222",
        "1000000000000000000",
        Some(12),
    )
    .attach_to_event(&mut e)
    .unwrap();
    e
}

fn bse_pending_event() -> ArrowEvent {
    let mut e = ArrowEvent {
        arrow_code: "1:BSE:PD:HL:0:B".to_string(),
        event_id: "evt-conf-bse-pending".to_string(),
        lifecycle_id: "base:0xconf02".to_string(),
        timestamp: "2026-06-03T00:00:00Z".to_string(),
        object_type: "transfer".to_string(),
        object_id: "0xconf02".to_string(),
        chain_id: Some("BASE".to_string()),
        proof_type: Some("tx_hash".to_string()),
        proof_ref: Some("0xconf02".to_string()),
        finality_basis: Some("optimistic_window".to_string()),
        // optimistic_window sets requires_finality_eta (FB-001): date the
        // challenge-window close (7 days after the 2026-06-03 event timestamp).
        finality_eta: Some(1781049600),
        finality_eta_type: Some("unix_ts".to_string()),
        ..Default::default()
    };
    SourceValueTransfer::evm_native(
        "BASE",
        "0xconf02",
        "0x1111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222",
        "500000000000000000",
        Some(0),
    )
    .attach_to_event(&mut e)
    .unwrap();
    e
}

fn arb_pending_event() -> ArrowEvent {
    let mut e = ArrowEvent {
        arrow_code: "1:ARB:PD:HL:0:B".to_string(),
        event_id: "evt-conf-arb-pending".to_string(),
        lifecycle_id: "arb:0xconf03".to_string(),
        timestamp: "2026-06-03T00:00:00Z".to_string(),
        object_type: "transfer".to_string(),
        object_id: "0xconf03".to_string(),
        chain_id: Some("ARB".to_string()),
        proof_type: Some("tx_hash".to_string()),
        proof_ref: Some("0xconf03".to_string()),
        finality_basis: Some("optimistic_window".to_string()),
        // optimistic_window sets requires_finality_eta (FB-001): date the
        // challenge-window close (7 days after the 2026-06-03 event timestamp).
        finality_eta: Some(1781049600),
        finality_eta_type: Some("unix_ts".to_string()),
        ..Default::default()
    };
    SourceValueTransfer::evm_native(
        "ARB",
        "0xconf03",
        "0x1111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222",
        "250000000000000000",
        Some(0),
    )
    .attach_to_event(&mut e)
    .unwrap();
    e
}

fn btc_final_event() -> ArrowEvent {
    let mut e = ArrowEvent {
        arrow_code: "1:BTC:FN:PR:1:A".to_string(),
        // LFC-008: settled Class 1 under bounded discipline declares closure
        // due immediately (0 blocks remaining).
        closure_eta: Some(0),
        closure_eta_type: Some("block_count".to_string()),
        event_id: "evt-conf-btc-final".to_string(),
        lifecycle_id: "btc:aabbccdd".to_string(),
        timestamp: "2026-06-03T00:00:00Z".to_string(),
        object_type: "transfer".to_string(),
        object_id: "aabbccdd".to_string(),
        chain_id: Some("BTC".to_string()),
        proof_type: Some("tx_hash".to_string()),
        proof_ref: Some("aabbccdd:0".to_string()),
        ..Default::default()
    };
    SourceValueTransfer::btc_native(
        "BTC",
        "aabbccdd",
        "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf",
        "1BpEi6DfDAUFd153wiGrvkiboLLaqFZu",
        "5000000",
        Some(6),
    )
    .attach_to_event(&mut e)
    .unwrap();
    e
}

fn sol_final_event() -> ArrowEvent {
    let mut e = ArrowEvent {
        arrow_code: "1:SOL:FN:PR:1:A".to_string(),
        // LFC-008: settled Class 1 under bounded discipline declares closure
        // due immediately (0 blocks remaining).
        closure_eta: Some(0),
        closure_eta_type: Some("block_count".to_string()),
        event_id: "evt-conf-sol-final".to_string(),
        lifecycle_id: "sol:5j7s8K9mN2pQ".to_string(),
        timestamp: "2026-06-03T00:00:00Z".to_string(),
        object_type: "transfer".to_string(),
        object_id: "5j7s8K9mN2pQ".to_string(),
        chain_id: Some("SOL".to_string()),
        proof_type: Some("tx_hash".to_string()),
        proof_ref: Some("5j7s8K9mN2pQ".to_string()),
        ..Default::default()
    };
    SourceValueTransfer::sol_native(
        "SOL",
        "5j7s8K9mN2pQ",
        "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
        "6FEgANEPqTsHxfyoTKtGBpBqSH6QqMG1V3HxVBFLbXPa",
        "1000000000",
        None,
    )
    .attach_to_event(&mut e)
    .unwrap();
    e
}

fn leg_judicial_event() -> ArrowEvent {
    leg_map(&LegalAttestation::final_judicial_order(
        "MX",
        "case-conf-2026",
        "did:arrow:oracle:mx-conf",
    ))
}

// ── Consumer helpers ──────────────────────────────────────────────────────────

/// ETH consumer wrapper: uses DryRun (no live settlement, dummy operator).
fn consume_eth(
    env: &TransportEnvelope,
    policy: &arrow_consume_eth::ProvenancePolicy,
    replay: &dyn arrow_consume_eth::ReplayStore,
) -> anyhow::Result<arrow_consume_eth::ConsumeOutcome> {
    use arrow_adapter::InMemoryDestinationExecutionStore;
    use arrow_consume_eth::{verify_and_consume_with_actions, OperatorConfig};
    let store = InMemoryDestinationExecutionStore::new();
    verify_and_consume_with_actions(
        env,
        policy,
        replay,
        &store,
        &OperatorConfig {
            rpc_url: String::new(),
            from: String::new(),
            ..Default::default()
        },
        ConsumerExecutionMode::DryRun,
    )
}

// ── Compatible pairs ──────────────────────────────────────────────────────────

/// ETH final → ETH consumer: DryRun receipt (FN:PR:1:A → ApprovedForSettlement).
#[test]
fn eth_origin_to_eth_consumer_dry_run() {
    let (env, did) = signed_envelope(eth_final_event());
    let policy = arrow_consume_eth::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = consume_eth(&env, &policy, &replay).expect("ETH→ETH must succeed");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::DryRun);
    assert!(
        receipt.action_intent.is_some(),
        "DryRun receipt must carry intent"
    );
    assert_eq!(receipt.consumer_adapter, "arrow-consume-eth");
}

/// ETH final → BASE consumer: RecordedOnly (EVM family, wei, approved).
#[test]
fn eth_origin_to_base_consumer_records() {
    let (env, did) = signed_envelope(eth_final_event());
    let policy = arrow_consume_base::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_base::verify_and_consume(&env, &policy, &replay)
        .expect("ETH→BASE must succeed (EVM family, wei)");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::RecordedOnly);
}

/// ETH final → ARB consumer: RecordedOnly (EVM family, wei, approved).
#[test]
fn eth_origin_to_arb_consumer_records() {
    let (env, did) = signed_envelope(eth_final_event());
    let policy = arrow_consume_arb::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_arb::verify_and_consume(&env, &policy, &replay)
        .expect("ETH→ARB must succeed (EVM family, wei)");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::RecordedOnly);
}

/// BSE pending → BASE consumer: NoAction (PD reduces to non-settlement).
#[test]
fn bse_origin_to_base_consumer_no_action() {
    let (env, did) = signed_envelope(bse_pending_event());
    let policy = arrow_consume_base::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_base::verify_and_consume(&env, &policy, &replay)
        .expect("BSE pending→BASE must not error");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(
        receipt.status,
        ConsumerReceiptStatus::NoAction,
        "L2 pending event must produce NoAction in its own consumer"
    );
}

/// ARB pending → ARB consumer: NoAction.
#[test]
fn arb_origin_to_arb_consumer_no_action() {
    let (env, did) = signed_envelope(arb_pending_event());
    let policy = arrow_consume_arb::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_arb::verify_and_consume(&env, &policy, &replay)
        .expect("ARB pending→ARB must not error");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::NoAction);
}

/// BTC final → BTC consumer: RecordedOnly.
#[test]
fn btc_origin_to_btc_consumer_records() {
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_btc::verify_and_consume(&env, &policy, &replay)
        .expect("BTC→BTC must succeed");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::RecordedOnly);
    assert!(
        receipt.action_intent.is_some(),
        "RecordedOnly receipt must carry intent"
    );
    assert_eq!(receipt.consumer_adapter, "arrow-consume-btc");
}

/// SOL final → SOL consumer: RecordedOnly.
#[test]
fn sol_origin_to_sol_consumer_records() {
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_sol::verify_and_consume(&env, &policy, &replay)
        .expect("SOL→SOL must succeed");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::RecordedOnly);
    assert!(receipt.action_intent.is_some());
    assert_eq!(receipt.consumer_adapter, "arrow-consume-sol");
}

/// LEG judicial order → LEG consumer: Quarantined (HoldForReview, never auto-acted).
#[test]
fn leg_origin_to_leg_consumer_quarantines() {
    let (env, did) = signed_envelope(leg_judicial_event());
    let policy = arrow_consume_leg::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let queue = InMemoryReviewQueue::new();
    let out = arrow_consume_leg::verify_and_review(
        &env,
        &policy,
        &replay,
        &queue,
        ConsumerExecutionMode::Execute,
    )
    .expect("LEG judicial → LEG consumer must succeed");
    assert_eq!(out.decision, LegReviewDecision::HoldForReview);
    assert_eq!(out.receipt.status, ConsumerReceiptStatus::Quarantined);
    assert!(out.queued, "judicial order must be queued for human review");
}

// ── Incompatible: EVM origin → non-EVM consumers ─────────────────────────────

/// ETH final → BTC consumer: typed wrong-chain error (wei vs sats).
#[test]
fn eth_origin_to_btc_consumer_wrong_chain() {
    let (env, did) = signed_envelope(eth_final_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_btc::verify_and_consume(&env, &policy, &replay)
        .expect_err("ETH→BTC must fail: wrong chain");
    let msg = err.to_string();
    assert!(
        msg.contains("origin BTC") || msg.contains("chain_id") || msg.contains("sats"),
        "must name the mismatch; got: {msg}"
    );
}

/// ETH final → SOL consumer: typed wrong-chain error (wei vs lamports).
#[test]
fn eth_origin_to_sol_consumer_wrong_chain() {
    let (env, did) = signed_envelope(eth_final_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_sol::verify_and_consume(&env, &policy, &replay)
        .expect_err("ETH→SOL must fail: wrong chain");
    let msg = err.to_string();
    assert!(
        msg.contains("origin SOL") || msg.contains("chain_id") || msg.contains("lamports"),
        "must name the mismatch; got: {msg}"
    );
}

/// ETH final → LEG consumer: wrong-origin error (LEG only handles LEG).
#[test]
fn eth_origin_to_leg_consumer_wrong_origin() {
    let (env, did) = signed_envelope(eth_final_event());
    let policy = arrow_consume_leg::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let queue = InMemoryReviewQueue::new();
    let err = arrow_consume_leg::verify_and_review(
        &env,
        &policy,
        &replay,
        &queue,
        ConsumerExecutionMode::RecordOnly,
    )
    .expect_err("ETH→LEG must fail: wrong origin");
    assert!(
        err.to_string().contains("LEG") && err.to_string().contains("ETH"),
        "must name both LEG and ETH; got: {err}"
    );
}

// ── Incompatible: BTC origin → non-BTC consumers ─────────────────────────────

/// BTC final → ETH consumer: typed wrong-origin / wrong-chain error.
#[test]
fn btc_origin_to_eth_consumer_wrong_chain() {
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_eth::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = consume_eth(&env, &policy, &replay).expect_err("BTC→ETH must fail: wrong chain");
    assert!(
        err.to_string().contains("wrong origin")
            || err.to_string().contains("sats")
            || err.to_string().contains("wei"),
        "must name the origin or unit mismatch; got: {err}"
    );
}

/// BTC final → SOL consumer: typed wrong-chain error (BTC origin vs SOL).
#[test]
fn btc_origin_to_sol_consumer_wrong_chain() {
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_sol::verify_and_consume(&env, &policy, &replay)
        .expect_err("BTC→SOL must fail: wrong chain");
    let msg = err.to_string();
    assert!(
        msg.contains("origin SOL") || msg.contains("sats") || msg.contains("lamports"),
        "must name the mismatch; got: {msg}"
    );
}

/// BTC final → LEG consumer: wrong-origin error.
#[test]
fn btc_origin_to_leg_consumer_wrong_origin() {
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_leg::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let queue = InMemoryReviewQueue::new();
    let err = arrow_consume_leg::verify_and_review(
        &env,
        &policy,
        &replay,
        &queue,
        ConsumerExecutionMode::RecordOnly,
    )
    .expect_err("BTC→LEG must fail: wrong origin");
    assert!(
        err.to_string().contains("LEG") && err.to_string().contains("BTC"),
        "must name both; got: {err}"
    );
}

// ── Incompatible: SOL origin → non-SOL consumers ─────────────────────────────

/// SOL final → ETH consumer: typed wrong-origin / wrong-chain error.
#[test]
fn sol_origin_to_eth_consumer_wrong_chain() {
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_eth::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = consume_eth(&env, &policy, &replay).expect_err("SOL→ETH must fail: wrong chain");
    assert!(
        err.to_string().contains("wrong origin")
            || err.to_string().contains("lamports")
            || err.to_string().contains("wei"),
        "must name the origin or unit mismatch; got: {err}"
    );
}

/// SOL final → BTC consumer: typed wrong-chain error (SOL origin vs BTC).
#[test]
fn sol_origin_to_btc_consumer_wrong_chain() {
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_btc::verify_and_consume(&env, &policy, &replay)
        .expect_err("SOL→BTC must fail: wrong chain");
    let msg = err.to_string();
    assert!(
        msg.contains("origin BTC") || msg.contains("lamports") || msg.contains("sats"),
        "must name the mismatch; got: {msg}"
    );
}

/// SOL final → LEG consumer: wrong-origin error.
#[test]
fn sol_origin_to_leg_consumer_wrong_origin() {
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_leg::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let queue = InMemoryReviewQueue::new();
    let err = arrow_consume_leg::verify_and_review(
        &env,
        &policy,
        &replay,
        &queue,
        ConsumerExecutionMode::RecordOnly,
    )
    .expect_err("SOL→LEG must fail: wrong origin");
    assert!(
        err.to_string().contains("LEG") && err.to_string().contains("SOL"),
        "must name both; got: {err}"
    );
}

// ── Incompatible: LEG origin → non-LEG consumers ─────────────────────────────

/// LEG judicial → ETH consumer: wrong-origin error.
#[test]
fn leg_origin_to_eth_consumer_no_typed_transfer() {
    let (env, did) = signed_envelope(leg_judicial_event());
    let policy = arrow_consume_eth::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    // JUD + Finality=1 → Action=PR → ApprovedForSettlement. The ETH consumer
    // rejects LEG at the canonical origin guard before any typed transfer can
    // drive an EVM intent. This is always an error, never NoAction.
    let err =
        consume_eth(&env, &policy, &replay).expect_err("LEG judicial→ETH must fail: wrong origin");
    assert!(
        err.to_string().contains("wrong origin") || err.to_string().contains("LEG"),
        "error must name the origin mismatch; got: {err}"
    );
}

/// LEG judicial → BTC consumer: wrong-origin error (BTC origin guard fires on LEG).
#[test]
fn leg_origin_to_btc_consumer_wrong_origin() {
    let (env, did) = signed_envelope(leg_judicial_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_btc::verify_and_consume(&env, &policy, &replay)
        .expect_err("LEG→BTC must fail: wrong origin");
    assert!(
        err.to_string().contains("origin BTC") || err.to_string().contains("LEG"),
        "must name the origin mismatch; got: {err}"
    );
}

/// LEG judicial → SOL consumer: wrong-origin error.
#[test]
fn leg_origin_to_sol_consumer_wrong_origin() {
    let (env, did) = signed_envelope(leg_judicial_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_sol::verify_and_consume(&env, &policy, &replay)
        .expect_err("LEG→SOL must fail: wrong origin");
    assert!(
        err.to_string().contains("origin SOL") || err.to_string().contains("LEG"),
        "must name the origin mismatch; got: {err}"
    );
}

// ── L2 pending events are safe across all consumers ───────────────────────────

/// BSE pending → ETH consumer: NoAction (pending, no settlement attempted).
/// Demonstrates L2 events are safe in any consumer — no wrong-chain panic.
#[test]
fn bse_pending_to_eth_consumer_no_action() {
    let (env, did) = signed_envelope(bse_pending_event());
    let policy = arrow_consume_eth::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = consume_eth(&env, &policy, &replay).expect("BSE pending→ETH must not panic");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(
        receipt.status,
        ConsumerReceiptStatus::NoAction,
        "L2 pending reduces to non-settlement in any consumer"
    );
}

/// BSE pending → BTC consumer: NoAction (pending, settlement guard never fires).
#[test]
fn bse_pending_to_btc_consumer_no_action() {
    let (env, did) = signed_envelope(bse_pending_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_btc::verify_and_consume(&env, &policy, &replay)
        .expect("BSE pending→BTC must not panic");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::NoAction);
}

/// ARB pending → SOL consumer: NoAction.
#[test]
fn arb_pending_to_sol_consumer_no_action() {
    let (env, did) = signed_envelope(arb_pending_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_sol::verify_and_consume(&env, &policy, &replay)
        .expect("ARB pending→SOL must not panic");
    let receipt = out.receipt.expect("must produce a receipt");
    assert_eq!(receipt.status, ConsumerReceiptStatus::NoAction);
}

// ── Receipt conformance ───────────────────────────────────────────────────────

/// Every receipt carries the required conformance fields.
#[test]
fn receipts_carry_conformance_fields() {
    use arrow_adapter::conformance::assert_receipt_conforms;

    // BTC RecordedOnly
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_btc::verify_and_consume(&env, &policy, &replay).unwrap();
    assert_receipt_conforms(
        out.receipt.as_ref().unwrap(),
        ConsumerExecutionMode::RecordOnly,
        ConsumerReceiptStatus::RecordedOnly,
    );

    // SOL RecordedOnly
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_sol::verify_and_consume(&env, &policy, &replay).unwrap();
    assert_receipt_conforms(
        out.receipt.as_ref().unwrap(),
        ConsumerExecutionMode::RecordOnly,
        ConsumerReceiptStatus::RecordedOnly,
    );
}

// ── Provenance / replay boundary ─────────────────────────────────────────────

/// Empty allow-list trusts no one — all origins rejected.
#[test]
fn empty_allow_list_trusts_no_one() {
    for (label, env) in [
        ("ETH", signed_envelope(eth_final_event()).0),
        ("BTC", signed_envelope(btc_final_event()).0),
        ("SOL", signed_envelope(sol_final_event()).0),
    ] {
        let policy = arrow_consume_btc::ProvenancePolicy::new(vec![], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let err = arrow_consume_btc::verify_and_consume(&env, &policy, &replay)
            .expect_err(&format!("{label}→BTC with empty allow-list must fail"));
        // The "no allowed senders" cause is wrapped by context; check the chain.
        let chain = format!("{err:#}");
        assert!(
            chain.contains("allow") || chain.contains("sender"),
            "{label}: got: {chain}"
        );
    }
}

/// Replay is rejected across adapters.
#[test]
fn replay_rejected_across_adapters() {
    // BTC
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    arrow_consume_btc::verify_and_consume(&env, &policy, &replay).unwrap();
    let err = arrow_consume_btc::verify_and_consume(&env, &policy, &replay)
        .expect_err("second consume must be rejected as replay");
    assert!(err.to_string().contains("replay"), "got: {err}");

    // SOL
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    arrow_consume_sol::verify_and_consume(&env, &policy, &replay).unwrap();
    let err = arrow_consume_sol::verify_and_consume(&env, &policy, &replay)
        .expect_err("second consume must be rejected as replay");
    assert!(err.to_string().contains("replay"), "got: {err}");
}

// ── Complete matrix: remaining 13 cells ──────────────────────────────────────
// Covers BSE/ARB→{SOL,ARB/ETH,BASE,BTC,LEG}, BTC/SOL→{BASE,ARB}, LEG→{BASE,ARB}.

/// BSE pending → SOL consumer: NoAction (pending, no settlement attempted).
#[test]
fn bse_pending_to_sol_consumer_no_action() {
    let (env, did) = signed_envelope(bse_pending_event());
    let policy = arrow_consume_sol::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_sol::verify_and_consume(&env, &policy, &replay).unwrap();
    assert_eq!(out.receipt.unwrap().status, ConsumerReceiptStatus::NoAction);
}

/// BSE pending → ARB consumer: NoAction.
#[test]
fn bse_pending_to_arb_consumer_no_action() {
    let (env, did) = signed_envelope(bse_pending_event());
    let policy = arrow_consume_arb::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_arb::verify_and_consume(&env, &policy, &replay).unwrap();
    assert_eq!(out.receipt.unwrap().status, ConsumerReceiptStatus::NoAction);
}

/// BSE pending → LEG consumer: wrong-origin error (LEG checks origin before disposition).
#[test]
fn bse_pending_to_leg_consumer_wrong_origin() {
    let (env, did) = signed_envelope(bse_pending_event());
    let policy = arrow_consume_leg::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let queue = InMemoryReviewQueue::new();
    let err = arrow_consume_leg::verify_and_review(
        &env,
        &policy,
        &replay,
        &queue,
        ConsumerExecutionMode::RecordOnly,
    )
    .expect_err("BSE→LEG must fail: non-LEG origin");
    assert!(
        err.to_string().contains("LEG") && err.to_string().contains("BSE"),
        "got: {err}"
    );
}

/// ARB pending → ETH consumer: NoAction.
#[test]
fn arb_pending_to_eth_consumer_no_action() {
    let (env, did) = signed_envelope(arb_pending_event());
    let policy = arrow_consume_eth::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = consume_eth(&env, &policy, &replay).unwrap();
    assert_eq!(out.receipt.unwrap().status, ConsumerReceiptStatus::NoAction);
}

/// ARB pending → BASE consumer: NoAction.
#[test]
fn arb_pending_to_base_consumer_no_action() {
    let (env, did) = signed_envelope(arb_pending_event());
    let policy = arrow_consume_base::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_base::verify_and_consume(&env, &policy, &replay).unwrap();
    assert_eq!(out.receipt.unwrap().status, ConsumerReceiptStatus::NoAction);
}

/// ARB pending → BTC consumer: NoAction.
#[test]
fn arb_pending_to_btc_consumer_no_action() {
    let (env, did) = signed_envelope(arb_pending_event());
    let policy = arrow_consume_btc::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let out = arrow_consume_btc::verify_and_consume(&env, &policy, &replay).unwrap();
    assert_eq!(out.receipt.unwrap().status, ConsumerReceiptStatus::NoAction);
}

/// ARB pending → LEG consumer: wrong-origin error.
#[test]
fn arb_pending_to_leg_consumer_wrong_origin() {
    let (env, did) = signed_envelope(arb_pending_event());
    let policy = arrow_consume_leg::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let queue = InMemoryReviewQueue::new();
    let err = arrow_consume_leg::verify_and_review(
        &env,
        &policy,
        &replay,
        &queue,
        ConsumerExecutionMode::RecordOnly,
    )
    .expect_err("ARB→LEG must fail: non-LEG origin");
    assert!(
        err.to_string().contains("LEG") && err.to_string().contains("ARB"),
        "got: {err}"
    );
}

/// BTC final → BASE consumer: wrong-origin / wrong-chain error.
#[test]
fn btc_origin_to_base_consumer_wrong_chain() {
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_base::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_base::verify_and_consume(&env, &policy, &replay)
        .expect_err("BTC→BASE must fail: sats vs wei");
    assert!(
        err.to_string().contains("wrong origin")
            || err.to_string().contains("sats")
            || err.to_string().contains("wei"),
        "must name origin or unit mismatch; got: {err}"
    );
}

/// BTC final → ARB consumer: wrong-origin / wrong-chain error.
#[test]
fn btc_origin_to_arb_consumer_wrong_chain() {
    let (env, did) = signed_envelope(btc_final_event());
    let policy = arrow_consume_arb::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_arb::verify_and_consume(&env, &policy, &replay)
        .expect_err("BTC→ARB must fail: sats vs wei");
    assert!(
        err.to_string().contains("wrong origin")
            || err.to_string().contains("sats")
            || err.to_string().contains("wei"),
        "must name origin or unit mismatch; got: {err}"
    );
}

/// SOL final → BASE consumer: wrong-origin / wrong-chain error.
#[test]
fn sol_origin_to_base_consumer_wrong_chain() {
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_base::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_base::verify_and_consume(&env, &policy, &replay)
        .expect_err("SOL→BASE must fail: lamports vs wei");
    assert!(
        err.to_string().contains("wrong origin")
            || err.to_string().contains("lamports")
            || err.to_string().contains("wei"),
        "must name origin or unit mismatch; got: {err}"
    );
}

/// SOL final → ARB consumer: wrong-origin / wrong-chain error.
#[test]
fn sol_origin_to_arb_consumer_wrong_chain() {
    let (env, did) = signed_envelope(sol_final_event());
    let policy = arrow_consume_arb::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_arb::verify_and_consume(&env, &policy, &replay)
        .expect_err("SOL→ARB must fail: lamports vs wei");
    assert!(
        err.to_string().contains("wrong origin")
            || err.to_string().contains("lamports")
            || err.to_string().contains("wei"),
        "must name origin or unit mismatch; got: {err}"
    );
}

/// LEG judicial → BASE consumer: wrong-origin error.
#[test]
fn leg_origin_to_base_consumer_no_typed_transfer() {
    let (env, did) = signed_envelope(leg_judicial_event());
    let policy = arrow_consume_base::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_base::verify_and_consume(&env, &policy, &replay)
        .expect_err("LEG judicial→BASE must fail: wrong origin");
    assert!(
        err.to_string().contains("wrong origin") || err.to_string().contains("LEG"),
        "must name the origin mismatch; got: {err}"
    );
}

/// LEG judicial → ARB consumer: same as BASE.
#[test]
fn leg_origin_to_arb_consumer_no_typed_transfer() {
    let (env, did) = signed_envelope(leg_judicial_event());
    let policy = arrow_consume_arb::ProvenancePolicy::new(vec![did], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let err = arrow_consume_arb::verify_and_consume(&env, &policy, &replay)
        .expect_err("LEG judicial→ARB must fail: wrong origin");
    assert!(
        err.to_string().contains("wrong origin") || err.to_string().contains("LEG"),
        "must name the origin mismatch; got: {err}"
    );
}
