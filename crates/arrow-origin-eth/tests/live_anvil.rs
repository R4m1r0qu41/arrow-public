//! Live integration test against a local anvil node.
//!
//! Gated with `#[ignore]` because it requires Foundry's `anvil` and `cast` on
//! PATH — they are not guaranteed in CI. Run explicitly with:
//!
//! ```text
//! cargo test -p arrow-origin-eth --test live_anvil -- --ignored
//! ```
//!
//! The test spawns anvil, sends a real value transfer with `cast`, observes the
//! transaction through [`EthObserver`], and asserts the minted Arrow event both
//! carries the expected code and passes `arrow-core`'s validation pipeline —
//! pending at 1 confirmation, final once mined to the reorg-safety depth.

use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use arrow_core::registry::loader::Registries;
use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_origin_eth::mapping::{map_to_arrow, ETH_REORG_SAFETY_CONFIRMATIONS};
use arrow_origin_eth::observer::EthObserver;

/// Anvil's first deterministic dev account (default mnemonic) and its key.
const ACCT0_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const ACCT1: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";

/// Kills the anvil child process on drop so the test never leaks a node.
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

/// Pick a free TCP port by binding to :0 and releasing it.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Run `cast` and return trimmed stdout, panicking with stderr on failure.
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

#[test]
#[ignore = "requires foundry (anvil + cast) on PATH"]
fn observe_real_anvil_tx_to_valid_arrow_event() {
    if !tool_available("anvil") || !tool_available("cast") {
        eprintln!("skipping: anvil/cast not found on PATH");
        return;
    }

    let port = free_port();
    let url = format!("http://127.0.0.1:{port}");

    let child = Command::new("anvil")
        .args(["--port", &port.to_string(), "--silent"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning anvil");
    let _guard = AnvilGuard(child);

    // Wait for the RPC to come up.
    let mut ready = false;
    for _ in 0..50 {
        let ok = Command::new("cast")
            .args(["block-number", "--rpc-url", &url])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            ready = true;
            break;
        }
        sleep(Duration::from_millis(100));
    }
    assert!(ready, "anvil did not become ready on {url}");

    // Send a real value transfer; anvil auto-mines it into a block.
    let tx_hash = cast(&[
        "send",
        "--rpc-url",
        &url,
        "--private-key",
        ACCT0_KEY,
        ACCT1,
        "--value",
        "1ether",
        "--json",
    ]);
    let receipt: serde_json::Value = serde_json::from_str(&tx_hash).expect("cast send --json");
    let hash = receipt["transactionHash"]
        .as_str()
        .expect("transactionHash in receipt")
        .to_string();

    let observer = EthObserver::new(&url);
    let registries = Registries::load_embedded().expect("embedded registries");
    let pipeline = ValidationPipeline::new(&registries);

    // At 1 confirmation: pending, Hold, finality 0 — still a protocol-valid event.
    let observed = observer.observe(&hash).expect("observe mined tx");
    assert_eq!(
        observed.confirmations(),
        1,
        "freshly mined tx → 1 confirmation"
    );
    let pending = map_to_arrow(&observed, ETH_REORG_SAFETY_CONFIRMATIONS);
    assert_eq!(pending.arrow_code, "1:ETH:PD:HL:0:B");
    assert!(
        pipeline.validate_event(&pending).passed,
        "pending event must validate"
    );

    // Mine to the reorg-safety depth: 11 more blocks → 12 confirmations.
    let _ = cast(&["rpc", "anvil_mine", "0xB", "--rpc-url", &url]);
    let observed = observer.observe(&hash).expect("re-observe deep tx");
    assert!(
        observed.confirmations() >= ETH_REORG_SAFETY_CONFIRMATIONS,
        "expected >= 12 confirmations, got {}",
        observed.confirmations()
    );
    let final_ev = map_to_arrow(&observed, ETH_REORG_SAFETY_CONFIRMATIONS);
    assert_eq!(final_ev.arrow_code, "1:ETH:FN:PR:1:A");
    assert!(
        pipeline.validate_event(&final_ev).passed,
        "final event must validate"
    );
}
