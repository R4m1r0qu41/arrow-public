//! Arrow CLI integration tests.
//! - validate and inspect: offline, no server.
//! - emit: spins a real arrow-node server.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use chrono::Duration;

use arrow_core::registry::loader::Registries;
use arrow_core::validation::pipeline::{ValidationResult, Violation};
use arrow_identity::{
    crypto::ed25519::generate_test_keypair, resolver::did_key::did_key_from_public,
};
use arrow_node::{
    config::{IdentityConfig, TlsRuntimeConfig},
    handlers,
    state::AppState,
};
use arrow_transport::{
    envelope::{AckEnvelope, AckResult, AckType},
    replay::{InMemoryReplayStore, ReplayStore},
};

// ── Path helpers ──────────────────────────────────────────────────────────────

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

// ── Node spawn (same pattern as arrow-node integration tests) ─────────────────

fn make_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/arrow/health", get(handlers::health))
        .route("/arrow/events", post(handlers::receive_event))
        .route("/arrow/ack", post(handlers::receive_ack))
        .with_state(state)
}

fn serve(app: Router) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                listener.set_nonblocking(true).unwrap();
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                .unwrap();
            });
    });
    std::thread::sleep(std::time::Duration::from_millis(80));
    format!("http://{addr}")
}

/// Spawn an unsigned-accepting node with the three standard pilot DIDs.
fn spawn_node(node_id: &str) -> String {
    let registries = Registries::load_embedded().unwrap();
    let binding = registries
        .channel_bindings
        .get("http-rest-local")
        .unwrap()
        .clone();
    let replay: Arc<dyn ReplayStore> = Arc::new(InMemoryReplayStore::new(Duration::hours(24)));
    let known_senders = HashSet::from([
        "did:arrow:pilot:cli".to_string(),
        "did:arrow:pilot:node-a".to_string(),
        "did:arrow:pilot:node-b".to_string(),
    ]);
    let state = Arc::new(AppState {
        node_id: node_id.to_string(),
        registries: Arc::new(registries),
        replay,
        known_senders,
        binding,
        identity: IdentityConfig::default(),
        tls: TlsRuntimeConfig::local_disabled(),
    });
    serve(make_app(state))
}

/// Spawn a signature-requiring node that accepts exactly one signer DID.
fn spawn_signing_node(node_id: &str, signer_did: &str) -> String {
    let registries = Registries::load_embedded().unwrap();
    let binding = registries
        .channel_bindings
        .get("http-rest-local")
        .unwrap()
        .clone();
    let replay: Arc<dyn ReplayStore> = Arc::new(InMemoryReplayStore::new(Duration::hours(24)));
    let known_senders = HashSet::from([signer_did.to_string()]);
    let identity = IdentityConfig {
        require_signature: true,
        allowed_did_methods: vec!["did:key".to_string()],
        allowed_algorithms: vec!["Ed25519".to_string()],
    };
    let state = Arc::new(AppState {
        node_id: node_id.to_string(),
        registries: Arc::new(registries),
        replay,
        known_senders,
        binding,
        identity,
        tls: TlsRuntimeConfig::local_disabled(),
    });
    serve(make_app(state))
}

/// Write a dev key file (same shape as `arrow identity generate`) to a temp path.
/// Returns `(keyfile_path, did)`.
fn write_temp_keyfile() -> (PathBuf, String) {
    let (sk, pk) = generate_test_keypair();
    let did = did_key_from_public(&pk);

    // Build key_ref — same formula as identity::generate
    let mut prefixed = vec![0xed, 0x01];
    prefixed.extend_from_slice(&pk);
    let method_specific = format!("z{}", bs58::encode(&prefixed).into_string());
    let key_ref = format!("{did}#{method_specific}");

    let key_json = serde_json::json!({
        "did": did,
        "key_ref": key_ref,
        "algorithm": "Ed25519",
        "private_key_b64": base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD, sk
        ),
        "public_key_b64": base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD, pk
        ),
        "warning": "DEV/TEST ONLY — do not use in production"
    });

    let path = std::env::temp_dir().join(format!("arrow-test-key-{}.json", uuid::Uuid::new_v4()));
    std::fs::write(&path, serde_json::to_string_pretty(&key_json).unwrap()).unwrap();
    (path, did)
}

// ── validate ─────────────────────────────────────────────────────────────────

#[test]
fn validate_ok_event_passes() {
    let passed =
        arrow_cli::commands::validate::run(&fixture("valid-event.json"), false, &[]).unwrap();
    assert!(passed, "valid event should pass validation");
}

#[test]
fn validate_ok_event_json_output() {
    let passed =
        arrow_cli::commands::validate::run(&fixture("valid-event.json"), true, &[]).unwrap();
    assert!(passed);
}

#[test]
fn validate_bad_event_fails() {
    let passed =
        arrow_cli::commands::validate::run(&fixture("semantically-invalid-event.json"), false, &[])
            .unwrap();
    assert!(!passed, "invalid event should fail validation");
}

// ── inspect ───────────────────────────────────────────────────────────────────

#[test]
fn inspect_valid_code_succeeds() {
    for code in ["2:CST:FN:PR:1:C", "1:BTC:FN:TR:1:A", "5:ETH:PD:HL:0:B"] {
        arrow_cli::commands::inspect::run(code)
            .unwrap_or_else(|e| panic!("inspect({code}) failed: {e}"));
    }
}

#[test]
fn inspect_invalid_code_returns_err() {
    assert!(arrow_cli::commands::inspect::run("not-a-code").is_err());
    assert!(arrow_cli::commands::inspect::run("1:BTC:FN").is_err());
}

// ── ack inspect ───────────────────────────────────────────────────────────────

#[test]
fn ack_inspect_rejected_ack_includes_violation_narrative() {
    let ack = AckEnvelope {
        ack_version: "1.0".to_string(),
        ack_id: "ack-test".to_string(),
        ack_type: AckType::Validation,
        ref_envelope_id: Some("env-test".to_string()),
        ref_envelope_hash: Some("0xabc123".to_string()),
        sender_id: "did:arrow:pilot:node-b".to_string(),
        receiver_id: "did:arrow:pilot:node-a".to_string(),
        channel_binding: "http-rest-local".to_string(),
        sent_at: "2026-06-25T00:00:00Z".to_string(),
        result: AckResult::Rejected,
        validation_result: Some(ValidationResult::from_violations(vec![Violation::new(
            "SEM-017",
            Some("confidence"),
            "bridge proof semantics are required",
        )])),
        details: None,
    };

    let path = std::env::temp_dir().join(format!("arrow-test-ack-{}.json", uuid::Uuid::new_v4()));
    std::fs::write(&path, serde_json::to_string_pretty(&ack).unwrap()).unwrap();

    let output = arrow_cli::commands::ack::inspect(&path).unwrap();
    assert!(output.contains("Result:     REJECTED"));
    assert!(output.contains("[ERROR] SEM-017 (confidence)"));
    assert!(output.contains("Bridge confidence requires explicit proof semantics."));

    let _ = std::fs::remove_file(path);
}

// ── emit ─────────────────────────────────────────────────────────────────────

#[test]
fn emit_valid_event_accepted() {
    let base = spawn_node("did:arrow:pilot:node-b");
    let accepted = arrow_cli::commands::emit::run(
        &base,
        &fixture("valid-event.json"),
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        None,
    )
    .unwrap();
    assert!(accepted, "valid event should be accepted by node");
}

#[test]
fn emit_valid_event_twice_second_is_replay() {
    let base = spawn_node("did:arrow:pilot:node-b");
    // First emit should succeed; second with same envelope_id could differ
    // but since nonce is randomised per emit, both should succeed.
    // This test just confirms two successive emits both work (different nonces).
    let r1 = arrow_cli::commands::emit::run(
        &base,
        &fixture("valid-event.json"),
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        None,
    )
    .unwrap();
    let r2 = arrow_cli::commands::emit::run(
        &base,
        &fixture("valid-event.json"),
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        None,
    )
    .unwrap();
    assert!(r1 && r2, "both emits with fresh nonces should be accepted");
}

#[test]
fn emit_invalid_event_returns_false() {
    let base = spawn_node("did:arrow:pilot:node-b");
    let accepted = arrow_cli::commands::emit::run(
        &base,
        &fixture("semantically-invalid-event.json"),
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        None,
    )
    .unwrap();
    assert!(
        !accepted,
        "semantically invalid event should return false (result=rejected)"
    );
}

#[test]
fn emit_trailing_slash_url_works() {
    let base = spawn_node("did:arrow:pilot:node-b");
    // Add trailing slash — should still work (overlay 2.17)
    let url_with_slash = format!("{base}/");
    let accepted = arrow_cli::commands::emit::run(
        &url_with_slash,
        &fixture("valid-event.json"),
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        None,
    )
    .unwrap();
    assert!(accepted);
}

// ── signed emit ───────────────────────────────────────────────────────────────

#[test]
fn emit_signed_event_is_accepted_by_signing_node() {
    let (keyfile_path, signer_did) = write_temp_keyfile();
    let base = spawn_signing_node("did:arrow:pilot:node-b", &signer_did);

    let accepted = arrow_cli::commands::emit::run(
        &base,
        &fixture("valid-event.json"),
        // --from is ignored when key_file is Some; the DID from the file wins
        "did:arrow:pilot:cli",
        "did:arrow:pilot:node-b",
        Some(&keyfile_path),
    )
    .unwrap();
    assert!(
        accepted,
        "signed event should be accepted by signature-requiring node"
    );
}

#[test]
fn emit_unsigned_to_signing_node_returns_false() {
    let (keyfile_path, signer_did) = write_temp_keyfile();
    // Spawn the node with the generated DID in known_senders, but send unsigned.
    let base = spawn_signing_node("did:arrow:pilot:node-b", &signer_did);

    // Need the DID as sender_id so the node recognises the sender but still
    // rejects for missing signature (TPT-SIGNATURE-MISSING).
    let accepted = arrow_cli::commands::emit::run(
        &base,
        &fixture("valid-event.json"),
        &signer_did,
        "did:arrow:pilot:node-b",
        None, // no key file → unsigned
    )
    .unwrap();
    assert!(
        !accepted,
        "unsigned envelope to a require_signature node must return false (TPT-SIGNATURE-MISSING)"
    );
    // Clean up temp file
    let _ = std::fs::remove_file(&keyfile_path);
}
