//! Two-node round-trip integration tests.
//!
//! Each test spins up two independent arrow-node HTTP servers (node-a and
//! node-b) and exercises the full Arrow Protocol HTTP REST flow:
//!
//!   1. node-a sends an event → node-b   POST /arrow/events
//!   2. node-b validates and returns an AckEnvelope (result = accepted/rejected)
//!   3. The AckEnvelope is forwarded → node-a   POST /arrow/ack
//!   4. node-a records the ACK and returns HTTP 200
//!
//! This covers the minimal "loopback" path mandated by the Arrow Protocol
//! Transport Model Part 4 (acknowledgment routing).

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
use serde_json::Value;
use uuid::Uuid;

use arrow_core::registry::loader::Registries;
use arrow_core::types::event::ArrowEvent;
use arrow_node::{
    config::{IdentityConfig, TlsRuntimeConfig},
    handlers,
    state::AppState,
};
use arrow_transport::{
    envelope::{AckEnvelope, AckResult, ReceiverId, TransportEnvelope},
    replay::{InMemoryReplayStore, ReplayStore},
};

// ── Infrastructure helpers ────────────────────────────────────────────────────

/// Build a node state that recognises all three pilot DIDs as known senders.
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

/// Spawn an axum HTTP server on a random port. Returns the base URL.
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

fn valid_event() -> ArrowEvent {
    serde_json::from_str(include_str!("fixtures/valid-event.json")).unwrap()
}

fn invalid_event() -> ArrowEvent {
    serde_json::from_str(include_str!("fixtures/semantically-invalid-event.json")).unwrap()
}

fn fresh_nonce() -> String {
    format!("n-{}", uuid::Uuid::new_v4().simple())
}

/// Build a transport envelope from node-a to `receiver_node_id`.
fn make_envelope(sender_id: &str, receiver_node_id: &str, event: ArrowEvent) -> TransportEnvelope {
    TransportEnvelope {
        envelope_version: "1.0".to_string(),
        envelope_id: format!("env-{}", Uuid::new_v4()),
        sender_id: sender_id.to_string(),
        receiver_id: ReceiverId::One(receiver_node_id.to_string()),
        channel_binding: "http-rest-local".to_string(),
        sent_at: Utc::now().to_rfc3339(),
        nonce: fresh_nonce(),
        sequence: None,
        signature: None,
        signing: None,
        correlation_id: None,
        reply_to: None,
        metadata: None,
        payload: event,
    }
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

/// POST JSON to `url` with Content-Type: application/json.
/// Returns (status, body_bytes).
async fn post_json(
    client: &Client<HttpConnector, Full<Bytes>>,
    url: &str,
    body: Vec<u8>,
) -> (StatusCode, Bytes) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(url)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, body)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Full happy-path loopback:
/// node-a sends a valid event → node-b accepts → ACK forwarded → node-a records it.
#[test]
fn two_node_event_accepted_and_ack_received() {
    let node_a_url = spawn_node(build_state("did:arrow:pilot:node-a"));
    let node_b_url = spawn_node(build_state("did:arrow:pilot:node-b"));

    let envelope = make_envelope(
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        valid_event(),
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client: Client<HttpConnector, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();

        // ── Step 1: event → node-b ────────────────────────────────────────────
        let (status, body) = post_json(
            &client,
            &format!("{node_b_url}/arrow/events"),
            serde_json::to_vec(&envelope).unwrap(),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::OK,
            "node-b must return 200 for valid event"
        );
        let ack: AckEnvelope =
            serde_json::from_slice(&body).expect("node-b response must deserialize as AckEnvelope");

        assert_eq!(
            ack.result,
            AckResult::Accepted,
            "valid event must be accepted"
        );
        assert_eq!(ack.sender_id, "did:arrow:pilot:node-b");
        assert_eq!(ack.receiver_id, "did:arrow:pilot:node-a");
        assert_eq!(ack.channel_binding, "http-rest-local");
        assert!(
            ack.ref_envelope_id.as_deref() == Some(envelope.envelope_id.as_str()),
            "ACK must reference the original envelope_id"
        );

        // ── Step 2: ACK → node-a ─────────────────────────────────────────────
        let (ack_status, ack_body) = post_json(
            &client,
            &format!("{node_a_url}/arrow/ack"),
            serde_json::to_vec(&ack).unwrap(),
        )
        .await;

        assert_eq!(
            ack_status,
            StatusCode::OK,
            "node-a must accept the forwarded ACK"
        );
        // The ack handler returns an empty JSON object on success.
        let ack_resp: Value = serde_json::from_slice(&ack_body).unwrap();
        assert!(
            ack_resp.as_object().map(|o| o.is_empty()).unwrap_or(false),
            "node-a ACK response must be an empty JSON object"
        );
    });
}

/// Semantic rejection loopback:
/// node-a sends an invalid event → node-b rejects (result=rejected, HTTP 200)
/// → the rejected ACK is still forwarded to node-a and accepted (HTTP 200).
#[test]
fn two_node_semantic_rejection_ack_is_forwarded() {
    let node_a_url = spawn_node(build_state("did:arrow:pilot:node-a"));
    let node_b_url = spawn_node(build_state("did:arrow:pilot:node-b"));

    let envelope = make_envelope(
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        invalid_event(),
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client: Client<HttpConnector, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();

        // ── Step 1: invalid event → node-b ───────────────────────────────────
        let (status, body) = post_json(
            &client,
            &format!("{node_b_url}/arrow/events"),
            serde_json::to_vec(&envelope).unwrap(),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::OK,
            "node-b returns 200 even for semantic rejections"
        );
        let ack: AckEnvelope =
            serde_json::from_slice(&body).expect("node-b response must deserialize as AckEnvelope");
        assert_eq!(
            ack.result,
            AckResult::Rejected,
            "semantically invalid event must be rejected"
        );

        // ── Step 2: rejected ACK → node-a ────────────────────────────────────
        let (ack_status, _) = post_json(
            &client,
            &format!("{node_a_url}/arrow/ack"),
            serde_json::to_vec(&ack).unwrap(),
        )
        .await;

        assert_eq!(
            ack_status,
            StatusCode::OK,
            "node-a must accept rejected ACKs"
        );
    });
}

/// Replay protection: sending the same envelope twice to node-b rejects the
/// second delivery with HTTP 409 (TPT-REPLAY-DETECTED).
/// The first delivery must have been accepted.
#[test]
fn two_node_replay_rejected_on_second_post() {
    let _node_a_url = spawn_node(build_state("did:arrow:pilot:node-a"));
    let node_b_url = spawn_node(build_state("did:arrow:pilot:node-b"));

    // Same envelope sent twice — same nonce, same envelope_id.
    let envelope = make_envelope(
        "did:arrow:pilot:node-a",
        "did:arrow:pilot:node-b",
        valid_event(),
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client: Client<HttpConnector, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();
        let body = serde_json::to_vec(&envelope).unwrap();

        // First delivery — must be accepted.
        let (status1, body1) =
            post_json(&client, &format!("{node_b_url}/arrow/events"), body.clone()).await;
        assert_eq!(status1, StatusCode::OK);
        let ack: AckEnvelope = serde_json::from_slice(&body1).unwrap();
        assert_eq!(ack.result, AckResult::Accepted);

        // Second delivery with identical nonce — must be rejected (409).
        let (status2, body2) =
            post_json(&client, &format!("{node_b_url}/arrow/events"), body).await;
        assert_eq!(status2, StatusCode::CONFLICT, "replay must return 409");
        let error: Value = serde_json::from_slice(&body2).unwrap();
        assert_eq!(
            error["error_code"].as_str(),
            Some("TPT-REPLAY-DETECTED"),
            "error_code must be TPT-REPLAY-DETECTED"
        );
    });
}

/// Health-check loopback: both nodes report their respective node_ids.
#[test]
fn two_node_health_identifies_each_node() {
    let node_a_url = spawn_node(build_state("did:arrow:pilot:node-a"));
    let node_b_url = spawn_node(build_state("did:arrow:pilot:node-b"));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client: Client<HttpConnector, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();

        for (url, expected_id) in [
            (node_a_url.as_str(), "did:arrow:pilot:node-a"),
            (node_b_url.as_str(), "did:arrow:pilot:node-b"),
        ] {
            let req = Request::builder()
                .method(Method::GET)
                .uri(format!("{url}/arrow/health"))
                .body(Full::new(Bytes::new()))
                .unwrap();
            let resp = client.request(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = resp.into_body().collect().await.unwrap().to_bytes();
            let health: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                health["node_id"].as_str(),
                Some(expected_id),
                "health endpoint must report the correct node_id"
            );
        }
    });
}
