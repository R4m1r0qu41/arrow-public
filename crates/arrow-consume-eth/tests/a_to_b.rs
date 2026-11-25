//! End-to-end A→B coordination test across two local anvil chains.
//!
//! Gated with `#[ignore]` (requires Foundry's `anvil` + `cast`). Run with:
//!
//! ```text
//! cargo test -p arrow-consume-eth --test a_to_b -- --ignored
//! ```
//!
//! The full pipeline, with chain A and chain B as independent anvil nodes:
//!
//!   chain A tx  →  EthObserver  →  map_to_arrow  →  ValidationPipeline
//!               →  arrow_reduce::reduce  →  ApprovedForSettlement
//!               →  consume_and_settle  →  value transfer on chain B
//!
//! The test asserts the destination account's balance on chain B increased by the
//! settled amount — i.e. an event on chain A produced a real action on chain B.

use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use arrow_consume_eth::{verify_and_consume, OperatorConfig, ProvenancePolicy};
use arrow_core::registry::loader::Registries;
use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_identity::crypto::ed25519::generate_test_keypair;
use arrow_identity::resolver::did_key::did_key_from_public;
use arrow_origin_eth::emit::build_signed_envelope;
use arrow_origin_eth::mapping::{map_to_arrow, ETH_REORG_SAFETY_CONFIRMATIONS};
use arrow_origin_eth::observer::EthObserver;
use arrow_transport::replay::InMemoryReplayStore;
use base64::Engine;

/// Write an `arrow identity generate`-shaped dev key file; returns the signer DID.
fn write_dev_key(path: &std::path::Path) -> String {
    let (sk, pk) = generate_test_keypair();
    let did = did_key_from_public(&pk);
    let suffix = did.strip_prefix("did:key:").unwrap();
    let key_ref = format!("{did}#{suffix}");
    let sk_b64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(sk);
    let json = serde_json::json!({
        "did": did,
        "key_ref": key_ref,
        "algorithm": "Ed25519",
        "private_key_b64": sk_b64,
    });
    std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    did
}

/// Anvil's first two deterministic dev accounts (default mnemonic) and a key.
const ACCT0: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
const ACCT0_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const ACCT1: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";
const ONE_ETHER_WEI: &str = "1000000000000000000";

struct AnvilGuard(Child);
impl Drop for AnvilGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
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

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn cast(args: &[&str]) -> String {
    let out = Command::new("cast")
        .args(args)
        .output()
        .expect("running cast");
    assert!(
        out.status.success(),
        "cast {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Spawn an anvil node on `port` and wait until its RPC answers.
fn spawn_anvil(port: u16) -> AnvilGuard {
    let child = Command::new("anvil")
        .args(["--port", &port.to_string(), "--silent"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning anvil");
    // Own the child immediately so it is always wait()ed on drop — including the
    // not-ready panic path below.
    let guard = AnvilGuard(child);
    let url = format!("http://127.0.0.1:{port}");
    for _ in 0..50 {
        let ok = Command::new("cast")
            .args(["block-number", "--rpc-url", &url])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return guard;
        }
        sleep(Duration::from_millis(100));
    }
    panic!("anvil did not become ready on {url}");
}

#[test]
#[ignore = "requires foundry (anvil + cast) on PATH"]
fn event_on_chain_a_settles_on_chain_b() {
    if !tool_available("anvil") || !tool_available("cast") {
        eprintln!("skipping: anvil/cast not found on PATH");
        return;
    }

    let port_a = free_port();
    let port_b = free_port();
    let url_a = format!("http://127.0.0.1:{port_a}");
    let url_b = format!("http://127.0.0.1:{port_b}");
    let _anvil_a = spawn_anvil(port_a);
    let _anvil_b = spawn_anvil(port_b);

    // ── Chain A: produce a finalized transaction ────────────────────────────
    let receipt = cast(&[
        "send",
        "--rpc-url",
        &url_a,
        "--private-key",
        ACCT0_KEY,
        ACCT1,
        "--value",
        "1ether",
        "--json",
    ]);
    let hash = serde_json::from_str::<serde_json::Value>(&receipt).unwrap()["transactionHash"]
        .as_str()
        .expect("transactionHash")
        .to_string();
    // Mine past the reorg-safety depth so the event is finality-locked.
    let _ = cast(&["rpc", "anvil_mine", "0xB", "--rpc-url", &url_a]);

    // ── Origin Adapter: observe A → mint → validate ─────────────────────────
    let observed = EthObserver::new(&url_a).observe(&hash).expect("observe A");
    let event = map_to_arrow(&observed, ETH_REORG_SAFETY_CONFIRMATIONS);
    assert_eq!(event.arrow_code, "1:ETH:FN:PR:1:A");

    let registries = Registries::load_embedded().unwrap();
    let pipeline = ValidationPipeline::new(&registries);
    assert!(
        pipeline.validate_event(&event).passed,
        "observed event must pass validation before consumption"
    );

    // ── Origin Adapter: sign the event into a transport envelope ────────────
    let key_path = std::env::temp_dir().join(format!("arrow-origin-key-{port_a}.json"));
    let sender_did = write_dev_key(&key_path);
    let envelope = build_signed_envelope(event, &sender_did, "did:arrow:consumer", Some(&key_path))
        .expect("sign envelope");

    // ── Consumption Adapter: verify provenance → reduce → settle on chain B ──
    let balance_before: u128 = cast(&["balance", ACCT1, "--rpc-url", &url_b])
        .parse()
        .expect("chain B balance before");

    // The consumer re-verifies the envelope signature and that the signer is
    // allow-listed; the operator supplies only the chain-B RPC + funding account.
    // The recipient and amount are derived from the event (observed chain-A
    // recipient ACCT1 and its 1-ETH value) — settlement is event-bound and
    // provenance-gated, not CLI-driven.
    let policy = ProvenancePolicy::new(vec![sender_did.clone()], "did:arrow:consumer");
    let replay = InMemoryReplayStore::with_24h_retention();
    let operator = OperatorConfig {
        rpc_url: url_b.clone(),
        from: ACCT0.to_string(),
        ..Default::default()
    };
    let outcome = verify_and_consume(&envelope, &policy, &replay, &operator, false)
        .expect("verify provenance + consume + settle");
    let _ = std::fs::remove_file(&key_path);

    assert!(
        outcome.reduction.disposition.permits_settlement(),
        "disposition should approve settlement"
    );
    let intent = outcome
        .intent
        .as_ref()
        .expect("approved settlement should carry an event-derived intent");
    // EVM addresses compare case-insensitively (RPC returns lowercase; ACCT1 is
    // EIP-55 checksummed) — the point is the recipient is event-derived, not CLI.
    assert_eq!(
        intent.destination.address.to_lowercase(),
        ACCT1.to_lowercase(),
        "recipient must be event-derived"
    );
    assert_eq!(
        intent.amount.value, ONE_ETHER_WEI,
        "amount must be event-derived"
    );
    assert!(
        outcome.settled,
        "settlement transaction should be submitted"
    );
    assert!(
        outcome.chain_b_tx.is_some(),
        "settlement should yield a chain-B tx hash"
    );

    // ── Verify the action actually landed on chain B ────────────────────────
    let balance_after: u128 = cast(&["balance", ACCT1, "--rpc-url", &url_b])
        .parse()
        .expect("chain B balance after");
    let expected: u128 = ONE_ETHER_WEI.parse().unwrap();
    assert_eq!(
        balance_after - balance_before,
        expected,
        "chain B recipient should have received exactly the settled amount"
    );
}
