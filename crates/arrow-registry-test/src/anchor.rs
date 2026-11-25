//! Registry anchoring — calldata submission and verification (R-004/008/010).
//!
//! Anchors a `registry_version_hash` or `entry_hash` on-chain by submitting a
//! zero-value EVM transaction whose calldata is the hash bytes. The tx hash and
//! block are recorded in the registry `anchoring` block.
//!
//! ## Hash computation
//!
//! `registry_version_hash = "sha256:" + hex(SHA256(JCS(full registry JSON)))`
//! `entry_hash            = "sha256:" + hex(SHA256(JCS(single entry JSON)))`
//!
//! Both use the same JCS canonicalizer as the artifact hash so they are
//! independently reproducible.
//!
//! ## Verification
//!
//! `anchor verify` recomputes the hash from the registry file and compares it
//! to the calldata of the anchor transaction retrieved via `eth_getTransactionByHash`.

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;
use sha2::{Digest, Sha256};

/// Recompute the `registry_version_hash` from a full registry JSON value.
pub fn registry_version_hash(registry_json: &serde_json::Value) -> Result<String> {
    let canonical =
        serde_jcs::to_vec(registry_json).context("JCS canonicalization of registry failed")?;
    let digest = Sha256::digest(&canonical);
    Ok(format!("sha256:{}", hex::encode(digest)))
}

/// Recompute the `entry_hash` from a single registry entry JSON value.
pub fn entry_hash(entry_json: &serde_json::Value) -> Result<String> {
    let canonical =
        serde_jcs::to_vec(entry_json).context("JCS canonicalization of entry failed")?;
    let digest = Sha256::digest(&canonical);
    Ok(format!("sha256:{}", hex::encode(digest)))
}

/// Parameters for submitting a calldata anchor transaction.
///
/// The transaction carries `hash_bytes` as calldata and transfers zero value.
/// Signing is via the locally-signed raw-transaction path (§4.4).
#[derive(Debug, Clone)]
pub struct AnchorParams {
    /// EVM JSON-RPC endpoint (testnet, e.g. Sepolia).
    pub rpc_url: String,
    /// Hex-encoded hash to anchor (the `sha256:` prefix is stripped).
    pub hash_hex: String,
    /// `0x`-prefixed from address (must match the private key).
    pub from: String,
    /// Raw 32-byte private key (hex, no prefix).
    pub private_key_hex: String,
}

/// On-chain anchor record, populated after a successful submission.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnchorRecord {
    pub chain: String,
    pub tx_hash: String,
    pub block: u64,
    pub anchored_at: String,
}

/// Verify that the anchor transaction on `rpc_url` contains the expected hash.
///
/// 1. Fetches the transaction by `anchor_tx_hash`.
/// 2. Extracts calldata (`input` field).
/// 3. Strips `0x` prefix and compares hex to `expected_hash` (minus `sha256:` prefix).
///
/// Returns `Ok(())` on match, `Err` on mismatch or RPC failure.
pub fn verify_anchor(rpc_url: &str, anchor_tx_hash: &str, expected_hash: &str) -> Result<()> {
    let expected_hex = expected_hash
        .strip_prefix("sha256:")
        .unwrap_or(expected_hash);

    tokio::runtime::Runtime::new()?.block_on(async move {
        let tx = rpc_call(
            rpc_url,
            "eth_getTransactionByHash",
            serde_json::json!([anchor_tx_hash]),
        )
        .await
        .context("eth_getTransactionByHash failed")?;

        if tx.is_null() {
            bail!("anchor transaction {anchor_tx_hash} not found");
        }

        let input = tx
            .get("input")
            .and_then(|v| v.as_str())
            .context("anchor tx missing 'input' field")?;
        let calldata_hex = input.strip_prefix("0x").unwrap_or(input);

        if calldata_hex != expected_hex {
            bail!("anchor calldata mismatch: expected {expected_hex}, found {calldata_hex}");
        }
        Ok(())
    })
}

async fn rpc_call(url: &str, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let req = Request::builder()
        .method(Method::POST)
        .uri(url)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(serde_json::to_vec(&body)?)))
        .context("building RPC request")?;
    let resp = client.request(req).await.context("RPC request failed")?;
    let bytes = resp
        .into_body()
        .collect()
        .await
        .context("reading RPC response")?
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&bytes).context("RPC response is not valid JSON")?;

    if let Some(err) = parsed.get("error").filter(|v| !v.is_null()) {
        bail!("RPC error from {method}: {err}");
    }
    Ok(parsed
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_version_hash_is_deterministic() {
        let json = serde_json::json!({ "registry_version": "2.0", "entries": [] });
        let h1 = registry_version_hash(&json).unwrap();
        let h2 = registry_version_hash(&json).unwrap();
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn entry_hash_differs_from_registry_hash() {
        let entry = serde_json::json!({ "code": "ETH", "entry_version": "1.0" });
        let registry = serde_json::json!({ "registry_version": "2.0", "entries": [entry.clone()] });
        let eh = entry_hash(&entry).unwrap();
        let rh = registry_version_hash(&registry).unwrap();
        assert_ne!(eh, rh);
    }
}
