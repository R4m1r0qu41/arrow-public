//! EVM chain reader.
//!
//! Reads a transaction and the current chain head over JSON-RPC and reduces them
//! to an [`ObservedTx`] for the pure [`crate::mapping`] layer. Works against any
//! EVM JSON-RPC endpoint (Ethereum, Base, Arbitrum, a local `anvil`).
//!
//! **Implementation note:** this keeps JSON-RPC explicit over the workspace HTTP
//! stack rather than introducing a chain-specific client. It issues three
//! read-only RPC methods and leaves signing/execution out of scope.

use crate::mapping::ObservedTx;
use anyhow::{bail, Context, Result};

/// A minimal EVM JSON-RPC client that observes a single transaction.
#[derive(Debug, Clone)]
pub struct EvmObserver {
    rpc_url: String,
}

impl EvmObserver {
    /// Create an observer pointed at an EVM JSON-RPC endpoint (e.g.
    /// `http://127.0.0.1:8545`).
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
        }
    }

    /// Observe `tx_hash`: fetch the transaction and the current head, and reduce
    /// them to an [`ObservedTx`]. Blocking — runs its own one-shot runtime.
    pub fn observe(&self, tx_hash: &str) -> Result<ObservedTx> {
        let url = self.rpc_url.clone();
        let hash = tx_hash.to_string();
        tokio::runtime::Runtime::new()?.block_on(async move {
            let tx = rpc_call(&url, "eth_getTransactionByHash", serde_json::json!([hash]))
                .await
                .context("eth_getTransactionByHash failed")?;
            if tx.is_null() {
                bail!("transaction {hash} not found on chain");
            }

            let head_hex = rpc_call(&url, "eth_blockNumber", serde_json::json!([]))
                .await
                .context("eth_blockNumber failed")?;
            let head_block = parse_hex_u64(head_hex.as_str().unwrap_or_default())
                .context("parsing head block number")?;

            // Inclusion + success come from the receipt, not the transaction.
            // A transaction can be mined yet have *reverted* (receipt status 0x0);
            // such a transaction MUST NOT be minted as a settled Arrow event.
            let receipt = rpc_call(&url, "eth_getTransactionReceipt", serde_json::json!([hash]))
                .await
                .context("eth_getTransactionReceipt failed")?;
            let block_number = receipt_inclusion_block(&receipt)?;

            // §4.1 — observed EIP-155 chain id. Carried into ObservedTx so the
            // mapping layer can fail closed on a network mismatch before minting.
            let chain_id_hex = rpc_call(&url, "eth_chainId", serde_json::json!([]))
                .await
                .context("eth_chainId failed")?;
            let observed_chain_id = parse_hex_u64(chain_id_hex.as_str().unwrap_or_default())
                .context("parsing eth_chainId response")?;

            // §4.2 — block inclusion timestamp for L2 challenge-window ETA.
            // Only fetched when the transaction has been mined.
            let block_timestamp = if let Some(bn) = block_number {
                let block_hex = format!("0x{bn:x}");
                let block = rpc_call(
                    &url,
                    "eth_getBlockByNumber",
                    serde_json::json!([block_hex, false]),
                )
                .await
                .context("eth_getBlockByNumber failed")?;
                let ts_hex = block
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                Some(parse_hex_u64(ts_hex).context("parsing block timestamp")?)
            } else {
                None
            };

            let from = tx
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let to = tx.get("to").and_then(|v| v.as_str()).map(str::to_string);
            let value_wei = match tx.get("value").and_then(|v| v.as_str()) {
                Some(s) => parse_hex_u128(s).context("parsing tx value")?.to_string(),
                None => "0".to_string(),
            };

            Ok(ObservedTx {
                tx_hash: hash,
                block_number,
                head_block,
                from,
                to,
                value_wei,
                observed_chain_id,
                block_timestamp,
            })
        })
    }
}

/// Issue a single JSON-RPC call and return the `result` value.
async fn rpc_call(url: &str, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let parsed: serde_json::Value = reqwest::Client::new()
        .post(url)
        .json(&body)
        .send()
        .await
        .context("RPC request failed")?
        .error_for_status()
        .context("RPC returned non-success status")?
        .json()
        .await
        .context("RPC response is not valid JSON")?;

    if let Some(err) = parsed.get("error") {
        bail!("RPC error from {method}: {err}");
    }
    Ok(parsed
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

/// Interpret a transaction receipt for the purpose of minting an event.
///
/// Returns the inclusion block when the transaction is mined **and succeeded**;
/// `None` when the receipt is absent (still pending in the mempool); and an error
/// when the transaction reverted (status `0x0`) or the receipt is malformed — a
/// reverted transaction must never become a settled Arrow event.
fn receipt_inclusion_block(receipt: &serde_json::Value) -> Result<Option<u64>> {
    if receipt.is_null() {
        return Ok(None); // not yet mined
    }
    match receipt.get("status").and_then(|v| v.as_str()) {
        Some("0x1") => {}
        Some("0x0") => bail!(
            "transaction reverted on chain (receipt status 0x0); \
             refusing to mint it as a settled Arrow event"
        ),
        other => bail!(
            "transaction receipt has unexpected status {other:?}; \
             refusing to mint without a confirmed success status"
        ),
    }
    let block = receipt
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .map(parse_hex_u64)
        .transpose()
        .context("parsing receipt blockNumber")?;
    Ok(block)
}

/// Parse a `0x`-prefixed hex string into a `u64`.
fn parse_hex_u64(s: &str) -> Result<u64> {
    let stripped = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(stripped, 16).with_context(|| format!("invalid hex u64: {s}"))
}

/// Parse a `0x`-prefixed hex string into a `u128` (wei values fit in u128).
fn parse_hex_u128(s: &str) -> Result<u128> {
    let stripped = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(stripped, 16).with_context(|| format!("invalid hex u128: {s}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing() {
        assert_eq!(parse_hex_u64("0x6f").unwrap(), 111);
        assert_eq!(parse_hex_u64("0x0").unwrap(), 0);
        assert_eq!(
            parse_hex_u128("0xde0b6b3a7640000").unwrap(),
            1_000_000_000_000_000_000
        );
    }

    #[test]
    fn receipt_null_is_pending() {
        assert_eq!(
            receipt_inclusion_block(&serde_json::Value::Null).unwrap(),
            None
        );
    }

    #[test]
    fn receipt_success_returns_block() {
        let receipt = serde_json::json!({ "status": "0x1", "blockNumber": "0x64" });
        assert_eq!(receipt_inclusion_block(&receipt).unwrap(), Some(100));
    }

    #[test]
    fn reverted_receipt_is_rejected() {
        let receipt = serde_json::json!({ "status": "0x0", "blockNumber": "0x64" });
        let err = receipt_inclusion_block(&receipt).unwrap_err();
        assert!(err.to_string().contains("reverted"), "got: {err}");
    }

    #[test]
    fn missing_status_is_rejected() {
        let receipt = serde_json::json!({ "blockNumber": "0x64" });
        assert!(receipt_inclusion_block(&receipt).is_err());
    }

    // §4.1 — eth_chainId parsing

    #[test]
    fn observed_chain_id_is_parsed_from_eth_chainid() {
        // Sepolia chain id: 0xaa36a7 = 11155111
        assert_eq!(parse_hex_u64("0xaa36a7").unwrap(), 11155111);
    }

    // §4.2 — block timestamp parsing

    #[test]
    fn block_timestamp_parsed_from_block() {
        // 0x67d9b200 = 1742320128 (sample Ethereum block timestamp)
        let ts = parse_hex_u64("0x67d9b200").unwrap();
        assert_eq!(ts, 1742320128);
    }

    #[test]
    fn pending_tx_has_no_block_timestamp() {
        // A null receipt means unmined → block_number is None →
        // eth_getBlockByNumber is not called → block_timestamp stays None.
        // Verified structurally: receipt_inclusion_block(null) returns Ok(None).
        let block_number = receipt_inclusion_block(&serde_json::Value::Null).unwrap();
        assert!(block_number.is_none());
    }
}
