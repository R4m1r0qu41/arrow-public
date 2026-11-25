//! Signed two-node loopback tests for the MVP demo path.
//!
//! The replay behavior here is intentionally per receiving node: the same
//! signed envelope can be accepted once by node-a and once by node-b because
//! each node owns an independent replay store.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
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
use uuid::Uuid;

use arrow_core::registry::loader::Registries;
use arrow_core::types::event::ArrowEvent;
use arrow_node::{
    config::{IdentityConfig, TlsRuntimeConfig},
    handlers,
    state::AppState,
};
use arrow_transport::{
    envelope::{ReceiverId, TransportEnvelope},
    replay::{InMemoryReplayStore, ReplayStore},
};

const CLI_DID: &str = "did:key:z6MktULudTtAsAhRegYPiZ6631RV3viv12qd4GQF8z1xB22S";
const NODE_A_DID: &str = "did:key:z6MkqGC3nWZhYieEVTVDKW5v588CiGfsDSmRVG9ZwwWTvLSK";
const NODE_B_DID: &str = "did:key:z6Mkg49NtQR2LyYRDCQFK4w1VVHqhypZSSRo7HsyuN7SV7v5";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn dev_key(name: &str) -> PathBuf {
    fixture("dev-keys").join(name)
}

fn valid_event() -> ArrowEvent {
    serde_json::from_str(include_str!("fixtures/valid-event.json")).unwrap()
}

fn fresh_nonce() -> String {
    format!("n-{}", uuid::Uuid::new_v4().simple())
}

fn build_signed_state(node_id: &str) -> Arc<AppState> {
    let registries = Registries::load_embedded().unwrap();
    let binding = registries
        .channel_bindings
        .get("http-rest-local")
        .unwrap()
        .clone();
    let replay: Arc<dyn ReplayStore> = Arc::new(InMemoryReplayStore::new(Duration::hours(24)));
    let known_senders = HashSet::from([
        CLI_DID.to_string(),
        NODE_A_DID.to_string(),
        NODE_B_DID.to_string(),
    ]);
    let identity = IdentityConfig {
        require_signature: true,
        allowed_did_methods: vec!["did:key".to_string()],
        allowed_algorithms: vec!["Ed25519".to_string()],
    };

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

fn spawn_node(node_id: &str) -> String {
    let app = Router::new()
        .route("/arrow/health", get(handlers::health))
        .route("/arrow/events", post(handlers::receive_event))
        .route("/arrow/ack", post(handlers::receive_ack))
        .with_state(build_signed_state(node_id));
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

fn unsigned_envelope(receiver_id: ReceiverId) -> TransportEnvelope {
    TransportEnvelope {
        envelope_version: "1.0".to_string(),
        envelope_id: format!("env-{}", Uuid::new_v4()),
        sender_id: CLI_DID.to_string(),
        receiver_id,
        channel_binding: "http-rest-local".to_string(),
        sent_at: Utc::now().to_rfc3339(),
        nonce: fresh_nonce(),
        sequence: None,
        signature: None,
        signing: None,
        correlation_id: None,
        reply_to: None,
        metadata: None,
        payload: valid_event(),
    }
}

fn signed_envelope(receiver_id: ReceiverId, key_file: &Path) -> TransportEnvelope {
    let mut envelope = unsigned_envelope(receiver_id);
    arrow_cli::commands::signing::sign_envelope_in_place(&mut envelope, key_file).unwrap();
    envelope
}

async fn post_envelope(
    base_url: &str,
    envelope: &TransportEnvelope,
) -> (StatusCode, serde_json::Value) {
    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("{base_url}/arrow/events"))
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(envelope).unwrap(),
        )))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap();
    (status, body)
}

#[test]
fn signed_emit_from_cli_persona_reaches_both_nodes() {
    let node_a_url = spawn_node(NODE_A_DID);
    let node_b_url = spawn_node(NODE_B_DID);
    let key = dev_key("cli-key.json");
    let event = fixture("valid-event.json");

    let to_b = arrow_cli::commands::emit::run(
        &node_b_url,
        &event,
        NODE_A_DID,
        NODE_B_DID,
        Some(key.as_path()),
    )
    .unwrap();
    assert!(to_b, "signed CLI persona should be accepted by node-b");

    let to_a = arrow_cli::commands::emit::run(
        &node_a_url,
        &event,
        NODE_A_DID,
        NODE_A_DID,
        Some(key.as_path()),
    )
    .unwrap();
    assert!(to_a, "signed CLI persona should be accepted by node-a");
}

#[test]
fn signed_fanout_replay_is_per_receiving_node() {
    let node_a_url = spawn_node(NODE_A_DID);
    let node_b_url = spawn_node(NODE_B_DID);
    let envelope = signed_envelope(
        ReceiverId::Many(vec![NODE_A_DID.to_string(), NODE_B_DID.to_string()]),
        &dev_key("cli-key.json"),
    );

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (status_a, body_a) = post_envelope(&node_a_url, &envelope).await;
        assert_eq!(status_a, StatusCode::OK, "node-a response: {body_a}");
        assert_eq!(body_a["result"], "accepted");

        let (replay_status_a, replay_body_a) = post_envelope(&node_a_url, &envelope).await;
        assert_eq!(
            replay_status_a,
            StatusCode::CONFLICT,
            "node-a replay response: {replay_body_a}"
        );
        assert_eq!(replay_body_a["error_code"], "TPT-REPLAY-DETECTED");

        let (status_b, body_b) = post_envelope(&node_b_url, &envelope).await;
        assert_eq!(status_b, StatusCode::OK, "node-b response: {body_b}");
        assert_eq!(body_b["result"], "accepted");
    });
}

#[test]
fn signed_sender_did_swap_is_rejected() {
    let node_b_url = spawn_node(NODE_B_DID);
    let mut envelope = signed_envelope(
        ReceiverId::One(NODE_B_DID.to_string()),
        &dev_key("cli-key.json"),
    );
    envelope.sender_id = NODE_A_DID.to_string();

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (status, body) = post_envelope(&node_b_url, &envelope).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "response: {body}");
        assert_eq!(body["error_code"], "TPT-SIGNATURE-INVALID");
    });
}
