//! Bitcoin JSON-RPC chain reader.
//!
//! Reads a transaction and the current block height from a Bitcoin Core
//! JSON-RPC endpoint (`getrawtransaction` + `getblock` + `getblockcount`) and
//! reduces a specific output to an [`ObservedUtxo`].
//!
//! **v0.1 scope:** a single tracked output (by txid + vout index). UTXO
//! selection, PSBTs, and multi-input tracking are out of scope.

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;

use crate::mapping::ObservedUtxo;

/// A minimal Bitcoin JSON-RPC client that observes a single UTXO output.
#[derive(Debug, Clone)]
pub struct BtcObserver {
    rpc_url: String,
    /// RPC username for `Authorization: Basic` header.
    rpc_user: String,
    rpc_pass: String,
}

impl BtcObserver {
    /// Create an observer pointed at a Bitcoin Core JSON-RPC endpoint
    /// (e.g. `http://127.0.0.1:8332`).
    pub fn new(
        rpc_url: impl Into<String>,
        rpc_user: impl Into<String>,
        rpc_pass: impl Into<String>,
    ) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            rpc_user: rpc_user.into(),
            rpc_pass: rpc_pass.into(),
        }
    }

    /// Observe output `vout` of transaction `txid`.
    ///
    /// Fetches the raw transaction (verbose), determines confirmation depth,
    /// and reduces the named output to an [`ObservedUtxo`]. Also calls
    /// `getblockchaininfo` to bind the observed network identity (`"main"`,
    /// `"signet"`, `"test"`, `"regtest"`) into the artifact. Blocking.
    pub fn observe(&self, txid: &str, vout: u32) -> Result<ObservedUtxo> {
        let url = self.rpc_url.clone();
        let auth = format!("{}:{}", self.rpc_user, self.rpc_pass);
        let txid = txid.to_string();
        tokio::runtime::Runtime::new()?.block_on(async move {
            // §4.7 — bind observed network identity for artifact generation.
            let chain_info = rpc_call(&url, &auth, "getblockchaininfo", serde_json::json!([]))
                .await
                .context("getblockchaininfo failed")?;
            let observed_network = chain_info
                .get("chain")
                .and_then(|v| v.as_str())
                .map(str::to_string);

            // Verbose = true returns parsed JSON with vout array. If txindex
            // is unavailable, fall back to wallet-local gettransaction verbose
            // output; this supports pruned nodes for transactions sent by the
            // configured Bitcoin Core wallet.
            let tx = match rpc_call(
                &url,
                &auth,
                "getrawtransaction",
                serde_json::json!([txid.clone(), true]),
            )
            .await
            {
                Ok(tx) => tx,
                Err(raw_err) => {
                    let wallet_tx = rpc_call(
                        &url,
                        &auth,
                        "gettransaction",
                        serde_json::json!([txid.clone(), false, true]),
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "getrawtransaction failed ({raw_err}); \
                             gettransaction verbose fallback failed"
                        )
                    })?;
                    wallet_transaction_to_raw_shape(&wallet_tx, &txid)?
                }
            };

            if tx.is_null() {
                bail!("transaction {txid} not found (check txindex=1 is enabled)");
            }

            let confirmations = tx
                .get("confirmations")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let outputs = tx
                .get("vout")
                .and_then(|v| v.as_array())
                .context("missing vout array in raw transaction")?;

            let output = outputs.get(vout as usize).with_context(|| {
                format!(
                    "vout {vout} out of range (tx has {} outputs)",
                    outputs.len()
                )
            })?;

            let amount_btc = output
                .get("value")
                .and_then(|v| v.as_f64())
                .context("missing or non-numeric output value")?;
            // Convert BTC (float) → satoshis (integer). Bitcoin Core returns 8
            // decimal places; round to nearest sat to absorb floating-point drift.
            let amount_sats = (amount_btc * 100_000_000.0).round() as u64;

            // Fail closed: non-standard outputs (OP_RETURN, bare multisig, unknown
            // scriptPubKey shapes) have no /scriptPubKey/address. Silently mapping
            // those to "" would mint a transfer with an empty recipient, so we
            // reject them explicitly. P2PKH/P2SH/P2WPKH/P2WSH outputs always carry
            // this field in Bitcoin Core's verbose getrawtransaction response.
            let recipient = output
                .pointer("/scriptPubKey/address")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .with_context(|| {
                    format!(
                        "output vout={vout} has no resolvable address \
                         (non-standard, OP_RETURN, or bare multisig output); \
                         refusing to mint a transfer with an empty recipient"
                    )
                })?
                .to_string();

            // Simplified: take the address from the first input's scriptSig/prevout.
            // A production observer would resolve all input addresses.
            let from = tx
                .pointer("/vin/0/address")
                .or_else(|| tx.pointer("/vin/0/scriptSig/asm"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok(ObservedUtxo {
                txid,
                vout,
                from,
                recipient,
                amount_sats,
                confirmations,
                observed_network,
            })
        })
    }
}

fn wallet_transaction_to_raw_shape(
    wallet_tx: &serde_json::Value,
    txid: &str,
) -> Result<serde_json::Value> {
    let mut decoded = wallet_tx
        .get("decoded")
        .and_then(|v| v.as_object())
        .cloned()
        .context("wallet gettransaction response missing decoded transaction")?;

    decoded
        .entry("txid".to_string())
        .or_insert_with(|| serde_json::Value::String(txid.to_string()));

    if let Some(confirmations) = wallet_tx.get("confirmations") {
        decoded.insert("confirmations".to_string(), confirmations.clone());
    }
    if let Some(blockhash) = wallet_tx.get("blockhash") {
        decoded.insert("blockhash".to_string(), blockhash.clone());
    }
    if let Some(blocktime) = wallet_tx.get("blocktime") {
        decoded.insert("blocktime".to_string(), blocktime.clone());
    }

    Ok(serde_json::Value::Object(decoded))
}

async fn rpc_call(
    url: &str,
    auth: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    use base64::Engine;
    let auth_b64 = base64::engine::general_purpose::STANDARD.encode(auth.as_bytes());

    let body = serde_json::json!({
        "jsonrpc": "1.0",
        "id": "arrow-btc-observer",
        "method": method,
        "params": params,
    });

    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let req = Request::builder()
        .method(Method::POST)
        .uri(url)
        .header("content-type", "application/json")
        .header("authorization", format!("Basic {auth_b64}"))
        .body(Full::new(Bytes::from(serde_json::to_vec(&body)?)))
        .context("building Bitcoin RPC request")?;

    let resp = client
        .request(req)
        .await
        .context("Bitcoin RPC request failed")?;
    let bytes = resp
        .into_body()
        .collect()
        .await
        .context("reading Bitcoin RPC response")?
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&bytes).context("Bitcoin RPC response is not valid JSON")?;

    if let Some(err) = parsed.get("error").filter(|v| !v.is_null()) {
        bail!("Bitcoin RPC error from {method}: {err}");
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
    fn wallet_transaction_verbose_output_normalizes_to_raw_shape() {
        let wallet_tx = serde_json::json!({
            "confirmations": 6,
            "blockhash": "0000000000000000000000000000000000000000000000000000000000000000",
            "decoded": {
                "vin": [{"txid": "prev", "vout": 0}],
                "vout": [{
                    "value": 0.00001000,
                    "n": 0,
                    "scriptPubKey": {
                        "address": "bc1qexample000000000000000000000000000000"
                    }
                }]
            }
        });

        let tx = wallet_transaction_to_raw_shape(&wallet_tx, "abc").unwrap();
        assert_eq!(tx["txid"], "abc");
        assert_eq!(tx["confirmations"], 6);
        assert_eq!(tx["vout"][0]["value"], 0.00001000);
    }
}
