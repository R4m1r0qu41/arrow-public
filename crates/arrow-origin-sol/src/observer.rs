//! Solana JSON-RPC chain reader.
//!
//! Fetches a transaction via `getTransaction` with the requested commitment
//! level and reduces a native SOL System Program transfer to an
//! [`ObservedTransaction`].
//!
//! **v0.1 scope:** single native SOL transfers via the System Program. SPL
//! token transfers and multi-instruction transactions are out of scope.

use anyhow::{bail, Context, Result};

use crate::mapping::{ObservedTransaction, SolCommitment};

/// A minimal Solana JSON-RPC client that observes a single transaction.
#[derive(Debug, Clone)]
pub struct SolObserver {
    rpc_url: String,
}

impl SolObserver {
    /// Create an observer pointed at a Solana JSON-RPC endpoint
    /// (e.g. `https://api.mainnet-beta.solana.com`).
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
        }
    }

    /// Observe `signature` at the given commitment level.
    ///
    /// Returns `None` if the transaction has not yet reached that commitment.
    /// Returns an error if the transaction failed (`meta.err != null`) — a
    /// failed Solana transaction must not be minted as an Arrow event.
    /// Also calls `getGenesisHash` to bind the cluster identity (§4.6).
    /// Blocking.
    pub fn observe(
        &self,
        signature: &str,
        commitment: SolCommitment,
    ) -> Result<Option<ObservedTransaction>> {
        let url = self.rpc_url.clone();
        let sig = signature.to_string();
        tokio::runtime::Runtime::new()?.block_on(async move {
            // §4.6 — cluster identity for lifecycle artifacts.
            let genesis_hash_val = rpc_call(&url, "getGenesisHash", serde_json::json!([]))
                .await
                .context("getGenesisHash failed")?;
            let genesis_hash = genesis_hash_val.as_str().map(str::to_string);

            let result = rpc_call(
                &url,
                "getTransaction",
                serde_json::json!([
                    sig,
                    {
                        "encoding": "jsonParsed",
                        "commitment": commitment.as_str(),
                        "maxSupportedTransactionVersion": 0
                    }
                ]),
            )
            .await
            .context("getTransaction failed")?;

            if result.is_null() {
                return Ok(None); // not yet at this commitment
            }

            // Fail closed: a transaction with meta.err != null failed on-chain.
            if let Some(err) = result.pointer("/meta/err").filter(|v| !v.is_null()) {
                bail!(
                    "Solana transaction {sig} failed on-chain (meta.err: {err}); \
                     refusing to mint a failed transaction as an Arrow event"
                );
            }

            let slot = result
                .get("slot")
                .and_then(|v| v.as_u64())
                .context("missing slot in getTransaction response")?;

            // Extract the first System Program transfer instruction (jsonParsed).
            let instructions = result
                .pointer("/transaction/message/instructions")
                .and_then(|v| v.as_array())
                .context("missing instructions array")?;

            let transfer_info = instructions
                .iter()
                .find_map(|ix| {
                    let program = ix.get("program").and_then(|v| v.as_str())?;
                    let parsed_type = ix.pointer("/parsed/type").and_then(|v| v.as_str())?;
                    if program == "system" && parsed_type == "transfer" {
                        ix.get("parsed").and_then(|p| p.get("info"))
                    } else {
                        None
                    }
                })
                .context(
                    "no System Program transfer instruction found; \
                     only native SOL transfers are supported in v0.1",
                )?;

            let from = transfer_info
                .get("source")
                .and_then(|v| v.as_str())
                .context("missing source in transfer instruction")?
                .to_string();

            let to = transfer_info
                .get("destination")
                .and_then(|v| v.as_str())
                .context("missing destination in transfer instruction")?
                .to_string();

            let lamports = transfer_info
                .get("lamports")
                .and_then(|v| v.as_u64())
                .context("missing lamports in transfer instruction")?;

            Ok(Some(ObservedTransaction {
                signature: sig,
                slot,
                commitment,
                from,
                to,
                lamports,
                genesis_hash,
            }))
        })
    }

    /// Observe at the highest available commitment, trying `finalized` first,
    /// then `confirmed`, then `processed`.
    pub fn observe_best_commitment(&self, signature: &str) -> Result<Option<ObservedTransaction>> {
        for commitment in [
            SolCommitment::Finalized,
            SolCommitment::Confirmed,
            SolCommitment::Processed,
        ] {
            if let Some(obs) = self.observe(signature, commitment)? {
                return Ok(Some(obs));
            }
        }
        Ok(None)
    }
}

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
        .context("Solana RPC request failed")?
        .error_for_status()
        .context("Solana RPC returned non-success status")?
        .json()
        .await
        .context("Solana RPC response is not valid JSON")?;

    if let Some(err) = parsed.get("error").filter(|v| !v.is_null()) {
        bail!("Solana RPC error from {method}: {err}");
    }
    Ok(parsed
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}
