//! EVM-family Consumption Adapter library for the Arrow Protocol.
//!
//! The consumption side of the pipeline (RFC-003 §4) for any EVM destination: it
//! takes an Arrow event log, **re-validates** it, reduces it to a disposition via
//! [`arrow_reduce`], and — when the disposition permits settlement — performs the
//! host-system action on a destination EVM chain via [`settle`].
//!
//! One implementation serves every EVM destination; the per-chain differences
//! (adapter id, destination chain id, expected source unit) live in
//! [`EvmConsumerConfig`]. A concrete consumer (e.g. `arrow-consume-eth`) is a
//! thin shell that supplies the config.
//!
//! # ⚠️ Prototype trust boundaries
//!
//! This adapter is a working **prototype**, not a production settlement system.
//! Settlement is lifecycle-bound (built from the typed, normalized
//! [`SourceValueTransfer`] on the event, never free-form metadata); the input log
//! is re-validated; provenance/signature is re-verified; and both envelope replay
//! and destination-execution replay are rejected. Residuals: identity mapping is
//! 1:1, and settlement uses an unlocked account (see [`settle`]).

pub mod config;
pub mod settle;

use anyhow::{bail, Context, Result};
use arrow_adapter::{
    destination_execution_id, ActionIntent, ConsumerExecutionMode, ConsumerReceipt,
    ConsumerReceiptStatus, DestinationExecutionStore, InMemoryDestinationExecutionStore,
    SourceValueTransfer,
};
use arrow_core::registry::loader::Registries;
use arrow_core::types::event::ArrowEvent;
use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_reduce::{reduce, Reduction};

pub use arrow_adapter::provenance::{
    verify_envelope_provenance as verify_envelope, ProvenancePolicy, ProvenanceVerification,
};
pub use arrow_transport::{envelope::TransportEnvelope, replay::ReplayStore};
pub use config::EvmConsumerConfig;
pub use settle::SettleParams;

/// Operator-supplied settlement configuration. This is the *only* settlement
/// input that comes from the operator rather than the event: where to settle
/// (chain-B RPC) and which account funds it. The recipient and amount are
/// event-derived (see [`settlement_intent`]).
#[derive(Debug, Clone)]
pub struct OperatorConfig {
    /// Destination chain JSON-RPC endpoint.
    pub rpc_url: String,
    /// `0x`-prefixed funding account on chain B.
    pub from: String,
    /// Hex-encoded raw 32-byte secp256k1 private key for EIP-155 raw signing
    /// (§4.4). `None` falls through to the unlocked path.
    pub raw_key_hex: Option<String>,
    /// If `true`, use `eth_sendTransaction` (Anvil unlocked-account path).
    /// Defaults to `true` for backwards compatibility with existing Anvil tests.
    pub unlocked: bool,
}

impl Default for OperatorConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::new(),
            from: String::new(),
            raw_key_hex: None,
            unlocked: true, // default: Anvil unlocked path
        }
    }
}

impl OperatorConfig {
    /// Create an OperatorConfig for raw EIP-155 signing on a public testnet.
    pub fn raw_signing(
        rpc_url: impl Into<String>,
        from: impl Into<String>,
        key_hex: impl Into<String>,
    ) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            from: from.into(),
            raw_key_hex: Some(key_hex.into()),
            unlocked: false,
        }
    }
}

/// Outcome of consuming an event log.
#[derive(Debug, Clone)]
pub struct ConsumeOutcome {
    /// The reduction that drove the decision.
    pub reduction: Reduction,
    /// The event-derived settlement target, present whenever the disposition
    /// permits settlement (even under `dry_run`).
    pub intent: Option<ActionIntent>,
    /// Destination-execution idempotency key, present whenever an action intent
    /// was derived from the normalized lifecycle.
    pub destination_execution_id: Option<String>,
    /// Whether a settlement transaction was submitted to chain B.
    pub settled: bool,
    /// The chain-B settlement transaction hash, if one was submitted.
    pub chain_b_tx: Option<String>,
    /// Typed receipt produced by the secure envelope entry point.
    pub receipt: Option<ConsumerReceipt>,
}

/// Re-validate every event against the Arrow validation pipeline.
///
/// The consumer MUST NOT trust the input log: an event that never passed a node
/// could otherwise drive settlement. Returns an error naming the first event that
/// fails, so an invalid log can never reach reduction or settlement.
pub fn validate_log(events: &[ArrowEvent]) -> Result<()> {
    let registries = Registries::load_embedded().context("loading embedded registries")?;
    let pipeline = ValidationPipeline::new(&registries);
    for ev in events {
        let result = pipeline.validate_event(ev);
        if !result.passed {
            let errors: Vec<String> = result
                .violations
                .iter()
                .filter(|v| v.is_error())
                .map(|v| format!("{}: {}", v.rule_id, v.message))
                .collect();
            bail!(
                "event `{}` failed Arrow validation: {}",
                ev.event_id,
                errors.join("; ")
            );
        }
    }
    Ok(())
}

/// Derive the destination action intent from the normalized Arrow lifecycle.
///
/// The origin supplies a typed [`SourceValueTransfer`] fact. This EVM consumer
/// becomes destination-specific here by translating that normalized fact into an
/// EVM value-transfer [`ActionIntent`] on `config.destination_chain_id`.
pub fn settlement_intent(event: &ArrowEvent, config: &EvmConsumerConfig) -> Result<ActionIntent> {
    let parsed = arrow_core::parser::message::parse(&event.arrow_code)
        .with_context(|| format!("malformed arrow_code `{}`", event.arrow_code))?;
    let expected_chain = evm_chain_for_origin(&parsed.origin).with_context(|| {
        format!(
            "wrong origin: EVM consumer requires an EVM-family Arrow code origin, got `{}`",
            parsed.origin
        )
    })?;
    match event.chain_id.as_deref() {
        Some(chain_id) if chain_id == expected_chain => {}
        other => bail!(
            "wrong origin: EVM consumer requires chain_id={} for Arrow origin {}, got {:?}",
            expected_chain,
            parsed.origin,
            other
        ),
    }

    let transfer = SourceValueTransfer::from_event(event).context(
        "event has no typed source value transfer; refusing metadata-derived settlement",
    )?;
    if transfer.source_system != "evm" {
        bail!(
            "wrong origin: EVM consumer requires source_system=evm, got `{}`",
            transfer.source_system
        );
    }
    if transfer.source_chain_id != expected_chain {
        bail!(
            "wrong origin: EVM consumer requires source_chain_id={}, got `{}`",
            expected_chain,
            transfer.source_chain_id
        );
    }
    if transfer.amount.unit != config.expected_source_unit {
        bail!(
            "unsupported source transfer unit `{}` for EVM settlement (expected `{}`)",
            transfer.amount.unit,
            config.expected_source_unit
        );
    }
    let intent = ActionIntent::evm_value_transfer(
        format!(
            "intent:evm-transfer:{}:{}:{}:{}",
            transfer.source_chain_id, transfer.source_tx, transfer.to, transfer.amount.value
        ),
        config.destination_chain_id.clone(),
        transfer.to,
        transfer.amount.value,
        event.event_id.clone(),
        event.proof_ref.clone(),
    );
    intent
        .validate_destination("evm", &config.expected_source_unit)
        .map_err(|e| anyhow::anyhow!("intent destination mismatch: {e}"))?;
    Ok(intent)
}

fn evm_chain_for_origin(origin: &str) -> Option<&'static str> {
    match origin {
        "ETH" => Some("ETH"),
        "BSE" => Some("BASE"),
        "ARB" => Some("ARB"),
        _ => None,
    }
}

/// Re-validate, reduce, and — when the disposition permits settlement and
/// `dry_run` is false — submit the event-bound settlement transfer to chain B.
pub fn consume_and_settle(
    events: &[ArrowEvent],
    operator: &OperatorConfig,
    config: &EvmConsumerConfig,
    dry_run: bool,
) -> Result<ConsumeOutcome> {
    // Trust boundary: never act on an unvalidated log.
    validate_log(events)?;

    let reduction = reduce(events)?;

    if !reduction.disposition.permits_settlement() {
        return Ok(ConsumeOutcome {
            reduction,
            intent: None,
            destination_execution_id: None,
            settled: false,
            chain_b_tx: None,
            receipt: None,
        });
    }

    // The settled event is the one the reduction acted on (the latest in the log;
    // a dominating BL/RV would not have permitted settlement).
    let settle_event = events
        .last()
        .expect("non-empty log: reduce() errors on an empty log");
    let intent = settlement_intent(settle_event, config)?;

    if dry_run {
        return Ok(ConsumeOutcome {
            reduction,
            intent: Some(intent),
            destination_execution_id: None,
            settled: false,
            chain_b_tx: None,
            receipt: None,
        });
    }

    let params = SettleParams {
        rpc_url: operator.rpc_url.clone(),
        from: operator.from.clone(),
        to: intent.destination.address.clone(),
        value_wei: intent.amount.value.clone(),
        // Default to unlocked (Anvil) path; callers can override via OperatorConfig
        // extensions when raw signing is needed (§4.4).
        raw_key_hex: operator.raw_key_hex.clone(),
        unlocked: operator.unlocked,
    };
    let tx = settle::send_settlement(&params)?;
    Ok(ConsumeOutcome {
        reduction,
        intent: Some(intent),
        destination_execution_id: None,
        settled: true,
        chain_b_tx: Some(tx),
        receipt: None,
    })
}

/// Verify an incoming **signed envelope**'s provenance, reject replay, then
/// consume its payload. This is the secure entry point.
pub fn verify_and_consume(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    operator: &OperatorConfig,
    config: &EvmConsumerConfig,
    dry_run: bool,
) -> Result<ConsumeOutcome> {
    let execution_store = InMemoryDestinationExecutionStore::new();
    let mode = if dry_run {
        ConsumerExecutionMode::DryRun
    } else {
        ConsumerExecutionMode::Execute
    };
    verify_and_consume_with_actions(
        envelope,
        policy,
        replay,
        &execution_store,
        operator,
        config,
        mode,
    )
}

pub fn verify_and_consume_with_actions(
    envelope: &TransportEnvelope,
    policy: &ProvenancePolicy,
    replay: &dyn ReplayStore,
    execution_store: &dyn DestinationExecutionStore,
    operator: &OperatorConfig,
    config: &EvmConsumerConfig,
    mode: ConsumerExecutionMode,
) -> Result<ConsumeOutcome> {
    let provenance = verify_envelope(envelope, policy).context("provenance verification failed")?;
    if mode.records_idempotency()
        && replay.check_and_record(&envelope.sender_id, &envelope.nonce)?
    {
        bail!(
            "replay detected: nonce `{}` from `{}` was already consumed",
            envelope.nonce,
            envelope.sender_id
        );
    }

    validate_log(std::slice::from_ref(&envelope.payload))?;
    let reduction = reduce(std::slice::from_ref(&envelope.payload))?;
    if !reduction.disposition.permits_settlement() {
        let receipt = ConsumerReceipt::new(
            config.adapter_id.clone(),
            mode,
            ConsumerReceiptStatus::NoAction,
            envelope.envelope_id.clone(),
            envelope.payload.event_id.clone(),
            &reduction,
            provenance,
        );
        return Ok(ConsumeOutcome {
            reduction,
            intent: None,
            destination_execution_id: None,
            settled: false,
            chain_b_tx: None,
            receipt: Some(receipt),
        });
    }

    let intent = settlement_intent(&envelope.payload, config)?;
    let execution_id = destination_execution_id(&intent);
    if mode.records_idempotency() && execution_store.check_and_record_execution(&execution_id)? {
        bail!("destination execution replay detected: `{execution_id}` was already recorded");
    }

    let status = match mode {
        ConsumerExecutionMode::DryRun => ConsumerReceiptStatus::DryRun,
        ConsumerExecutionMode::RecordOnly => ConsumerReceiptStatus::RecordedOnly,
        ConsumerExecutionMode::Execute => ConsumerReceiptStatus::Executed,
    };

    if !mode.executes_destination() {
        let receipt = ConsumerReceipt::new(
            config.adapter_id.clone(),
            mode,
            status,
            envelope.envelope_id.clone(),
            envelope.payload.event_id.clone(),
            &reduction,
            provenance,
        )
        .with_action(intent.clone(), Some(execution_id.clone()), None);
        return Ok(ConsumeOutcome {
            reduction,
            intent: Some(intent),
            destination_execution_id: Some(execution_id),
            settled: false,
            chain_b_tx: None,
            receipt: Some(receipt),
        });
    }

    let params = SettleParams {
        rpc_url: operator.rpc_url.clone(),
        from: operator.from.clone(),
        to: intent.destination.address.clone(),
        value_wei: intent.amount.value.clone(),
        // Default to unlocked (Anvil) path; callers can override via OperatorConfig
        // extensions when raw signing is needed (§4.4).
        raw_key_hex: operator.raw_key_hex.clone(),
        unlocked: operator.unlocked,
    };
    let tx = settle::send_settlement(&params)?;
    let receipt = ConsumerReceipt::new(
        config.adapter_id.clone(),
        mode,
        status,
        envelope.envelope_id.clone(),
        envelope.payload.event_id.clone(),
        &reduction,
        provenance,
    )
    .with_action(intent.clone(), Some(execution_id.clone()), Some(tx.clone()));
    Ok(ConsumeOutcome {
        reduction,
        intent: Some(intent),
        destination_execution_id: Some(execution_id),
        settled: true,
        chain_b_tx: Some(tx),
        receipt: Some(receipt),
    })
}

/// Parse an event file that holds either a single `ArrowEvent` object or a JSON
/// array of them.
pub fn parse_event_log(raw: &str) -> Result<Vec<ArrowEvent>> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let events = if value.is_array() {
        serde_json::from_value(value)?
    } else {
        vec![serde_json::from_value(value)?]
    };
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_adapter::{ConsumerExecutionMode, ConsumerReceiptStatus};
    use arrow_identity::{
        crypto::ed25519::{generate_test_keypair, sign_ed25519},
        resolver::did_key::did_key_from_public,
    };
    use arrow_transport::{
        canonical::envelope_canonical_bytes,
        envelope::{EnvelopeSigningMetadata, ReceiverId},
        replay::InMemoryReplayStore,
        signature::{CANONICALIZATION, SIGNED_FIELDS},
    };
    use base64::Engine;
    use bytes::Bytes;
    use http_body_util::{BodyExt, Full};
    use hyper::body::Incoming;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use hyper_util::rt::TokioIo;
    use std::convert::Infallible;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct CapturedRpcRequest {
        body: serde_json::Value,
    }

    fn config() -> EvmConsumerConfig {
        EvmConsumerConfig::eth()
    }

    fn operator(rpc_url: impl Into<String>) -> OperatorConfig {
        OperatorConfig {
            rpc_url: rpc_url.into(),
            from: "0x1111111111111111111111111111111111111111".to_string(),
            raw_key_hex: None,
            unlocked: true,
        }
    }

    fn approved_eth_event() -> ArrowEvent {
        let mut event = ArrowEvent {
            arrow_code: "1:ETH:FN:PR:1:A".to_string(),
            closure_eta: Some(0),
            closure_eta_type: Some("block_count".to_string()),
            event_id: "evt-eth-approved".to_string(),
            lifecycle_id: "life-eth-approved".to_string(),
            timestamp: "2026-06-07T00:00:00Z".to_string(),
            object_type: "transfer".to_string(),
            object_id: "0xabc".to_string(),
            chain_id: Some("ETH".to_string()),
            proof_type: Some("tx_hash".to_string()),
            proof_ref: Some("0xabc".to_string()),
            metadata: Some(serde_json::json!({
                "to": "0x9999999999999999999999999999999999999999",
                "value_wei": "1"
            })),
            ..Default::default()
        };
        SourceValueTransfer::evm_native(
            "ETH",
            "0xabc",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            Some(12),
        )
        .attach_to_event(&mut event)
        .expect("source transfer should serialize");
        event
    }

    fn signed_envelope(
        event: ArrowEvent,
        envelope_id: &str,
        nonce: &str,
    ) -> (TransportEnvelope, String) {
        let (sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let suffix = did.strip_prefix("did:key:").unwrap();
        let key_ref = format!("{did}#{suffix}");
        let mut envelope = TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: envelope_id.to_string(),
            sender_id: did.clone(),
            receiver_id: ReceiverId::One("did:arrow:consumer".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: "2026-06-07T00:00:00Z".to_string(),
            nonce: nonce.to_string(),
            sequence: None,
            signature: None,
            signing: None,
            correlation_id: None,
            reply_to: None,
            metadata: None,
            payload: event,
        };
        let canonical = envelope_canonical_bytes(&envelope).expect("canonicalize envelope");
        let sig_b64 =
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(sign_ed25519(&sk, &canonical));
        envelope.signing = Some(EnvelopeSigningMetadata {
            alg: "Ed25519".to_string(),
            key_ref,
            sig_b64,
            created: None,
            canonicalization: CANONICALIZATION.to_string(),
            signed_fields: SIGNED_FIELDS.to_string(),
        });
        (envelope, did)
    }

    fn serve_once(
        status: u16,
        body: impl Into<String>,
    ) -> (String, Arc<Mutex<Option<CapturedRpcRequest>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock RPC server");
        let addr = listener.local_addr().expect("mock RPC server address");
        let captured = Arc::new(Mutex::new(None));
        let captured_for_server = Arc::clone(&captured);
        let body = body.into();

        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("mock RPC runtime")
                .block_on(async move {
                    listener
                        .set_nonblocking(true)
                        .expect("set mock listener nonblocking");
                    let listener =
                        tokio::net::TcpListener::from_std(listener).expect("tokio listener");
                    let (stream, _) = listener.accept().await.expect("accept mock request");
                    let captured = Arc::clone(&captured_for_server);
                    let response_body = body.clone();

                    let service = service_fn(move |req: Request<Incoming>| {
                        let captured = Arc::clone(&captured);
                        let response_body = response_body.clone();

                        async move {
                            let body_bytes = req
                                .into_body()
                                .collect()
                                .await
                                .expect("read mock request body")
                                .to_bytes();
                            let body = serde_json::from_slice(&body_bytes)
                                .expect("mock request body is JSON");
                            *captured.lock().expect("capture request") =
                                Some(CapturedRpcRequest { body });

                            Ok::<_, Infallible>(
                                Response::builder()
                                    .status(status)
                                    .body(Full::new(Bytes::from(response_body)))
                                    .expect("mock response"),
                            )
                        }
                    });

                    http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await
                        .expect("mock RPC server");
                });
        });

        (format!("http://{addr}"), captured)
    }

    #[test]
    fn settlement_intent_refuses_metadata_only_event() {
        let mut event = approved_eth_event();
        event.extensions = None;

        let err = settlement_intent(&event, &config())
            .expect_err("metadata-only events must not drive settlement");

        assert!(
            err.to_string().contains("typed source value transfer"),
            "got: {err:#}"
        );
    }

    #[test]
    fn settlement_intent_refuses_wrong_source_system() {
        let mut event = approved_eth_event();
        event.extensions = None;
        SourceValueTransfer::btc_native(
            "BTC",
            "aabbccdd",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf",
            "1BpEi6DfDAUFd153wiGrvkiboLLaqFZu",
            "5000000",
            Some(6),
        )
        .attach_to_event(&mut event)
        .expect("source transfer should serialize");

        let err = settlement_intent(&event, &config())
            .expect_err("non-EVM source system must fail closed");

        assert!(
            err.to_string().contains("source_system=evm"),
            "got: {err:#}"
        );
    }

    #[test]
    fn settlement_intent_refuses_wrong_source_chain_id() {
        let mut event = approved_eth_event();
        event.extensions = None;
        SourceValueTransfer::evm_native(
            "BASE",
            "0xabc",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            Some(12),
        )
        .attach_to_event(&mut event)
        .expect("source transfer should serialize");

        let err = settlement_intent(&event, &config())
            .expect_err("wrong EVM-family source chain must fail closed");

        assert!(
            err.to_string().contains("source_chain_id=ETH"),
            "got: {err:#}"
        );
    }

    #[test]
    fn settlement_intent_refuses_wrong_amount_unit() {
        let mut event = approved_eth_event();
        event.extensions = None;
        let mut transfer = SourceValueTransfer::evm_native(
            "ETH",
            "0xabc",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            Some(12),
        );
        transfer.amount.unit = "gwei".to_string();
        transfer
            .attach_to_event(&mut event)
            .expect("source transfer should serialize");

        let err =
            settlement_intent(&event, &config()).expect_err("wrong source unit must fail closed");

        assert!(
            err.to_string()
                .contains("unsupported source transfer unit `gwei`"),
            "got: {err:#}"
        );
    }

    #[test]
    fn settlement_intent_refuses_unsupported_transfer_version() {
        let mut event = approved_eth_event();
        event.extensions = None;
        let mut transfer = SourceValueTransfer::evm_native(
            "ETH",
            "0xabc",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            Some(12),
        );
        transfer.transfer_version = "2.0".to_string();
        transfer
            .attach_to_event(&mut event)
            .expect("source transfer should serialize");

        let err = settlement_intent(&event, &config())
            .expect_err("unsupported transfer version must fail closed");

        assert!(
            format!("{err:#}").contains("source value transfer version `2.0` is unsupported"),
            "got: {err:#}"
        );
    }

    #[test]
    fn dry_run_derives_intent_without_settling_or_recording_replay() {
        let event = approved_eth_event();
        let (envelope, did) = signed_envelope(event, "env-dry-run", "nonce-dry-run");
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let execution_store = InMemoryDestinationExecutionStore::new();

        let first = verify_and_consume_with_actions(
            &envelope,
            &policy,
            &replay,
            &execution_store,
            &operator("http://127.0.0.1:1"),
            &config(),
            ConsumerExecutionMode::DryRun,
        )
        .expect("dry run should derive intent without RPC");
        assert!(!first.settled);
        assert!(first.chain_b_tx.is_none());
        assert!(first.intent.is_some());
        assert!(first.destination_execution_id.is_some());
        assert_eq!(
            first.receipt.as_ref().map(|r| r.status),
            Some(ConsumerReceiptStatus::DryRun)
        );

        verify_and_consume_with_actions(
            &envelope,
            &policy,
            &replay,
            &execution_store,
            &operator("http://127.0.0.1:1"),
            &config(),
            ConsumerExecutionMode::DryRun,
        )
        .expect("dry run should not record replay state");
    }

    #[test]
    fn record_only_records_replay_and_destination_idempotency_without_settling() {
        let event = approved_eth_event();
        let (envelope, did) = signed_envelope(event.clone(), "env-record-1", "nonce-record-1");
        let (reminted, reminted_did) = signed_envelope(event, "env-record-2", "nonce-record-2");
        let policy = ProvenancePolicy::new(vec![did, reminted_did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let execution_store = InMemoryDestinationExecutionStore::new();

        let first = verify_and_consume_with_actions(
            &envelope,
            &policy,
            &replay,
            &execution_store,
            &operator("http://127.0.0.1:1"),
            &config(),
            ConsumerExecutionMode::RecordOnly,
        )
        .expect("record-only should derive and record intent without RPC");
        assert!(!first.settled);
        assert!(first.chain_b_tx.is_none());
        assert!(first.destination_execution_id.is_some());
        assert_eq!(
            first.receipt.as_ref().map(|r| r.status),
            Some(ConsumerReceiptStatus::RecordedOnly)
        );

        let replay_err = verify_and_consume_with_actions(
            &envelope,
            &policy,
            &replay,
            &execution_store,
            &operator("http://127.0.0.1:1"),
            &config(),
            ConsumerExecutionMode::RecordOnly,
        )
        .expect_err("same signed envelope must be rejected as replay");
        assert!(
            replay_err.to_string().contains("replay detected"),
            "got: {replay_err:#}"
        );

        let destination_replay_err = verify_and_consume_with_actions(
            &reminted,
            &policy,
            &replay,
            &execution_store,
            &operator("http://127.0.0.1:1"),
            &config(),
            ConsumerExecutionMode::RecordOnly,
        )
        .expect_err("same logical action in a fresh envelope must be rejected");
        assert!(
            destination_replay_err
                .to_string()
                .contains("destination execution replay detected"),
            "got: {destination_replay_err:#}"
        );
    }

    #[test]
    fn execute_mode_sends_event_bound_settlement_and_records_receipt() {
        let (url, captured) = serve_once(200, r#"{"jsonrpc":"2.0","id":1,"result":"0xsettled"}"#);
        let (envelope, did) = signed_envelope(approved_eth_event(), "env-execute", "nonce-execute");
        let policy = ProvenancePolicy::new(vec![did], "did:arrow:consumer");
        let replay = InMemoryReplayStore::with_24h_retention();
        let execution_store = InMemoryDestinationExecutionStore::new();

        let out = verify_and_consume_with_actions(
            &envelope,
            &policy,
            &replay,
            &execution_store,
            &operator(url),
            &config(),
            ConsumerExecutionMode::Execute,
        )
        .expect("execute mode should submit settlement");

        assert!(out.settled);
        assert_eq!(out.chain_b_tx.as_deref(), Some("0xsettled"));
        assert!(out.destination_execution_id.is_some());
        let receipt = out.receipt.expect("execute should produce receipt");
        assert_eq!(receipt.status, ConsumerReceiptStatus::Executed);
        assert_eq!(receipt.destination_tx.as_deref(), Some("0xsettled"));

        let req = captured
            .lock()
            .expect("capture lock")
            .clone()
            .expect("captured RPC request");
        assert_eq!(req.body["method"], "eth_sendTransaction");
        assert_eq!(
            req.body["params"][0]["to"],
            "0x2222222222222222222222222222222222222222"
        );
        assert_eq!(req.body["params"][0]["value"], "0xde0b6b3a7640000");
    }
}
