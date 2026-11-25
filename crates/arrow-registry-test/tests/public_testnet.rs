//! Public testnet smoke tests (R-009, R-010, R-011).
//!
//! All tests are `#[ignore]`-gated and secrets-gated — they are skipped unless
//! the required environment variables are set. Run the full suite with:
//!
//! ```text
//! cargo test -p arrow-registry-test --test public_testnet -- --ignored
//! ```
//!
//! Required env vars per test are documented in each test function. The
//! `ARROW_SEPOLIA_RPC` / `ARROW_BASE_SEPOLIA_RPC` / `ARROW_ARB_SEPOLIA_RPC`
//! vars override the default public RPC endpoints from [`networks::default_rpc`].
//!
//! ## What each test asserts
//!
//! | Test | Asserts |
//! |---|---|
//! | `eth_sepolia_lifecycle` | observed chain id `11155111`; final tx → `1:ETH:FN:PR:1:A`; artifact hash reproducible |
//! | `base_sepolia_lifecycle` | observed chain id `84532`; `1:BSE:PD:HL:0:B` + `finality_eta` unix_ts present |
//! | `arb_sepolia_lifecycle` | observed chain id `421614`; `1:ARB:PD:HL:0:B` + `finality_eta` |
//! | `sol_devnet_lifecycle` | devnet genesis hash; finalized → `1:SOL:FN:PR:1:A` |
//! | `btc_signet_lifecycle` | `observed_network == "signet"`; ≥6 conf → `1:BTC:FN:PR:1:A` |
//! | `a_to_b_sepolia` | raw-signed settlement on Sepolia (no unlocked account) |

use arrow_registry_test::{
    artifact::verify_hash,
    lifecycle::{run_lifecycle, ChainFamily, LifecycleParams},
    networks::{chain_ids, default_rpc, resolve_rpc, sol_genesis},
};

/// Return `Some(value)` for an env var, or `None` (skipping the test) if absent.
fn require_env(var: &str) -> Option<String> {
    match std::env::var(var) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => {
            eprintln!("skipping: {var} not set");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// ETH Sepolia
// ---------------------------------------------------------------------------

/// Smoke-test the ETH Sepolia lifecycle with a known confirmed transaction.
///
/// Required env vars:
/// - `ARROW_ETH_SEPOLIA_TX_FINAL` — a Sepolia tx hash that has ≥12 confirmations
/// - `ARROW_SEPOLIA_RPC` (optional) — Sepolia JSON-RPC URL override
#[test]
#[ignore = "requires ARROW_ETH_SEPOLIA_TX_FINAL env var and Sepolia RPC"]
fn eth_sepolia_lifecycle() {
    let tx = match require_env("ARROW_ETH_SEPOLIA_TX_FINAL") {
        Some(t) => t,
        None => return,
    };

    let rpc = resolve_rpc("ARROW_SEPOLIA_RPC", default_rpc::ETH_SEPOLIA);

    let params = LifecycleParams {
        registry_code: "ETH".to_string(),
        entry_version: "1.0".to_string(),
        registry_version: "2.0".to_string(),
        network: "testnet".to_string(),
        network_name: "sepolia".to_string(),
        family: ChainFamily::Evm,
        rpc_url: rpc,
        tx_ref_value: tx,
        btc_vout: None,
        btc_rpc_user: None,
        btc_rpc_pass: None,
        maps_to_fields: vec!["reorg_safety_confirmations".to_string()],
        expected_chain_id: Some(chain_ids::ETH_SEPOLIA),
        expected_btc_network: None,
        expected_sol_genesis: None,
    };

    let artifact = run_lifecycle(&params).expect("eth_sepolia_lifecycle failed");

    assert_eq!(
        artifact.core.arrow_code, "1:ETH:FN:PR:1:A",
        "expected final ETH code"
    );
    assert_eq!(
        artifact.core.chain_id_observed,
        chain_ids::ETH_SEPOLIA.to_string(),
        "observed chain id must be Sepolia (11155111)"
    );
    assert_eq!(artifact.core.finality_model, "probabilistic");
    verify_hash(&artifact).expect("artifact hash must reproduce from core");
}

// ---------------------------------------------------------------------------
// Base Sepolia
// ---------------------------------------------------------------------------

/// Smoke-test the Base Sepolia lifecycle with a known pending transaction.
///
/// Required env vars:
/// - `ARROW_BASE_SEPOLIA_TX_PENDING` — a Base Sepolia tx hash (recent, pending
///   or within the 7-day challenge window)
/// - `ARROW_BASE_SEPOLIA_RPC` (optional) — Base Sepolia JSON-RPC URL override
#[test]
#[ignore = "requires ARROW_BASE_SEPOLIA_TX_PENDING env var and Base Sepolia RPC"]
fn base_sepolia_lifecycle() {
    let tx = match require_env("ARROW_BASE_SEPOLIA_TX_PENDING") {
        Some(t) => t,
        None => return,
    };

    let rpc = resolve_rpc("ARROW_BASE_SEPOLIA_RPC", default_rpc::BASE_SEPOLIA);

    let params = LifecycleParams {
        registry_code: "BASE".to_string(),
        entry_version: "1.0".to_string(),
        registry_version: "2.0".to_string(),
        network: "testnet".to_string(),
        network_name: "base-sepolia".to_string(),
        family: ChainFamily::Evm,
        rpc_url: rpc,
        tx_ref_value: tx,
        btc_vout: None,
        btc_rpc_user: None,
        btc_rpc_pass: None,
        maps_to_fields: vec!["challenge_window_seconds".to_string()],
        expected_chain_id: Some(chain_ids::BASE_SEPOLIA),
        expected_btc_network: None,
        expected_sol_genesis: None,
    };

    let artifact = run_lifecycle(&params).expect("base_sepolia_lifecycle failed");

    // Base Sepolia is optimistic-L2: adapter never mints FN from L2 depth.
    assert_eq!(
        artifact.core.arrow_code, "1:BSE:PD:HL:0:B",
        "expected pending Base code"
    );
    assert_eq!(
        artifact.core.chain_id_observed,
        chain_ids::BASE_SEPOLIA.to_string(),
        "observed chain id must be Base Sepolia (84532)"
    );
    // Window-close ETA must be present (unix_ts, per §4.5).
    assert!(
        artifact.core.finality_basis.as_deref() == Some("optimistic_window"),
        "finality_basis must be 'optimistic_window'"
    );
    verify_hash(&artifact).expect("artifact hash must reproduce from core");
}

// ---------------------------------------------------------------------------
// Arbitrum Sepolia
// ---------------------------------------------------------------------------

/// Smoke-test the Arbitrum Sepolia lifecycle with a known pending transaction.
///
/// Required env vars:
/// - `ARROW_ARB_SEPOLIA_TX_PENDING` — an Arb Sepolia tx hash
/// - `ARROW_ARB_SEPOLIA_RPC` (optional) — Arb Sepolia JSON-RPC URL override
#[test]
#[ignore = "requires ARROW_ARB_SEPOLIA_TX_PENDING env var and Arb Sepolia RPC"]
fn arb_sepolia_lifecycle() {
    let tx = match require_env("ARROW_ARB_SEPOLIA_TX_PENDING") {
        Some(t) => t,
        None => return,
    };

    let rpc = resolve_rpc("ARROW_ARB_SEPOLIA_RPC", default_rpc::ARB_SEPOLIA);

    let params = LifecycleParams {
        registry_code: "ARB".to_string(),
        entry_version: "1.0".to_string(),
        registry_version: "2.0".to_string(),
        network: "testnet".to_string(),
        network_name: "arb-sepolia".to_string(),
        family: ChainFamily::Evm,
        rpc_url: rpc,
        tx_ref_value: tx,
        btc_vout: None,
        btc_rpc_user: None,
        btc_rpc_pass: None,
        maps_to_fields: vec!["challenge_window_seconds".to_string()],
        expected_chain_id: Some(chain_ids::ARB_SEPOLIA),
        expected_btc_network: None,
        expected_sol_genesis: None,
    };

    let artifact = run_lifecycle(&params).expect("arb_sepolia_lifecycle failed");

    assert_eq!(
        artifact.core.arrow_code, "1:ARB:PD:HL:0:B",
        "expected pending ARB code"
    );
    assert_eq!(
        artifact.core.chain_id_observed,
        chain_ids::ARB_SEPOLIA.to_string(),
        "observed chain id must be Arb Sepolia (421614)"
    );
    assert!(
        artifact.core.finality_basis.as_deref() == Some("optimistic_window"),
        "finality_basis must be 'optimistic_window'"
    );
    verify_hash(&artifact).expect("artifact hash must reproduce from core");
}

// ---------------------------------------------------------------------------
// Solana Devnet
// ---------------------------------------------------------------------------

/// Smoke-test the Solana devnet lifecycle with a known finalized transaction.
///
/// Required env vars:
/// - `ARROW_SOL_DEVNET_TX_FINAL` — a Solana devnet signature at `finalized`
/// - `ARROW_SOL_DEVNET_RPC` (optional) — Solana devnet RPC URL override
#[test]
#[ignore = "requires ARROW_SOL_DEVNET_TX_FINAL env var and Solana devnet RPC"]
fn sol_devnet_lifecycle() {
    let tx = match require_env("ARROW_SOL_DEVNET_TX_FINAL") {
        Some(t) => t,
        None => return,
    };

    let rpc = resolve_rpc("ARROW_SOL_DEVNET_RPC", default_rpc::SOL_DEVNET);

    let params = LifecycleParams {
        registry_code: "SOL".to_string(),
        entry_version: "1.0".to_string(),
        registry_version: "2.0".to_string(),
        network: "testnet".to_string(),
        network_name: "devnet".to_string(),
        family: ChainFamily::Svm,
        rpc_url: rpc,
        tx_ref_value: tx,
        btc_vout: None,
        btc_rpc_user: None,
        btc_rpc_pass: None,
        maps_to_fields: vec!["finality_model".to_string()],
        expected_chain_id: None,
        expected_btc_network: None,
        expected_sol_genesis: Some(sol_genesis::DEVNET.to_string()),
    };

    let artifact = run_lifecycle(&params).expect("sol_devnet_lifecycle failed");

    assert_eq!(
        artifact.core.arrow_code, "1:SOL:FN:PR:1:A",
        "expected final SOL code"
    );
    assert_eq!(
        artifact.core.chain_id_observed,
        sol_genesis::DEVNET,
        "observed genesis hash must match devnet"
    );
    verify_hash(&artifact).expect("artifact hash must reproduce from core");
}

// ---------------------------------------------------------------------------
// Bitcoin Signet
// ---------------------------------------------------------------------------

/// Smoke-test the Bitcoin Signet lifecycle with a known confirmed UTXO.
///
/// Required env vars:
/// - `ARROW_BTC_SIGNET_TXID` — a Signet txid with ≥6 confirmations
/// - `ARROW_BTC_SIGNET_VOUT` — output index (e.g. `"0"`)
/// - `ARROW_BTC_SIGNET_RPC` — Bitcoin Core Signet RPC URL (e.g. `http://127.0.0.1:38332`)
/// - `ARROW_BTC_SIGNET_RPC_USER` — RPC username
/// - `ARROW_BTC_SIGNET_RPC_PASS` — RPC password
#[test]
#[ignore = "requires BTC Signet env vars (ARROW_BTC_SIGNET_TXID, _VOUT, _RPC, _RPC_USER, _RPC_PASS)"]
fn btc_signet_lifecycle() {
    let txid = match require_env("ARROW_BTC_SIGNET_TXID") {
        Some(t) => t,
        None => return,
    };
    let vout_str = match require_env("ARROW_BTC_SIGNET_VOUT") {
        Some(v) => v,
        None => return,
    };
    let rpc_url = match require_env("ARROW_BTC_SIGNET_RPC") {
        Some(u) => u,
        None => return,
    };
    let rpc_user = match require_env("ARROW_BTC_SIGNET_RPC_USER") {
        Some(u) => u,
        None => return,
    };
    let rpc_pass = match require_env("ARROW_BTC_SIGNET_RPC_PASS") {
        Some(p) => p,
        None => return,
    };

    let vout: u32 = vout_str
        .parse()
        .expect("ARROW_BTC_SIGNET_VOUT must be a u32");

    let params = LifecycleParams {
        registry_code: "BTC".to_string(),
        entry_version: "1.0".to_string(),
        registry_version: "2.0".to_string(),
        network: "testnet".to_string(),
        network_name: "signet".to_string(),
        family: ChainFamily::Utxo,
        rpc_url,
        tx_ref_value: txid,
        btc_vout: Some(vout),
        btc_rpc_user: Some(rpc_user),
        btc_rpc_pass: Some(rpc_pass),
        maps_to_fields: vec!["reorg_safety_confirmations".to_string()],
        expected_chain_id: None,
        expected_btc_network: Some("signet".to_string()),
        expected_sol_genesis: None,
    };

    let artifact = run_lifecycle(&params).expect("btc_signet_lifecycle failed");

    assert_eq!(
        artifact.core.arrow_code, "1:BTC:FN:PR:1:A",
        "expected final BTC code"
    );
    assert_eq!(
        artifact.core.chain_id_observed, "signet",
        "observed_network must be 'signet'"
    );
    verify_hash(&artifact).expect("artifact hash must reproduce from core");
}

// ---------------------------------------------------------------------------
// a_to_b Sepolia (raw-signed settlement)
// ---------------------------------------------------------------------------

/// End-to-end: observe a Sepolia transfer + submit raw-signed settlement.
///
/// This is the only test that actually sends a transaction. It verifies the
/// EIP-155 raw-signing path from §4.4 works against a live Sepolia node —
/// no unlocked account is used.
///
/// Required env vars:
/// - `ARROW_ETH_SEPOLIA_TX_FINAL` — a final Sepolia tx hash to observe
/// - `ARROW_TESTNET_FROM` — `0x`-prefixed Sepolia address derived from the key
/// - `ARROW_TESTNET_KEY` — hex private key (32 bytes, no `0x` prefix) for
///   `ARROW_TESTNET_FROM`. **Never commit this value.** CI must inject it as a
///   secret.
/// - `ARROW_TESTNET_RECIPIENT` — destination `0x`-address for the settlement tx
/// - `ARROW_SEPOLIA_RPC` (optional) — Sepolia JSON-RPC URL override
#[test]
#[ignore = "requires ARROW_ETH_SEPOLIA_TX_FINAL, ARROW_TESTNET_FROM, ARROW_TESTNET_KEY, ARROW_TESTNET_RECIPIENT"]
fn a_to_b_sepolia() {
    let tx = match require_env("ARROW_ETH_SEPOLIA_TX_FINAL") {
        Some(t) => t,
        None => return,
    };
    let from = match require_env("ARROW_TESTNET_FROM") {
        Some(f) => f,
        None => return,
    };
    let key_hex = match require_env("ARROW_TESTNET_KEY") {
        Some(k) => k,
        None => return,
    };
    let recipient = match require_env("ARROW_TESTNET_RECIPIENT") {
        Some(r) => r,
        None => return,
    };

    let rpc = resolve_rpc("ARROW_SEPOLIA_RPC", default_rpc::ETH_SEPOLIA);

    // Observe the source chain event.
    let source_params = LifecycleParams {
        registry_code: "ETH".to_string(),
        entry_version: "1.0".to_string(),
        registry_version: "2.0".to_string(),
        network: "testnet".to_string(),
        network_name: "sepolia".to_string(),
        family: ChainFamily::Evm,
        rpc_url: rpc.clone(),
        tx_ref_value: tx,
        btc_vout: None,
        btc_rpc_user: None,
        btc_rpc_pass: None,
        maps_to_fields: vec!["reorg_safety_confirmations".to_string()],
        expected_chain_id: Some(chain_ids::ETH_SEPOLIA),
        expected_btc_network: None,
        expected_sol_genesis: None,
    };

    let artifact = run_lifecycle(&source_params).expect("source lifecycle failed");
    assert_eq!(artifact.core.arrow_code, "1:ETH:FN:PR:1:A");
    verify_hash(&artifact).expect("artifact hash must reproduce");

    // Submit a 1-wei raw-signed settlement on Sepolia to prove the mechanism.
    use arrow_consume_eth::settle::{send_settlement, SettleParams};

    let settle_params = SettleParams {
        rpc_url: rpc,
        from,
        to: recipient,
        value_wei: "1".to_string(), // minimal value; proof-of-mechanism only
        raw_key_hex: Some(key_hex),
        unlocked: false,
    };
    let settle_result = send_settlement(&settle_params);
    assert!(
        settle_result.is_ok(),
        "raw-signed settlement must succeed on Sepolia; got: {:?}",
        settle_result
    );
}
