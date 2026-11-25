//! Live integration test against a local Bitcoin Core regtest node.
//!
//! Gated with `#[ignore]` because it requires `bitcoind` and `bitcoin-cli` on
//! PATH with a regtest node accepting RPC on the default port (18443). Run
//! explicitly with:
//!
//! ```text
//! cargo test -p arrow-origin-btc --test live_regtest -- --ignored
//! ```
//!
//! The test mines enough blocks to fund the coinbase, sends a real value
//! transfer, observes it through [`BtcObserver`], and asserts:
//! - fewer than 6 confirmations → `1:BTC:PD:HL:0:B`
//! - after mining to the 6-confirmation depth → `1:BTC:FN:PR:1:A`
//!
//! Set `BTC_REGTEST_RPC_USER` / `BTC_REGTEST_RPC_PASS` env vars to override the
//! default `rpcuser1` / `rpcpassword1` test credentials.

use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use arrow_core::registry::loader::Registries;
use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_origin_btc::{mapping::map_to_arrow, observer::BtcObserver};

const DEFAULT_RPC_HOST: &str = "http://127.0.0.1:18443";
const DEFAULT_RPC_USER: &str = "rpcuser1";
const DEFAULT_RPC_PASS: &str = "rpcpassword1";

fn rpc_user() -> String {
    std::env::var("BTC_REGTEST_RPC_USER").unwrap_or_else(|_| DEFAULT_RPC_USER.to_string())
}
fn rpc_pass() -> String {
    std::env::var("BTC_REGTEST_RPC_PASS").unwrap_or_else(|_| DEFAULT_RPC_PASS.to_string())
}

fn tool_available(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run `bitcoin-cli -regtest` with the test credentials; return trimmed stdout.
/// Returns `None` if the command fails (node not available).
fn btc_cli_try(args: &[&str]) -> Option<String> {
    let user = rpc_user();
    let pass = rpc_pass();
    let out = Command::new("bitcoin-cli")
        .args([
            "-regtest",
            &format!("-rpcuser={user}"),
            &format!("-rpcpassword={pass}"),
        ])
        .args(args)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn btc_cli(args: &[&str]) -> String {
    let user = rpc_user();
    let pass = rpc_pass();
    let out = Command::new("bitcoin-cli")
        .args([
            "-regtest",
            &format!("-rpcuser={user}"),
            &format!("-rpcpassword={pass}"),
        ])
        .args(args)
        .output()
        .expect("running bitcoin-cli");
    assert!(
        out.status.success(),
        "bitcoin-cli {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
#[ignore = "requires bitcoin-cli + regtest node on 127.0.0.1:18443"]
fn observe_regtest_tx_to_valid_arrow_event() {
    if !tool_available("bitcoin-cli") {
        eprintln!("skipping: bitcoin-cli not found on PATH");
        return;
    }
    if btc_cli_try(&["getblockchaininfo"]).is_none() {
        eprintln!("skipping: no bitcoin regtest node on {DEFAULT_RPC_HOST}");
        return;
    }

    // Ensure a default wallet exists (create if absent — regtest convenience).
    let wallets: serde_json::Value =
        serde_json::from_str(&btc_cli(&["listwallets"])).expect("listwallets JSON");
    let has_default = wallets
        .as_array()
        .map(|v| {
            v.iter()
                .any(|w| w.as_str() == Some("") || w.as_str() == Some("default"))
        })
        .unwrap_or(false);
    if !has_default {
        btc_cli(&["createwallet", "default"]);
    }

    // Mine 101 blocks so the coinbase reward is spendable.
    let addr = btc_cli(&["-rpcwallet=default", "getnewaddress"]);
    let mined: serde_json::Value = serde_json::from_str(&btc_cli(&[
        "-rpcwallet=default",
        "generatetoaddress",
        "101",
        &addr,
    ]))
    .expect("generatetoaddress JSON");
    assert!(
        mined.as_array().map(|v| v.len() >= 101).unwrap_or(false),
        "expected ≥ 101 mined blocks"
    );

    // Send 0.001 BTC to a fresh address.
    let recipient = btc_cli(&["-rpcwallet=default", "getnewaddress"]);
    let txid = btc_cli(&["-rpcwallet=default", "sendtoaddress", &recipient, "0.001"]);
    let txid = txid.trim().to_string();

    // Mine one block to confirm the tx.
    btc_cli(&["-rpcwallet=default", "generatetoaddress", "1", &addr]);

    // Small delay to ensure the block is indexed.
    sleep(Duration::from_millis(200));

    let observer = BtcObserver::new(DEFAULT_RPC_HOST, rpc_user(), rpc_pass());
    let registries = Registries::load_embedded().expect("embedded registries");
    let pipeline = ValidationPipeline::new(&registries);

    // Decode the tx to find the vout for the recipient.
    let raw_json: serde_json::Value =
        serde_json::from_str(&btc_cli(&["getrawtransaction", &txid, "1"]))
            .expect("getrawtransaction JSON");
    let vout_idx: u32 = raw_json["vout"]
        .as_array()
        .expect("vout array")
        .iter()
        .position(|o| o["scriptPubKey"]["address"].as_str() == Some(&recipient))
        .expect("recipient output not found") as u32;

    // 1 confirmation → pending.
    let obs = observer
        .observe(&txid, vout_idx)
        .expect("observe after 1 block");
    assert_eq!(obs.confirmations, 1, "expected 1 confirmation");
    let pending = map_to_arrow(&obs);
    assert_eq!(pending.arrow_code, "1:BTC:PD:HL:0:B");
    assert!(
        pipeline.validate_event(&pending).passed,
        "pending event must validate"
    );

    // Mine 5 more blocks → 6 confirmations → final.
    btc_cli(&["-rpcwallet=default", "generatetoaddress", "5", &addr]);
    sleep(Duration::from_millis(200));

    let obs = observer
        .observe(&txid, vout_idx)
        .expect("re-observe at 6 conf");
    assert!(
        obs.confirmations >= 6,
        "expected >= 6 confirmations, got {}",
        obs.confirmations
    );
    let final_ev = map_to_arrow(&obs);
    assert_eq!(final_ev.arrow_code, "1:BTC:FN:PR:1:A");
    assert!(
        pipeline.validate_event(&final_ev).passed,
        "final event must validate"
    );

    // Assert observed_network is "regtest".
    assert_eq!(
        obs.observed_network.as_deref(),
        Some("regtest"),
        "regtest node must report observed_network='regtest'"
    );
}
