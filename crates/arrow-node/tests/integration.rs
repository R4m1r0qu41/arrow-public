//! Arrow Node — HTTP REST integration tests.
//!
//! Required negative tests (patch spec):
//!  1. rejects missing Content-Type
//!  2. rejects wrong Content-Type
//!  3. accepts application/json; charset=utf-8
//!  4. rejects malformed JSON
//!  5. rejects empty nonce
//!  6. rejects malformed nonce
//!  7. rejects wrong channel_binding
//!  8. rejects receiver mismatch
//!  9. rejects empty receiver array
//! 10. rejects future sent_at
//! 11. rejects unknown sender_id
//! 12. rejects duplicate nonce from same sender
//! 13. rejects malformed ACK
//! 14. rejects ACK receiver mismatch

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use chrono::{Duration, Utc};
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, StatusCode};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;
use serde_json::{json, Value};
use uuid::Uuid;

use arrow_core::registry::loader::Registries;
use arrow_core::types::event::ArrowEvent;
use arrow_identity::{
    crypto::ed25519::{generate_test_keypair, sign_ed25519},
    resolver::did_key::did_key_from_public,
};
use arrow_node::{
    config::{IdentityConfig, TlsMode, TlsRuntimeConfig},
    handlers,
    state::AppState,
};
use arrow_transport::{
    canonical::envelope_canonical_bytes,
    envelope::{EnvelopeSigningMetadata, ReceiverId, TransportEnvelope},
    error::ReplayStoreError,
    replay::{InMemoryReplayStore, ReplayStore, SqliteReplayStore},
};

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn valid_event() -> ArrowEvent {
    serde_json::from_str(include_str!("fixtures/valid-event.json")).unwrap()
}
fn invalid_event() -> ArrowEvent {
    serde_json::from_str(include_str!("fixtures/semantically-invalid-event.json")).unwrap()
}

struct FailingReplayStore;

impl ReplayStore for FailingReplayStore {
    fn check_and_record(&self, _sender_id: &str, _nonce: &str) -> Result<bool, ReplayStoreError> {
        Err(ReplayStoreError::Io {
            source: "injected replay store failure".to_string(),
        })
    }

    fn sweep(&self) -> Result<u64, ReplayStoreError> {
        Ok(0)
    }
}

// ── State & server helpers ────────────────────────────────────────────────────

fn build_state(node_id: &str) -> Arc<AppState> {
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
    Arc::new(AppState {
        node_id: node_id.to_string(),
        registries: Arc::new(registries),
        replay,
        known_senders,
        binding,
        identity: IdentityConfig::default(),
        tls: TlsRuntimeConfig::local_disabled(),
    })
}

fn build_state_with_identity(
    node_id: &str,
    identity: IdentityConfig,
    extra_senders: &[String],
) -> Arc<AppState> {
    let replay: Arc<dyn ReplayStore> = Arc::new(InMemoryReplayStore::new(Duration::hours(24)));
    build_state_with_identity_and_replay(node_id, identity, extra_senders, replay)
}

fn build_state_with_identity_and_replay(
    node_id: &str,
    identity: IdentityConfig,
    extra_senders: &[String],
    replay: Arc<dyn ReplayStore>,
) -> Arc<AppState> {
    let registries = Registries::load_embedded().unwrap();
    let binding = registries
        .channel_bindings
        .get("http-rest-local")
        .unwrap()
        .clone();
    let mut known_senders = HashSet::from([
        "did:arrow:pilot:cli".to_string(),
        "did:arrow:pilot:node-a".to_string(),
        "did:arrow:pilot:node-b".to_string(),
    ]);
    known_senders.extend(extra_senders.iter().cloned());
    Arc::new(AppState {
        node_id: node_id.to_string(),
        registries: Arc::new(registries),
        replay,
        known_senders,
        binding,
        identity,
        tls: TlsRuntimeConfig::local_disabled(),
    })
}

fn build_https_state(node_id: &str) -> Arc<AppState> {
    let registries = Registries::load_embedded().unwrap();
    let binding = registries
        .channel_bindings
        .get("https-rest")
        .unwrap()
        .clone();
    let replay: Arc<dyn ReplayStore> = Arc::new(InMemoryReplayStore::new(Duration::hours(24)));
    let known_senders = HashSet::from([
        "did:arrow:pilot:cli".to_string(),
        "did:arrow:pilot:node-a".to_string(),
        "did:arrow:pilot:node-b".to_string(),
    ]);
    Arc::new(AppState {
        node_id: node_id.to_string(),
        registries: Arc::new(registries),
        replay,
        known_senders,
        binding,
        identity: IdentityConfig::default(),
        tls: TlsRuntimeConfig {
            mode: TlsMode::TerminatedUpstream,
            require_forwarded_proto_https: true,
            trusted_proxy: Some("127.0.0.1".parse().unwrap()),
        },
    })
}

fn spawn_node(state: Arc<AppState>) -> String {
    let app = Router::new()
        .route("/arrow/health", get(handlers::health))
        .route("/arrow/events", post(handlers::receive_event))
        .route("/arrow/ack", post(handlers::receive_ack))
        .with_state(state);
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

fn good_nonce() -> String {
    format!("n-{}", uuid::Uuid::new_v4().simple())
}

fn make_envelope(receiver: &str, event: ArrowEvent, nonce: &str) -> TransportEnvelope {
    TransportEnvelope {
        envelope_version: "1.0".to_string(),
        envelope_id: format!("env-{}", Uuid::new_v4()),
        sender_id: "did:arrow:pilot:node-a".to_string(),
        receiver_id: ReceiverId::One(receiver.to_string()),
        channel_binding: "http-rest-local".to_string(),
        sent_at: Utc::now().to_rfc3339(),
        nonce: nonce.to_string(),
        sequence: None,
        signature: None,
        signing: None,
        correlation_id: None,
        reply_to: None,
        metadata: None,
        payload: event,
    }
}

fn signature_required() -> IdentityConfig {
    IdentityConfig {
        require_signature: true,
        allowed_did_methods: vec!["did:key".to_string()],
        allowed_algorithms: vec!["Ed25519".to_string()],
    }
}

fn key_ref_for_public_key(sender_did: &str, public_key: &[u8; 32]) -> String {
    let mut prefixed = vec![0xed, 0x01];
    prefixed.extend_from_slice(public_key);
    let method_specific = format!("z{}", bs58::encode(&prefixed).into_string());
    format!("{sender_did}#{method_specific}")
}

fn sign_envelope_with_sender(
    receiver: &str,
    sender_id: String,
    key_ref_did_override: Option<String>,
) -> (TransportEnvelope, String) {
    let (secret_key, public_key) = generate_test_keypair();
    let signer_did = did_key_from_public(&public_key);
    let key_ref_did = key_ref_did_override.unwrap_or_else(|| signer_did.clone());
    let key_ref = key_ref_for_public_key(&key_ref_did, &public_key);
    let mut envelope = make_envelope(receiver, valid_event(), &good_nonce());
    envelope.sender_id = sender_id;

    let canonical = envelope_canonical_bytes(&envelope).unwrap();
    let signature = sign_ed25519(&secret_key, &canonical);
    let sig_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, signature);

    envelope.signing = Some(EnvelopeSigningMetadata {
        alg: "Ed25519".to_string(),
        key_ref,
        sig_b64,
        created: Some(Utc::now().to_rfc3339()),
        canonicalization: "arrow-canonical-json-v0".to_string(),
        signed_fields: "envelope-minus-signature-and-signing".to_string(),
    });

    (envelope, signer_did)
}

fn signed_envelope(receiver: &str) -> (TransportEnvelope, String) {
    let (secret_key, public_key) = generate_test_keypair();
    let sender_did = did_key_from_public(&public_key);
    let key_ref = key_ref_for_public_key(&sender_did, &public_key);
    let mut envelope = make_envelope(receiver, valid_event(), &good_nonce());
    envelope.sender_id = sender_did.clone();

    let canonical = envelope_canonical_bytes(&envelope).unwrap();
    let signature = sign_ed25519(&secret_key, &canonical);
    let sig_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, signature);

    envelope.signing = Some(EnvelopeSigningMetadata {
        alg: "Ed25519".to_string(),
        key_ref,
        sig_b64,
        created: Some(Utc::now().to_rfc3339()),
        canonicalization: "arrow-canonical-json-v0".to_string(),
        signed_fields: "envelope-minus-signature-and-signing".to_string(),
    });

    (envelope, sender_did)
}

// ── HTTP client helpers ───────────────────────────────────────────────────────

async fn post_raw(url: &str, content_type: Option<&str>, body: Vec<u8>) -> (StatusCode, Value) {
    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let mut builder = Request::builder().method(Method::POST).uri(url);
    if let Some(ct) = content_type {
        builder = builder.header("content-type", ct);
    }
    let req = builder.body(Full::new(Bytes::from(body))).unwrap();
    let resp = client.request(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn post_json(url: &str, body: &Value) -> (StatusCode, Value) {
    post_raw(
        url,
        Some("application/json"),
        serde_json::to_vec(body).unwrap(),
    )
    .await
}

async fn post_json_with_forwarded_proto(
    url: &str,
    body: &Value,
    proto: &str,
) -> (StatusCode, Value) {
    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let req = Request::builder()
        .method(Method::POST)
        .uri(url)
        .header("content-type", "application/json")
        .header("x-forwarded-proto", proto)
        .body(Full::new(Bytes::from(serde_json::to_vec(body).unwrap())))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn get_json(url: &str) -> (StatusCode, Value) {
    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let req = Request::builder()
        .method(Method::GET)
        .uri(url)
        .body(Full::new(Bytes::new()))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

// ── Baseline positive tests ───────────────────────────────────────────────────

#[tokio::test]
async fn health_returns_node_info() {
    let base = spawn_node(build_state("did:arrow:pilot:node-a"));
    let (status, body) = get_json(&format!("{base}/arrow/health")).await;
    assert_eq!(status, 200);
    assert_eq!(body["node_id"], "did:arrow:pilot:node-a");
    assert_eq!(body["channel_binding"], "http-rest-local");
    assert_eq!(body["transport_model"], "1.0");
}

#[tokio::test]
async fn valid_event_accepted() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let env = serde_json::to_value(make_envelope(
        "did:arrow:pilot:node-b",
        valid_event(),
        &good_nonce(),
    ))
    .unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &env).await;
    assert_eq!(status, 200);
    assert_eq!(body["result"], "accepted");
    assert_eq!(body["ack_type"], "validation");
    assert!(body["validation_result"]["passed"]
        .as_bool()
        .unwrap_or(false));
    assert!(body["ref_envelope_hash"]
        .as_str()
        .unwrap_or("")
        .starts_with("0x"));
}

#[tokio::test]
async fn semantic_failure_returns_200_rejected() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let env = serde_json::to_value(make_envelope(
        "did:arrow:pilot:node-b",
        invalid_event(),
        &good_nonce(),
    ))
    .unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &env).await;
    assert_eq!(status, 200);
    assert_eq!(body["result"], "rejected");
    assert!(!body["validation_result"]["passed"]
        .as_bool()
        .unwrap_or(true));
    assert!(!body["validation_result"]["violations"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn ack_with_ref_returns_200() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let ack = json!({
        "ack_version":"1.0","ack_id":"ack-ok","ack_type":"validation",
        "ref_envelope_id":"env-abc",
        "sender_id":"did:arrow:pilot:node-a",
        "receiver_id":"did:arrow:pilot:node-b",
        "channel_binding":"http-rest-local",
        "sent_at": Utc::now().to_rfc3339(),
        "result":"accepted"
    });
    let (status, _) = post_json(&format!("{base}/arrow/ack"), &ack).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn https_profile_accepts_forwarded_https_from_trusted_proxy() {
    let base = spawn_node(build_https_state("did:arrow:pilot:node-b"));
    let mut env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    env.channel_binding = "https-rest".to_string();
    let body = serde_json::to_value(&env).unwrap();

    let (status, body) =
        post_json_with_forwarded_proto(&format!("{base}/arrow/events"), &body, "https").await;

    assert_eq!(status, 200, "expected 200, got {status}: {body}");
    assert_eq!(body["channel_binding"], "https-rest");
    assert_eq!(body["result"], "accepted");
}

#[tokio::test]
async fn https_profile_rejects_missing_forwarded_https() {
    let base = spawn_node(build_https_state("did:arrow:pilot:node-b"));
    let mut env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    env.channel_binding = "https-rest".to_string();
    let body = serde_json::to_value(&env).unwrap();

    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;

    assert_eq!(status, 409, "expected 409, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-CHANNEL-MISMATCH");
}

// ── Identity / signature tests ────────────────────────────────────────────────

#[tokio::test]
async fn valid_signature_with_key_ref_did_equal_to_sender_passes() {
    let (env, sender_did) = signed_envelope("did:arrow:pilot:node-b");
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[sender_did],
    ));
    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;
    assert_eq!(status, 200, "expected 200, got {status}: {body}");
    assert_eq!(body["result"], "accepted");
}

#[tokio::test]
async fn valid_signature_by_different_did_but_allowlisted_sender_fails() {
    let (env, signer_did) = sign_envelope_with_sender(
        "did:arrow:pilot:node-b",
        "did:arrow:pilot:node-a".to_string(),
        None,
    );
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[signer_did],
    ));
    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;
    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn allowed_did_key_does_not_accept_did_keyevil_prefix() {
    let sender_id = "did:keyevil:zBadActor".to_string();
    let (env, _) = sign_envelope_with_sender(
        "did:arrow:pilot:node-b",
        sender_id.clone(),
        Some(sender_id.clone()),
    );
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[sender_id],
    ));
    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;
    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn wrong_signed_fields_is_rejected() {
    let (mut env, sender_did) = signed_envelope("did:arrow:pilot:node-b");
    env.signing.as_mut().unwrap().signed_fields = "envelope-minus-signing".to_string();
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[sender_did],
    ));
    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;
    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn wrong_canonicalization_is_rejected() {
    let (mut env, sender_did) = signed_envelope("did:arrow:pilot:node-b");
    env.signing.as_mut().unwrap().canonicalization = "not-arrow-canonical-json".to_string();
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[sender_did],
    ));
    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;
    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn missing_signature_on_required_node_is_rejected() {
    let env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[],
    ));

    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;

    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-MISSING");
}

#[tokio::test]
async fn tampered_payload_after_signing_is_rejected() {
    let (mut env, sender_did) = signed_envelope("did:arrow:pilot:node-b");
    env.payload.event_id = "ARR-TAMPERED-AFTER-SIGNING".to_string();
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[sender_did],
    ));

    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;

    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn swapped_key_ref_after_signing_is_rejected() {
    let (mut env, _) = signed_envelope("did:arrow:pilot:node-b");
    let (_, public_key_b) = generate_test_keypair();
    let did_b = did_key_from_public(&public_key_b);
    env.sender_id = did_b.clone();
    env.signing.as_mut().unwrap().key_ref = key_ref_for_public_key(&did_b, &public_key_b);
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[did_b],
    ));

    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;

    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn spoofed_algorithm_is_rejected() {
    let (mut env, sender_did) = signed_envelope("did:arrow:pilot:node-b");
    env.signing.as_mut().unwrap().alg = "ES256".to_string();
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[sender_did],
    ));

    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;

    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn blocked_did_method_is_rejected() {
    let (env, sender_did) = signed_envelope("did:arrow:pilot:node-b");
    let identity = IdentityConfig {
        require_signature: true,
        allowed_did_methods: vec!["web".to_string()],
        allowed_algorithms: vec!["Ed25519".to_string()],
    };
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        identity,
        &[sender_did],
    ));

    let body = serde_json::to_value(&env).unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &body).await;

    assert_eq!(status, 401, "expected 401, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
}

#[tokio::test]
async fn signed_replay_is_rejected_but_fresh_nonce_is_accepted() {
    let (env, sender_did) = signed_envelope("did:arrow:pilot:node-b");
    let (fresh, fresh_sender_did) = signed_envelope("did:arrow:pilot:node-b");
    let base = spawn_node(build_state_with_identity(
        "did:arrow:pilot:node-b",
        signature_required(),
        &[sender_did, fresh_sender_did],
    ));

    let body = serde_json::to_value(&env).unwrap();
    let (status, first) = post_json(&format!("{base}/arrow/events"), &body).await;
    assert_eq!(status, 200, "expected 200, got {status}: {first}");
    assert_eq!(first["result"], "accepted");

    let (status, replay) = post_json(&format!("{base}/arrow/events"), &body).await;
    assert_eq!(status, 409, "expected 409, got {status}: {replay}");
    assert_eq!(replay["error_code"], "TPT-REPLAY-DETECTED");

    let fresh_body = serde_json::to_value(&fresh).unwrap();
    let (status, fresh_response) = post_json(&format!("{base}/arrow/events"), &fresh_body).await;
    assert_eq!(status, 200, "expected 200, got {status}: {fresh_response}");
    assert_eq!(fresh_response["result"], "accepted");
}

#[tokio::test]
async fn sqlite_replay_store_rejects_nonce_after_node_restart() {
    let path = std::env::temp_dir().join(format!("arrow-node-replay-{}.db", Uuid::new_v4()));
    let env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    let body = serde_json::to_value(&env).unwrap();

    let replay: Arc<dyn ReplayStore> =
        Arc::new(SqliteReplayStore::open(&path, Duration::hours(24)).unwrap());
    let first_base = spawn_node(build_state_with_identity_and_replay(
        "did:arrow:pilot:node-b",
        IdentityConfig::default(),
        &[],
        replay,
    ));
    let (status, first) = post_json(&format!("{first_base}/arrow/events"), &body).await;
    assert_eq!(status, 200, "expected 200, got {status}: {first}");
    assert_eq!(first["result"], "accepted");

    let replay_after_restart: Arc<dyn ReplayStore> =
        Arc::new(SqliteReplayStore::open(&path, Duration::hours(24)).unwrap());
    let second_base = spawn_node(build_state_with_identity_and_replay(
        "did:arrow:pilot:node-b",
        IdentityConfig::default(),
        &[],
        replay_after_restart,
    ));
    let (status, replay) = post_json(&format!("{second_base}/arrow/events"), &body).await;
    assert_eq!(status, 409, "expected 409, got {status}: {replay}");
    assert_eq!(replay["error_code"], "TPT-REPLAY-DETECTED");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
}

#[tokio::test]
async fn replay_store_error_returns_503() {
    let replay: Arc<dyn ReplayStore> = Arc::new(FailingReplayStore);
    let base = spawn_node(build_state_with_identity_and_replay(
        "did:arrow:pilot:node-b",
        IdentityConfig::default(),
        &[],
        replay,
    ));
    let env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    let body = serde_json::to_value(&env).unwrap();

    let (status, response) = post_json(&format!("{base}/arrow/events"), &body).await;

    assert_eq!(status, 503, "expected 503, got {status}: {response}");
    assert_eq!(response["error_code"], "TPT-REPLAY-STORE-UNAVAILABLE");
}

// ── Negative tests (patch spec §5) ───────────────────────────────────────────

// 1. Rejects missing Content-Type
#[tokio::test]
async fn rejects_missing_content_type() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let env = serde_json::to_vec(&make_envelope(
        "did:arrow:pilot:node-b",
        valid_event(),
        &good_nonce(),
    ))
    .unwrap();
    let (status, body) = post_raw(&format!("{base}/arrow/events"), None, env).await;
    assert_eq!(status, 415, "expected 415, got {status}: {body}");
}

// 2. Rejects wrong Content-Type
#[tokio::test]
async fn rejects_wrong_content_type() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let env = serde_json::to_vec(&make_envelope(
        "did:arrow:pilot:node-b",
        valid_event(),
        &good_nonce(),
    ))
    .unwrap();
    let (status, body) = post_raw(&format!("{base}/arrow/events"), Some("text/plain"), env).await;
    assert_eq!(status, 415, "expected 415, got {status}: {body}");
}

// 3. Accepts application/json; charset=utf-8
#[tokio::test]
async fn accepts_json_with_charset() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let env = serde_json::to_vec(&make_envelope(
        "did:arrow:pilot:node-b",
        valid_event(),
        &good_nonce(),
    ))
    .unwrap();
    let (status, body) = post_raw(
        &format!("{base}/arrow/events"),
        Some("application/json; charset=utf-8"),
        env,
    )
    .await;
    assert_eq!(status, 200, "expected 200, got {status}: {body}");
    assert_eq!(body["result"], "accepted");
}

// 4. Rejects malformed JSON
#[tokio::test]
async fn rejects_malformed_json() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let (status, body) = post_raw(
        &format!("{base}/arrow/events"),
        Some("application/json"),
        b"{not valid json".to_vec(),
    )
    .await;
    assert_eq!(status, 400, "expected 400, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-MALFORMED");
}

// 5. Rejects empty nonce
#[tokio::test]
async fn rejects_empty_nonce() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let mut env = make_envelope("did:arrow:pilot:node-b", valid_event(), "");
    env.nonce = "".to_string();
    let (status, body) = post_json(
        &format!("{base}/arrow/events"),
        &serde_json::to_value(&env).unwrap(),
    )
    .await;
    assert_eq!(status, 400, "expected 400, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-MALFORMED");
}

// 6. Rejects malformed nonce
#[tokio::test]
async fn rejects_malformed_nonce() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let mut env = make_envelope("did:arrow:pilot:node-b", valid_event(), "badnonce");
    env.nonce = "not-the-right-format".to_string();
    let (status, body) = post_json(
        &format!("{base}/arrow/events"),
        &serde_json::to_value(&env).unwrap(),
    )
    .await;
    assert_eq!(status, 400, "expected 400, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-MALFORMED");
}

// 7. Rejects wrong channel_binding (note: unregistered → TPT-004 fires first, 400)
#[tokio::test]
async fn rejects_wrong_channel_binding() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let mut env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    env.channel_binding = "grpc".to_string();
    let (status, body) = post_json(
        &format!("{base}/arrow/events"),
        &serde_json::to_value(&env).unwrap(),
    )
    .await;
    // Unregistered binding hits TPT-004 → 400
    assert_eq!(status, 400, "expected 400, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-MALFORMED");
}

// 8. Rejects receiver mismatch
#[tokio::test]
async fn rejects_receiver_mismatch() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    // Send envelope addressed to a different node
    let env = serde_json::to_value(make_envelope(
        "did:arrow:pilot:node-c",
        valid_event(),
        &good_nonce(),
    ))
    .unwrap();
    let (status, body) = post_json(&format!("{base}/arrow/events"), &env).await;
    assert_eq!(status, 403, "expected 403, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-RECEIVER-MISMATCH");
}

// 9. Rejects empty receiver array
#[tokio::test]
async fn rejects_empty_receiver_array() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let mut env_val = serde_json::to_value(make_envelope(
        "did:arrow:pilot:node-b",
        valid_event(),
        &good_nonce(),
    ))
    .unwrap();
    env_val["receiver_id"] = json!([]);
    let (status, body) = post_json(&format!("{base}/arrow/events"), &env_val).await;
    assert_eq!(status, 400, "expected 400, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-MALFORMED");
}

// 10. Rejects future sent_at beyond tolerance
#[tokio::test]
async fn rejects_future_sent_at() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let mut env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    env.sent_at = (Utc::now() + Duration::minutes(10)).to_rfc3339();
    let (status, body) = post_json(
        &format!("{base}/arrow/events"),
        &serde_json::to_value(&env).unwrap(),
    )
    .await;
    assert_eq!(status, 400, "expected 400, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-STALE");
}

// 11. Rejects unknown sender_id
#[tokio::test]
async fn rejects_unknown_sender() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let mut env = make_envelope("did:arrow:pilot:node-b", valid_event(), &good_nonce());
    env.sender_id = "did:arrow:intruder:x".to_string();
    let (status, body) = post_json(
        &format!("{base}/arrow/events"),
        &serde_json::to_value(&env).unwrap(),
    )
    .await;
    assert_eq!(status, 403, "expected 403, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-UNKNOWN-SENDER");
}

// 12. Rejects duplicate nonce from same sender
#[tokio::test]
async fn rejects_duplicate_nonce() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let fixed_nonce = "n-00000000000000000000000000000001";
    let env = serde_json::to_value(make_envelope(
        "did:arrow:pilot:node-b",
        valid_event(),
        fixed_nonce,
    ))
    .unwrap();
    let (s1, _) = post_json(&format!("{base}/arrow/events"), &env).await;
    assert_eq!(s1, 200);
    let (s2, body) = post_json(&format!("{base}/arrow/events"), &env).await;
    assert_eq!(s2, 409, "expected 409, got {s2}: {body}");
    assert_eq!(body["error_code"], "TPT-REPLAY-DETECTED");
}

// 13. Rejects malformed ACK (no content-type)
#[tokio::test]
async fn rejects_malformed_ack_no_content_type() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let (status, _) = post_raw(&format!("{base}/arrow/ack"), None, b"{}".to_vec()).await;
    assert_eq!(status, 415);
}

// 14. Rejects ACK receiver mismatch
#[tokio::test]
async fn rejects_ack_receiver_mismatch() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let ack = json!({
        "ack_version":"1.0","ack_id":"ack-x","ack_type":"validation",
        "ref_envelope_id":"env-abc",
        "sender_id":"did:arrow:pilot:node-a",
        "receiver_id":"did:arrow:pilot:node-DIFFERENT",   // mismatch
        "channel_binding":"http-rest-local",
        "sent_at": Utc::now().to_rfc3339(),
        "result":"accepted"
    });
    let (status, body) = post_json(&format!("{base}/arrow/ack"), &ack).await;
    assert_eq!(status, 403, "expected 403, got {status}: {body}");
    assert_eq!(body["error_code"], "TPT-RECEIVER-MISMATCH");
}

// ── ACK validation tests ──────────────────────────────────────────────────────

#[tokio::test]
async fn ack_missing_ref_returns_400() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let ack = json!({
        "ack_version":"1.0","ack_id":"ack-001","ack_type":"validation",
        "sender_id":"did:arrow:pilot:node-a","receiver_id":"did:arrow:pilot:node-b",
        "channel_binding":"http-rest-local","sent_at":Utc::now().to_rfc3339(),"result":"accepted"
    });
    let (status, body) = post_json(&format!("{base}/arrow/ack"), &ack).await;
    assert_eq!(status, 400);
    assert_eq!(body["error_code"], "TPT-MALFORMED");
}

#[tokio::test]
async fn ack_future_sent_at_rejected() {
    let base = spawn_node(build_state("did:arrow:pilot:node-b"));
    let ack = json!({
        "ack_version":"1.0","ack_id":"ack-fut","ack_type":"validation",
        "ref_envelope_id":"env-x",
        "sender_id":"did:arrow:pilot:node-a","receiver_id":"did:arrow:pilot:node-b",
        "channel_binding":"http-rest-local",
        "sent_at": (Utc::now() + Duration::minutes(10)).to_rfc3339(),
        "result":"accepted"
    });
    let (status, body) = post_json(&format!("{base}/arrow/ack"), &ack).await;
    assert_eq!(status, 400);
    assert_eq!(body["error_code"], "TPT-MALFORMED");
}
