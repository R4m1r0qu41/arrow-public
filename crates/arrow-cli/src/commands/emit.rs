use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::path::Path;
use uuid::Uuid;

use arrow_core::types::event::ArrowEvent;
use arrow_transport::envelope::{ReceiverId, TransportEnvelope};

/// Send an `ArrowEvent` JSON file to a node as a `TransportEnvelope`.
///
/// When `key_file` is `Some`, the envelope is signed with the Ed25519 key from
/// that file.  The DID embedded in the key file overrides `sender_id`; if the
/// two differ a warning is emitted to stderr.  When `key_file` is `None` the
/// envelope is sent unsigned (the receiving node may reject it if it requires
/// signatures).
pub fn run(
    to_url: &str,
    event_path: &Path,
    sender_id: &str,
    receiver_id: &str,
    key_file: Option<&Path>,
) -> Result<bool> {
    // Read and parse the event
    let raw = std::fs::read_to_string(event_path)
        .with_context(|| format!("cannot read {}", event_path.display()))?;
    let event: ArrowEvent = serde_json::from_str(&raw)
        .with_context(|| format!("cannot parse {} as ArrowEvent", event_path.display()))?;

    // Build nonce (overlay Section 2.16)
    let nonce = format!("n-{}", Uuid::new_v4().simple());

    let mut envelope = TransportEnvelope {
        envelope_version: "1.0".to_string(),
        envelope_id: format!("env-{}", Uuid::new_v4()),
        sender_id: sender_id.to_string(),
        receiver_id: ReceiverId::One(receiver_id.to_string()),
        channel_binding: "http-rest-local".to_string(),
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

    // Sign if a key file was provided.  The DID in the key file wins as sender_id.
    if let Some(kf) = key_file {
        // Warn when --from would be silently overridden.
        let key_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(kf).context("reading key file for DID check")?,
        )
        .context("parsing key file for DID check")?;
        if let Some(key_did) = key_json["did"].as_str() {
            if key_did != sender_id {
                eprintln!(
                    "warning: --from `{sender_id}` ignored; using DID from key file `{key_did}`"
                );
            }
        }
        super::signing::sign_envelope_in_place(&mut envelope, kf)?;
    }

    // Build URL (overlay Section 2.17 — trim trailing slash)
    let base = to_url.trim_end_matches('/');
    let url = format!("{base}/arrow/events");

    // Send via hyper (synchronous wrapper around a mini tokio runtime)
    let body_bytes = serde_json::to_vec(&envelope)?;
    let response_bytes = send_post(&url, body_bytes)?;
    let response_json: serde_json::Value =
        serde_json::from_slice(&response_bytes).context("response is not valid JSON")?;

    // Determine exit outcome (overlay Section 2.18)
    if let Some(error_code) = response_json.get("error_code") {
        // ErrorEnvelope
        let msg = response_json
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        eprintln!("Transport error: {error_code} — {msg}");
        return Ok(false);
    }

    if let Some(result) = response_json.get("result").and_then(|v| v.as_str()) {
        let passed = result == "accepted";

        // Print a human-readable summary
        let status = if passed { "ACCEPTED" } else { "REJECTED" };
        let ref_id = response_json
            .get("ref_envelope_id")
            .and_then(|v| v.as_str())
            .unwrap_or("—");
        println!("[{status}]  ref: {ref_id}");

        if let Some(vr) = response_json.get("validation_result") {
            if let Some(violations) = vr.get("violations").and_then(|v| v.as_array()) {
                for v in violations {
                    let rid = v.get("rule_id").and_then(|x| x.as_str()).unwrap_or("?");
                    let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("");
                    let sev = v
                        .get("severity")
                        .and_then(|x| x.as_str())
                        .unwrap_or("ERROR");
                    println!("  [{sev}] {rid}: {msg}");
                }
            }
        }

        return Ok(passed);
    }

    bail!("unrecognised response shape: {response_json}");
}

/// Blocking HTTP POST using a one-shot tokio runtime + hyper.
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
