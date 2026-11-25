use axum::{
    extract::{connect_info::ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_transport::{
    canonical::envelope_hash,
    envelope::{AckEnvelope, AckResult, AckType, ErrorEnvelope, TransportEnvelope},
    error::TransportError,
    signature::{verify_envelope_signature, SignatureVerificationPolicy},
    validation::{validate_envelope, TransportOutcome},
};

use crate::{
    config::{TlsMode, TlsRuntimeConfig},
    state::AppState,
};

/// Unified response type — ack or transport error envelope.
#[derive(Serialize)]
#[serde(untagged)]
pub enum EventResponse {
    Ack(AckEnvelope),
    Error(ErrorEnvelope),
}

// ── Content-Type guard ────────────────────────────────────────────────────────

/// Returns true if the Content-Type header indicates JSON.
/// Accepts "application/json" and "application/json; charset=utf-8" (and variants).
fn is_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            let base = s.split(';').next().unwrap_or("").trim();
            base.eq_ignore_ascii_case("application/json")
        })
        .unwrap_or(false)
}

fn unsupported_media_type(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        Json(json!({ "error_code": "TPT-MALFORMED", "error_message": msg })),
    )
}

fn forwarded_proto_is_https(headers: &HeaderMap) -> bool {
    if headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|proto| proto.trim().eq_ignore_ascii_case("https"))
        .unwrap_or(false)
    {
        return true;
    }

    headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value.split(';').any(|part| {
                let part = part.trim();
                part.strip_prefix("proto=")
                    .map(|proto| proto.trim_matches('"').eq_ignore_ascii_case("https"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn validate_tls_request(
    tls: &TlsRuntimeConfig,
    headers: &HeaderMap,
    peer_addr: Option<SocketAddr>,
) -> Result<(), String> {
    if tls.mode != TlsMode::TerminatedUpstream {
        return Ok(());
    }

    let trusted_proxy = tls
        .trusted_proxy
        .ok_or_else(|| "https-rest requires a configured trusted proxy".to_string())?;
    let peer_addr =
        peer_addr.ok_or_else(|| "peer address unavailable for trusted proxy check".to_string())?;

    if peer_addr.ip() != trusted_proxy {
        return Err(format!(
            "request peer `{}` is not the configured trusted proxy `{}`",
            peer_addr.ip(),
            trusted_proxy
        ));
    }

    if tls.require_forwarded_proto_https && !forwarded_proto_is_https(headers) {
        return Err("https-rest requires X-Forwarded-Proto or Forwarded proto=https".to_string());
    }

    Ok(())
}

fn tls_error_envelope(state: &AppState, message: String) -> (StatusCode, Json<EventResponse>) {
    let err = TransportError::ChannelMismatch(message);
    let ev = err.into_error_envelope(
        state.node_id.clone(),
        "unknown".to_string(),
        state.binding.code.clone(),
        None,
        None,
    );
    (
        StatusCode::from_u16(TransportError::ChannelMismatch(String::new()).http_status())
            .unwrap_or(StatusCode::CONFLICT),
        Json(EventResponse::Error(ev)),
    )
}

// ── GET /arrow/health ─────────────────────────────────────────────────────────

pub async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "node_id": state.node_id,
        "channel_binding": state.binding.code,
        "transport_model": "1.0"
    }))
}

// ── POST /arrow/events ────────────────────────────────────────────────────────

pub async fn receive_event(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> (StatusCode, Json<EventResponse>) {
    // Content-Type guard — 415 on missing or non-JSON Content-Type
    if !is_json_content_type(&headers) {
        let err = TransportError::Malformed("Content-Type must be application/json".to_string());
        let ev = err.into_error_envelope(
            state.node_id.clone(),
            "unknown".to_string(),
            state.binding.code.clone(),
            None,
            None,
        );
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Json(EventResponse::Error(ev)),
        );
    }

    if state.binding.code == "https-rest" {
        if let Err(message) = validate_tls_request(&state.tls, &headers, Some(peer_addr)) {
            return tls_error_envelope(state.as_ref(), message);
        }
    }

    // Step 1 — Deserialize. Failure → 400 TPT-MALFORMED
    let envelope: TransportEnvelope = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            let err = TransportError::Malformed(format!("envelope not parseable: {e}"));
            let ev = err.into_error_envelope(
                state.node_id.clone(),
                "unknown".to_string(),
                state.binding.code.clone(),
                None,
                None,
            );
            return (StatusCode::BAD_REQUEST, Json(EventResponse::Error(ev)));
        }
    };

    // Helper closure — build error response from a parsed envelope
    let make_error = |err: TransportError| {
        let http = StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST);
        let hash = envelope_hash(&envelope).ok();
        let ev = err.into_error_envelope(
            state.node_id.clone(),
            envelope.sender_id.clone(),
            envelope.channel_binding.clone(),
            Some(envelope.envelope_id.clone()),
            hash,
        );
        (http, Json(EventResponse::Error(ev)))
    };

    // Step 2 — Stateless transport validation (TPT-001..005, 010, 040, 041)
    let check = validate_envelope(&envelope, state.registries.as_ref());
    let warnings = check.warnings;
    if let TransportOutcome::Rejected(err) = check.outcome {
        return make_error(err);
    }

    // Step 3 — Channel binding match
    if envelope.channel_binding != state.binding.code {
        return make_error(TransportError::ChannelMismatch(format!(
            "channel_binding `{}` does not match this endpoint",
            envelope.channel_binding
        )));
    }

    // Step 4 — Receiver match
    if !envelope.receiver_id.contains(&state.node_id) {
        return make_error(TransportError::ReceiverMismatch(format!(
            "receiver_id does not include this node `{}`",
            state.node_id
        )));
    }

    // Step 5 — Known sender
    if !state.known_senders.contains(&envelope.sender_id) {
        return make_error(TransportError::UnknownSender(format!(
            "sender `{}` is not in the allowlist",
            envelope.sender_id
        )));
    }

    // Step 6 — Signature verification
    let signature_policy = SignatureVerificationPolicy {
        require_signature: state.identity.require_signature,
        allowed_did_methods: state.identity.allowed_did_methods.clone(),
        allowed_algorithms: state.identity.allowed_algorithms.clone(),
    };
    if let Err(err) = verify_envelope_signature(&envelope, &signature_policy) {
        return make_error(err);
    }

    // Step 7 — Replay protection
    match state
        .replay
        .check_and_record(&envelope.sender_id, &envelope.nonce)
    {
        Ok(true) => {
            return make_error(TransportError::ReplayDetected(format!(
                "nonce `{}` from `{}` already seen",
                envelope.nonce, envelope.sender_id
            )));
        }
        Ok(false) => {}
        Err(err) => {
            return make_error(TransportError::ReplayStoreUnavailable(err.to_string()));
        }
    }

    // Step 8 — Semantic validation
    let pipeline = ValidationPipeline::new(state.registries.as_ref());
    let validation_result = pipeline.validate_event(&envelope.payload);

    // Step 9 — Build validation ACK (HTTP 200 for both accepted and rejected — Part 5.4)
    let hash = envelope_hash(&envelope).ok();
    let ack_result = if validation_result.passed {
        AckResult::Accepted
    } else {
        AckResult::Rejected
    };
    let details = if warnings.is_empty() {
        None
    } else {
        Some(json!({ "warnings": warnings }))
    };

    let ack = AckEnvelope {
        ack_version: "1.0".to_string(),
        ack_id: format!("ack-{}", Uuid::new_v4()),
        ack_type: AckType::Validation,
        ref_envelope_id: Some(envelope.envelope_id.clone()),
        ref_envelope_hash: hash,
        sender_id: state.node_id.clone(),
        receiver_id: envelope.sender_id.clone(),
        channel_binding: state.binding.code.clone(),
        sent_at: Utc::now().to_rfc3339(),
        result: ack_result,
        validation_result: Some(validation_result),
        details,
    };

    (StatusCode::OK, Json(EventResponse::Ack(ack)))
}

// ── POST /arrow/ack ───────────────────────────────────────────────────────────

pub async fn receive_ack(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    // Content-Type guard — 415
    if !is_json_content_type(&headers) {
        let (s, j) = unsupported_media_type("Content-Type must be application/json");
        return (s, j);
    }

    if state.binding.code == "https-rest" {
        if let Err(message) = validate_tls_request(&state.tls, &headers, Some(peer_addr)) {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error_code": "TPT-CHANNEL-MISMATCH",
                    "error_message": message
                })),
            );
        }
    }

    // Deserialize
    let ack: AckEnvelope = match serde_json::from_slice(&body) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error_code": "TPT-MALFORMED",
                    "error_message": format!("ack not parseable: {e}")
                })),
            );
        }
    };

    // ack_version == "1.0"
    if ack.ack_version != "1.0" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_code": "TPT-MALFORMED",
                "error_message": format!("ack_version `{}` is not supported; expected \"1.0\"", ack.ack_version)
            })),
        );
    }

    // channel_binding matches this configured endpoint.
    if ack.channel_binding != state.binding.code {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_code": "TPT-MALFORMED",
                "error_message": format!("ack channel_binding `{}` does not match this binding", ack.channel_binding)
            })),
        );
    }

    // receiver_id includes this node
    if ack.receiver_id != state.node_id {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error_code": "TPT-RECEIVER-MISMATCH",
                "error_message": format!("ack receiver_id `{}` does not match this node `{}`", ack.receiver_id, state.node_id)
            })),
        );
    }

    // sender_id is allowlisted
    if !state.known_senders.contains(&ack.sender_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error_code": "TPT-UNKNOWN-SENDER",
                "error_message": format!("ack sender `{}` is not in the allowlist", ack.sender_id)
            })),
        );
    }

    // sent_at is valid RFC 3339 and not more than 5 minutes in the future
    match DateTime::parse_from_rfc3339(&ack.sent_at) {
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error_code": "TPT-MALFORMED",
                    "error_message": format!("ack sent_at `{}` is not a valid RFC 3339 timestamp", ack.sent_at)
                })),
            );
        }
        Ok(dt) => {
            let sent = dt.with_timezone(&Utc);
            if sent > Utc::now() + Duration::minutes(5) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error_code": "TPT-MALFORMED",
                        "error_message": "ack sent_at is more than 5 minutes in the future"
                    })),
                );
            }
        }
    }

    // ref_envelope_id or ref_envelope_hash required (TPT-030)
    if ack.ref_envelope_id.as_deref().unwrap_or("").is_empty()
        && ack.ref_envelope_hash.as_deref().unwrap_or("").is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_code": "TPT-MALFORMED",
                "error_message": "ack must contain ref_envelope_id or ref_envelope_hash (TPT-030)"
            })),
        );
    }

    tracing::info!(
        ack_id = %ack.ack_id,
        ref_envelope_id = ?ack.ref_envelope_id,
        result = ?ack.result,
        node_id = %state.node_id,
        "received outbound acknowledgment"
    );

    (StatusCode::OK, Json(json!({})))
}
