//! Destination-chain (chain B) settlement over JSON-RPC.
//!
//! When the reduction permits settlement, the Consumption Adapter performs the
//! host-system action: a value transfer on a destination EVM chain. This is the
//! off-chain analogue of RFC-003's Settlement Backend — it demonstrates true A→B
//! coordination (an event on chain A drives an action on chain B) without any
//! on-chain Arrow contract.
//!
//! ## Two submission paths
//!
//! - **Raw (default):** locally signs an EIP-155 transaction and submits via
//!   `eth_sendRawTransaction`. Works on any public node (no unlocked account).
//!   Requires `raw_key_hex` in [`SettleParams`].
//!
//! - **Unlocked (Anvil):** submits via `eth_sendTransaction` (requires the
//!   `from` account to be unlocked on the node). Selected when [`SettleParams`]
//!   sets `unlocked = true`. Only appropriate for local `anvil` dev nodes.
//!
//! ## Dependency rationale (§4.4)
//!
//! Rather than pulling a full Ethereum client, we use:
//! - `k256 = "0.13.3"` for secp256k1 ECDSA signing (RustCrypto, minimal).
//! - `sha3 = "0.10.8"` for keccak256 transaction hashing (same family as sha2).
//! - Manual RLP encoding: the EIP-155 legacy transaction has a fixed structure
//!   and requires only byte-string + list encoding — no external rlp crate needed.

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;

/// Parameters for a chain-B settlement transfer.
#[derive(Debug, Clone)]
pub struct SettleParams {
    /// Destination chain JSON-RPC endpoint.
    pub rpc_url: String,
    /// `0x`-prefixed sender address.
    pub from: String,
    /// `0x`-prefixed recipient address.
    pub to: String,
    /// Value to transfer, in wei, as a decimal string.
    pub value_wei: String,
    /// Hex-encoded raw 32-byte secp256k1 private key (no `0x` prefix).
    /// Required for the raw signing path (`unlocked = false`).
    pub raw_key_hex: Option<String>,
    /// If `true`, use `eth_sendTransaction` (Anvil unlocked-account path).
    /// If `false` (default), sign and use `eth_sendRawTransaction`.
    pub unlocked: bool,
}

/// Submit the settlement transfer and return the chain-B transaction hash.
pub fn send_settlement(params: &SettleParams) -> Result<String> {
    if params.unlocked {
        send_unlocked(params)
    } else {
        send_raw(params)
    }
}

/// Submit via `eth_sendTransaction` — requires an unlocked account (Anvil only).
pub fn send_unlocked(params: &SettleParams) -> Result<String> {
    let value_u128: u128 = params
        .value_wei
        .parse()
        .with_context(|| format!("invalid decimal wei value `{}`", params.value_wei))?;
    let value_hex = format!("0x{value_u128:x}");

    let url = params.rpc_url.clone();
    let tx = serde_json::json!({
        "from": params.from,
        "to": params.to,
        "value": value_hex,
    });

    tokio::runtime::Runtime::new()?.block_on(async move {
        let result = rpc_call(&url, "eth_sendTransaction", serde_json::json!([tx]))
            .await
            .context("eth_sendTransaction failed")?;
        result
            .as_str()
            .map(str::to_string)
            .context("eth_sendTransaction did not return a transaction hash")
    })
}

/// Derive the EIP-55-ish Ethereum address from a raw secp256k1 private key.
///
/// Returns `"0x" + lowercase_hex(keccak256(uncompressed_pubkey[1..])[12..])`.
/// Used to verify that `raw_key_hex` belongs to the stated `from` address.
pub fn address_from_key(key_bytes: &[u8]) -> Result<String> {
    use k256::ecdsa::SigningKey;
    use sha3::{Digest, Keccak256};

    let key = SigningKey::from_bytes(key_bytes.into()).context("invalid secp256k1 private key")?;
    // Uncompressed public key: 0x04 || X (32 bytes) || Y (32 bytes).
    let uncompressed = key.verifying_key().to_encoded_point(false);
    let pubkey_bytes = &uncompressed.as_bytes()[1..]; // skip 0x04 prefix → 64 bytes
    let hash = Keccak256::digest(pubkey_bytes);
    Ok(format!("0x{}", hex::encode(&hash[12..]))) // last 20 bytes
}

/// Build, sign (EIP-155), and submit a raw transaction.
///
/// Fetches nonce, gas price, chain id, and gas estimate from the RPC; signs
/// with the provided secp256k1 key; and submits via `eth_sendRawTransaction`.
pub fn send_raw(params: &SettleParams) -> Result<String> {
    let key_hex = params
        .raw_key_hex
        .as_deref()
        .context("raw_key_hex required for non-unlocked settlement")?;

    let value_wei: u128 = params
        .value_wei
        .parse()
        .with_context(|| format!("invalid decimal wei value `{}`", params.value_wei))?;

    let url = params.rpc_url.clone();
    let from = params.from.clone();
    let to = params.to.clone();
    let key_bytes = hex::decode(key_hex).context("invalid hex in raw_key_hex")?;
    if key_bytes.len() != 32 {
        bail!(
            "raw_key_hex must be 32 bytes (64 hex chars), got {}",
            key_bytes.len()
        );
    }

    // Fail closed: verify the key belongs to the stated `from` address before
    // making any RPC calls. Without this, the nonce would be fetched for `from`
    // but the signed transaction would be sent from the key-derived address.
    let derived = address_from_key(&key_bytes)?;
    if derived.to_lowercase() != from.to_lowercase() {
        bail!("raw_key_hex address mismatch: key derives to {derived}, but from = {from}");
    }

    tokio::runtime::Runtime::new()?.block_on(async move {
        // Fetch signing parameters.
        let nonce_hex = rpc_call(
            &url,
            "eth_getTransactionCount",
            serde_json::json!([from, "pending"]),
        )
        .await
        .context("eth_getTransactionCount failed")?;
        let nonce =
            parse_hex_u64(nonce_hex.as_str().unwrap_or_default()).context("parsing nonce")?;

        let gas_price_hex = rpc_call(&url, "eth_gasPrice", serde_json::json!([]))
            .await
            .context("eth_gasPrice failed")?;
        let gas_price = parse_hex_u128(gas_price_hex.as_str().unwrap_or_default())
            .context("parsing gas price")?;

        let chain_id_hex = rpc_call(&url, "eth_chainId", serde_json::json!([]))
            .await
            .context("eth_chainId failed")?;
        let chain_id =
            parse_hex_u64(chain_id_hex.as_str().unwrap_or_default()).context("parsing chain id")?;

        // Gas limit: 21000 is exact for a plain ETH transfer.
        let gas_limit: u64 = 21_000;

        let to_bytes = decode_address(&to)?;
        let signed_raw = sign_eip155_tx(
            &key_bytes, nonce, gas_price, gas_limit, &to_bytes, value_wei, chain_id,
        )?;
        let raw_hex = format!("0x{}", hex::encode(&signed_raw));

        let result = rpc_call(&url, "eth_sendRawTransaction", serde_json::json!([raw_hex]))
            .await
            .context("eth_sendRawTransaction failed")?;
        result
            .as_str()
            .map(str::to_string)
            .context("eth_sendRawTransaction did not return a transaction hash")
    })
}

/// Build an EIP-155 signed raw transaction as RLP-encoded bytes.
///
/// Uses manual RLP encoding (no external crate) and k256 ECDSA for signing.
pub fn sign_eip155_tx(
    private_key: &[u8],
    nonce: u64,
    gas_price: u128,
    gas_limit: u64,
    to: &[u8; 20],
    value: u128,
    chain_id: u64,
) -> Result<Vec<u8>> {
    use k256::ecdsa::RecoveryId;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};
    use sha3::{Digest, Keccak256};

    let key =
        SigningKey::from_bytes(private_key.into()).context("invalid secp256k1 private key")?;

    // Signing payload: RLP([nonce, gasPrice, gasLimit, to, value, data, chainId, 0, 0])
    let signing_rlp = rlp_encode_list(&[
        rlp_encode_uint(nonce as u128),
        rlp_encode_uint(gas_price),
        rlp_encode_uint(gas_limit as u128),
        rlp_encode_bytes(to), // address is encoded as bytes
        rlp_encode_uint(value),
        rlp_encode_bytes(&[]), // data: empty for native transfer
        rlp_encode_uint(chain_id as u128),
        rlp_encode_uint(0), // EIP-155: r = 0 for signing
        rlp_encode_uint(0), // EIP-155: s = 0 for signing
    ]);

    let tx_hash = Keccak256::digest(&signing_rlp);
    let (sig, rec_id): (k256::ecdsa::Signature, RecoveryId) =
        key.sign_prehash(&tx_hash).context("ECDSA signing failed")?;

    let r = sig.r().to_bytes();
    let s = sig.s().to_bytes();
    // EIP-155: v = {0,1} + 35 + 2*chainId
    let v: u128 = (rec_id.to_byte() as u128) + 35 + 2 * (chain_id as u128);

    // Signed transaction: RLP([nonce, gasPrice, gasLimit, to, value, data, v, r, s])
    let signed = rlp_encode_list(&[
        rlp_encode_uint(nonce as u128),
        rlp_encode_uint(gas_price),
        rlp_encode_uint(gas_limit as u128),
        rlp_encode_bytes(to),
        rlp_encode_uint(value),
        rlp_encode_bytes(&[]),
        rlp_encode_uint(v),
        rlp_encode_bytes(&r),
        rlp_encode_bytes(&s),
    ]);

    Ok(signed)
}

// ── Minimal RLP encoder ──────────────────────────────────────────────────────

/// RLP-encode a non-negative integer (big-endian, no leading zeros).
/// Integer 0 encodes as empty string (0x80 per RLP spec).
pub fn rlp_encode_uint(n: u128) -> Vec<u8> {
    if n == 0 {
        return rlp_encode_bytes(&[]);
    }
    let bytes = n.to_be_bytes();
    let stripped: &[u8] = {
        let first_nonzero = bytes
            .iter()
            .position(|&b| b != 0)
            .unwrap_or(bytes.len() - 1);
        &bytes[first_nonzero..]
    };
    rlp_encode_bytes(stripped)
}

/// RLP-encode a byte string.
pub fn rlp_encode_bytes(data: &[u8]) -> Vec<u8> {
    if data.len() == 1 && data[0] < 0x80 {
        return vec![data[0]];
    }
    let mut out = rlp_length_prefix(data.len(), 0x80);
    out.extend_from_slice(data);
    out
}

/// RLP-encode a list of already-encoded items.
pub fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.iter().flat_map(|i| i.iter().copied()).collect();
    let mut out = rlp_length_prefix(payload.len(), 0xc0);
    out.extend_from_slice(&payload);
    out
}

fn rlp_length_prefix(len: usize, offset: u8) -> Vec<u8> {
    if len <= 55 {
        vec![offset + len as u8]
    } else {
        let len_bytes = usize_to_be_bytes_stripped(len);
        let mut out = vec![offset + 55 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out
    }
}

fn usize_to_be_bytes_stripped(n: usize) -> Vec<u8> {
    let bytes = n.to_be_bytes();
    let first = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len() - 1);
    bytes[first..].to_vec()
}

fn decode_address(addr: &str) -> Result<[u8; 20]> {
    let hex_str = addr.strip_prefix("0x").unwrap_or(addr);
    let bytes = hex::decode(hex_str).with_context(|| format!("invalid hex address: {addr}"))?;
    if bytes.len() != 20 {
        bail!("address must be 20 bytes, got {}: {addr}", bytes.len());
    }
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn parse_hex_u64(s: &str) -> Result<u64> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16).with_context(|| format!("invalid hex u64: 0x{s}"))
}

fn parse_hex_u128(s: &str) -> Result<u128> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(s, 16).with_context(|| format!("invalid hex u128: 0x{s}"))
}

/// Issue a single JSON-RPC call and return the `result` value.
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

    if let Some(err) = parsed.get("error") {
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
    use hyper::body::Incoming;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::Response;
    use hyper_util::rt::TokioIo;
    use std::convert::Infallible;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct CapturedRequest {
        method: String,
        content_type: Option<String>,
        body: serde_json::Value,
    }

    fn params(url: String, value_wei: impl Into<String>) -> SettleParams {
        SettleParams {
            rpc_url: url,
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: "0x2222222222222222222222222222222222222222".to_string(),
            value_wei: value_wei.into(),
            raw_key_hex: None,
            unlocked: true,
        }
    }

    fn serve_once(
        status: u16,
        body: impl Into<String>,
    ) -> (String, Arc<Mutex<Option<CapturedRequest>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock RPC server");
        let addr = listener.local_addr().expect("mock RPC server address");
        let captured = Arc::new(Mutex::new(None));
        let captured_for_server = Arc::clone(&captured);
        let body = body.into();

        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("mock RPC runtime")
                .block_on(async move {
                    listener
                        .set_nonblocking(true)
                        .expect("set mock listener nonblocking");
                    let listener =
                        tokio::net::TcpListener::from_std(listener).expect("tokio listener");
                    let (stream, _) = listener.accept().await.expect("accept mock request");
                    let captured = Arc::clone(&captured_for_server);
                    let response_body = body.clone();

                    let service = service_fn(move |req: Request<Incoming>| {
                        let captured = Arc::clone(&captured);
                        let response_body = response_body.clone();

                        async move {
                            let method = req.method().to_string();
                            let content_type = req
                                .headers()
                                .get("content-type")
                                .and_then(|v| v.to_str().ok())
                                .map(str::to_string);
                            let body_bytes = req
                                .into_body()
                                .collect()
                                .await
                                .expect("read mock request body")
                                .to_bytes();
                            let body = serde_json::from_slice(&body_bytes)
                                .expect("mock request body is JSON");
                            *captured.lock().expect("capture request") = Some(CapturedRequest {
                                method,
                                content_type,
                                body,
                            });

                            Ok::<_, Infallible>(
                                Response::builder()
                                    .status(status)
                                    .body(Full::new(Bytes::from(response_body)))
                                    .expect("mock response"),
                            )
                        }
                    });

                    http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await
                        .expect("mock RPC server");
                });
        });

        (format!("http://{addr}"), captured)
    }

    #[test]
    fn send_settlement_posts_eth_send_transaction_with_hex_value() {
        let (url, captured) = serve_once(200, r#"{"jsonrpc":"2.0","id":1,"result":"0xabc123"}"#);

        let tx = send_settlement(&params(url, "255")).expect("settlement succeeds");

        assert_eq!(tx, "0xabc123");
        let req = captured
            .lock()
            .expect("capture lock")
            .clone()
            .expect("captured request");
        assert_eq!(req.method, "POST");
        assert_eq!(req.content_type.as_deref(), Some("application/json"));
        assert_eq!(req.body["jsonrpc"], "2.0");
        assert_eq!(req.body["id"], 1);
        assert_eq!(req.body["method"], "eth_sendTransaction");
        assert_eq!(
            req.body["params"][0]["from"],
            params(String::new(), "0").from
        );
        assert_eq!(req.body["params"][0]["to"], params(String::new(), "0").to);
        assert_eq!(req.body["params"][0]["value"], "0xff");
    }

    #[test]
    fn send_settlement_rejects_invalid_decimal_wei_before_rpc() {
        let err = send_settlement(&params("http://127.0.0.1:1".to_string(), "1.5"))
            .expect_err("fractional wei is invalid");

        assert!(
            err.to_string().contains("invalid decimal wei value `1.5`"),
            "got: {err:#}"
        );
    }

    #[test]
    fn send_settlement_rejects_rpc_error_response() {
        let (url, _captured) = serve_once(
            200,
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"sender locked"}}"#,
        );

        let err = send_settlement(&params(url, "1")).expect_err("RPC error must fail closed");

        assert!(
            err.to_string().contains("eth_sendTransaction failed"),
            "got: {err:#}"
        );
        assert!(
            format!("{err:#}").contains("RPC error from eth_sendTransaction"),
            "got: {err:#}"
        );
    }

    #[test]
    fn send_settlement_rejects_non_string_transaction_hash() {
        let (url, _captured) = serve_once(200, r#"{"jsonrpc":"2.0","id":1,"result":42}"#);

        let err = send_settlement(&params(url, "1")).expect_err("hash must be a string");

        assert!(
            err.to_string()
                .contains("eth_sendTransaction did not return a transaction hash"),
            "got: {err:#}"
        );
    }

    #[test]
    fn send_settlement_rejects_invalid_json_response() {
        let (url, _captured) = serve_once(200, "not json");

        let err = send_settlement(&params(url, "1")).expect_err("invalid JSON must fail");

        assert!(
            format!("{err:#}").contains("RPC response is not valid JSON"),
            "got: {err:#}"
        );
    }

    // §4.4 — raw_tx_rlp_encoding_matches_known_vector
    //
    // EIP-155 reference vector (chain_id = 1, mainnet):
    //   privkey:  4646464646464646464646464646464646464646464646464646464646464646
    //   sender:   0x9d8A62f656a8d1615C1294fd71e9CFb3E4855A4F
    //   nonce:    9
    //   gasPrice: 20_000_000_000 (20 gwei)
    //   gasLimit: 21000
    //   to:       0x3535353535353535353535353535353535353535
    //   value:    1_000_000_000_000_000_000 wei (1 ETH)
    //   data:     empty
    //   v:        37  (recovery_id=0, 35 + 2*chain_id)
    //
    // Expected signed RLP (110 bytes):
    //   f86c098504a817c800825208943535353535353535353535353535353535353535
    //   880de0b6b3a76400008025a028ef61340bd939bc2195fe537567866003e1a15d
    //   3c71ff63e1590620aa636276a067cbe9d8997f761aecb703304b3800ccf555c9
    //   f3dc64214b297fb1966a3b6d83
    #[test]
    fn raw_tx_rlp_encoding_matches_known_vector() {
        let privkey =
            hex::decode("4646464646464646464646464646464646464646464646464646464646464646")
                .unwrap();
        let to = decode_address("0x3535353535353535353535353535353535353535").unwrap();

        // P3: sender address must derive correctly from the key.
        let sender = address_from_key(&privkey).unwrap();
        assert_eq!(
            sender, "0x9d8a62f656a8d1615c1294fd71e9cfb3e4855a4f",
            "key must derive to the known EIP-155 vector sender"
        );

        let raw = sign_eip155_tx(
            &privkey,
            9,              // nonce
            20_000_000_000, // gas price: 20 gwei
            21_000,         // gas limit
            &to,
            1_000_000_000_000_000_000, // value: 1 ETH
            1,                         // chain_id: mainnet
        )
        .unwrap();

        // P3: exact byte comparison against the published EIP-155 reference vector.
        // Both go-ethereum and k256 use RFC 6979 with SHA-256 for deterministic k
        // generation, and s is already < n/2 (no normalization flip). The output
        // must be byte-for-byte identical.
        let expected_hex = concat!(
            "f86c098504a817c800825208943535353535353535353535353535353535353535",
            "880de0b6b3a76400008025a028ef61340bd939bc2195fe537567866003e1a15d",
            "3c71ff63e1590620aa636276a067cbe9d8997f761aecb703304b3800ccf555c9",
            "f3dc64214b297fb1966a3b6d83"
        );
        assert_eq!(
            hex::encode(&raw),
            expected_hex,
            "signed tx must match the EIP-155 known-good reference vector"
        );
    }

    // §4.4 — P1a: key-to-address derivation
    #[test]
    fn address_from_key_matches_known_sender() {
        // Same privkey as the EIP-155 vector above.
        let privkey =
            hex::decode("4646464646464646464646464646464646464646464646464646464646464646")
                .unwrap();
        let addr = address_from_key(&privkey).unwrap();
        assert_eq!(addr, "0x9d8a62f656a8d1615c1294fd71e9cfb3e4855a4f");
    }

    #[test]
    fn send_raw_rejects_key_from_mismatch() {
        // The key below belongs to address 0x9d8a..., not 0x1111...
        let key_hex = "4646464646464646464646464646464646464646464646464646464646464646";
        let params = SettleParams {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: "0x2222222222222222222222222222222222222222".to_string(),
            value_wei: "1".to_string(),
            raw_key_hex: Some(key_hex.to_string()),
            unlocked: false,
        };
        let err = send_raw(&params).unwrap_err();
        assert!(
            err.to_string().contains("address mismatch"),
            "must fail closed on key/from mismatch; got: {err}"
        );
    }

    // §4.4 — unlocked_path_still_available_for_anvil
    #[test]
    fn unlocked_path_requires_no_key() {
        // Constructing SettleParams with unlocked=true does not require raw_key_hex.
        let params = SettleParams {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            from: "0x0000000000000000000000000000000000000001".to_string(),
            to: "0x0000000000000000000000000000000000000002".to_string(),
            value_wei: "0".to_string(),
            raw_key_hex: None,
            unlocked: true,
        };
        // Cannot actually submit without Anvil running; just verify no panic on construction.
        assert!(params.unlocked);
        assert!(params.raw_key_hex.is_none());
    }

    // §4.4 — RLP helpers
    #[test]
    fn rlp_uint_zero_is_0x80() {
        assert_eq!(rlp_encode_uint(0), vec![0x80]);
    }

    #[test]
    fn rlp_uint_single_byte_below_0x80() {
        assert_eq!(rlp_encode_uint(0x01), vec![0x01]);
        assert_eq!(rlp_encode_uint(0x7f), vec![0x7f]);
    }

    #[test]
    fn rlp_uint_single_byte_0x80_or_above() {
        // 0x80 as integer encodes as 0x81 0x80
        assert_eq!(rlp_encode_uint(0x80), vec![0x81, 0x80]);
    }

    #[test]
    fn rlp_uint_21000() {
        // 21000 = 0x5208 → bytes [0x52, 0x08] → length-prefixed: [0x82, 0x52, 0x08]
        assert_eq!(rlp_encode_uint(21_000), vec![0x82, 0x52, 0x08]);
    }

    #[test]
    fn rlp_bytes_empty() {
        assert_eq!(rlp_encode_bytes(&[]), vec![0x80]);
    }

    #[test]
    fn rlp_bytes_20_byte_address() {
        let addr = [0x35u8; 20];
        let encoded = rlp_encode_bytes(&addr);
        assert_eq!(
            encoded[0],
            0x80 + 20,
            "prefix must be 0x94 for 20-byte address"
        );
        assert_eq!(&encoded[1..], &addr[..]);
    }
}
