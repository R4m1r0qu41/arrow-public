//! Wrap a minted [`ArrowEvent`] in a transport envelope, sign it, and POST it to
//! an Arrow node.
//!
//! Shared by every EVM-family Origin Adapter. The signing logic is inlined here
//! rather than shared through `arrow-transport`, which is deliberately kept free
//! of crypto dependencies; all call sites use the same canonicalisation
//! ([`arrow_transport::canonical::envelope_canonical_bytes`]) and signer
//! ([`arrow_identity::crypto::ed25519::sign_ed25519`]), so they stay compatible.
//!
//! Envelopes are emitted on the active `http-rest-local` channel binding; the
//! legacy `http-rest` binding is inactive (HTTP REST TLS hardening v0.2).

use std::path::Path;

use anyhow::{bail, Context, Result};
use base64::Engine;
use chrono::Utc;
use uuid::Uuid;

use arrow_core::types::event::ArrowEvent;
use arrow_identity::crypto::ed25519::sign_ed25519;
use arrow_transport::canonical::envelope_canonical_bytes;
use arrow_transport::envelope::{EnvelopeSigningMetadata, ReceiverId, TransportEnvelope};

/// Active local-development channel binding (`http-rest` is inactive in v0.2).
pub const CHANNEL_BINDING: &str = "http-rest-local";

/// Outcome of emitting an event to a node.
#[derive(Debug, Clone)]
pub struct EmitOutcome {
    /// Whether the node accepted the event (ACK result == "accepted").
    pub accepted: bool,
    /// Referenced envelope id echoed back by the node, if any.
    pub ref_envelope_id: Option<String>,
    /// Human-readable lines describing any validation violations.
    pub violations: Vec<String>,
}

/// Build a transport envelope around `event`, signing it when `key_file` is
/// `Some`.
///
/// When signed, the DID from the key file becomes the `sender_id` (it must equal
/// the signing key's DID — the signature covers `sender_id`). The returned
/// envelope is exactly what would be POSTed to a node, so it can also be written
/// to a file for a Consumption Adapter to verify.
pub fn build_signed_envelope(
    event: ArrowEvent,
    sender_id: &str,
    receiver_id: &str,
    key_file: Option<&Path>,
) -> Result<TransportEnvelope> {
    let nonce = format!("n-{}", Uuid::new_v4().simple());

    let mut envelope = TransportEnvelope {
        envelope_version: "1.0".to_string(),
        envelope_id: format!("env-{}", Uuid::new_v4()),
        sender_id: sender_id.to_string(),
        receiver_id: ReceiverId::One(receiver_id.to_string()),
        channel_binding: CHANNEL_BINDING.to_string(),
        sent_at: Utc::now().to_rfc3339(),
        nonce,
        sequence: None,
        signature: None,
        signing: None,
        correlation_id: None,
        reply_to: None,
        metadata: None,
        payload: event,
    };

    if let Some(kf) = key_file {
        sign_envelope_in_place(&mut envelope, kf)?;
    }

    Ok(envelope)
}

/// POST an already-built envelope to `<to_url>/arrow/events` and interpret the ACK.
pub fn post_envelope(envelope: &TransportEnvelope, to_url: &str) -> Result<EmitOutcome> {
    let base = to_url.trim_end_matches('/');
    let url = format!("{base}/arrow/events");
    let body = serde_json::to_vec(envelope)?;
    let response_bytes = send_post(&url, body)?;
    let response: serde_json::Value =
        serde_json::from_slice(&response_bytes).context("response is not valid JSON")?;

    if let Some(error_code) = response.get("error_code").and_then(|v| v.as_str()) {
        let msg = response
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        bail!("transport error: {error_code} — {msg}");
    }

    let result = response
        .get("result")
        .and_then(|v| v.as_str())
        .with_context(|| format!("unrecognised response shape: {response}"))?;

    let mut violations = Vec::new();
    if let Some(arr) = response
        .get("validation_result")
        .and_then(|vr| vr.get("violations"))
        .and_then(|v| v.as_array())
    {
        for v in arr {
            let rid = v.get("rule_id").and_then(|x| x.as_str()).unwrap_or("?");
            let sev = v
                .get("severity")
                .and_then(|x| x.as_str())
                .unwrap_or("ERROR");
            let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("");
            violations.push(format!("[{sev}] {rid}: {msg}"));
        }
    }

    Ok(EmitOutcome {
        accepted: result == "accepted",
        ref_envelope_id: response
            .get("ref_envelope_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        violations,
    })
}

/// Build a signed envelope around `event` and POST it to `<to_url>/arrow/events`.
///
/// Convenience wrapper over [`build_signed_envelope`] + [`post_envelope`]. When
/// `key_file` is `None` the envelope is unsigned and a node that requires
/// signatures will reject it.
pub fn emit_event(
    event: ArrowEvent,
    to_url: &str,
    sender_id: &str,
    receiver_id: &str,
    key_file: Option<&Path>,
) -> Result<EmitOutcome> {
    let envelope = build_signed_envelope(event, sender_id, receiver_id, key_file)?;
    post_envelope(&envelope, to_url)
}

/// Sign an envelope in place from a dev key file (`{ private_key_b64, key_ref, did }`).
///
/// The DID from the key file wins as `sender_id`, `signing` is cleared before
/// canonicalisation, and the signature is over the canonical bytes
/// (`arrow-canonical-json-v0`).
///
/// **Dev / test only — key files carry unprotected private keys.**
fn sign_envelope_in_place(envelope: &mut TransportEnvelope, key_file: &Path) -> Result<()> {
    let key_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(key_file).context("reading key file")?)
            .context("parsing key file")?;

    let sk_b64 = key_json["private_key_b64"]
        .as_str()
        .context("missing private_key_b64 in key file")?;
    let key_ref = key_json["key_ref"]
        .as_str()
        .context("missing key_ref in key file")?;
    let did = key_json["did"]
        .as_str()
        .context("missing did in key file")?;

    let sk_bytes = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(sk_b64)
        .context("decoding private key (expected STANDARD_NO_PAD base64)")?;
    let sk: [u8; 32] = sk_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("private key must be exactly 32 bytes"))?;

    // DID from the key file wins — sender_id is covered by the signature.
    envelope.sender_id = did.to_string();
    // `signing` must be absent during canonicalisation (canonical bytes exclude it).
    envelope.signing = None;

    let canonical = envelope_canonical_bytes(envelope)
        .map_err(|e| anyhow::anyhow!("canonicalisation failed: {e}"))?;
    let sig = sign_ed25519(&sk, &canonical);
    let sig_b64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(sig);

    envelope.signing = Some(EnvelopeSigningMetadata {
        alg: "Ed25519".into(),
        key_ref: key_ref.into(),
        sig_b64,
        created: Some(Utc::now().to_rfc3339()),
        canonicalization: "arrow-canonical-json-v0".into(),
        signed_fields: "envelope-minus-signature-and-signing".into(),
    });

    Ok(())
}

/// Blocking HTTP POST using a one-shot tokio runtime + hyper, matching the CLI.
fn send_post(url: &str, body: Vec<u8>) -> Result<Vec<u8>> {
    let url_owned = url.to_string();
    tokio::runtime::Runtime::new()?.block_on(async move {
        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};
        use hyper::{Method, Request};
        use hyper_util::client::legacy::{connect::HttpConnector, Client};
        use hyper_util::rt::TokioExecutor;

        let client: Client<HttpConnector, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();
        let req = Request::builder()
            .method(Method::POST)
            .uri(&url_owned)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .context("failed to build HTTP request")?;
        let resp = client.request(req).await.context("HTTP request failed")?;
        let bytes = resp
            .into_body()
            .collect()
            .await
            .context("reading response body")?
            .to_bytes();
        Ok(bytes.to_vec())
    })
}
