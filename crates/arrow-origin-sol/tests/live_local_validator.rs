//! Live integration test against a local Solana test validator.
//!
//! Gated with `#[ignore]` because it requires `solana-test-validator`,
//! `solana-keygen`, and `solana` CLI on PATH. Run explicitly with:
//!
//! ```text
//! cargo test -p arrow-origin-sol --test live_local_validator -- --ignored
//! ```
//!
//! The test spawns a local validator, creates two keypairs, airdrops SOL,
//! sends a real native transfer, observes the transaction through
//! [`SolObserver`], and asserts:
//! - at `Confirmed` commitment → `1:SOL:PD:HL:0:B`
//! - at `Finalized` commitment → `1:SOL:FN:PR:1:A`

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use arrow_core::registry::loader::Registries;
use arrow_core::validation::pipeline::ValidationPipeline;
use arrow_origin_sol::{
    mapping::{map_to_arrow, SolCommitment},
    observer::SolObserver,
};

/// Kills the validator child process on drop.
struct ValidatorGuard(Child);
impl Drop for ValidatorGuard {
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

fn sol_cli(url: &str, args: &[&str]) -> String {
    let out = Command::new("solana")
        .args(["--url", url])
        .args(args)
        .output()
        .expect("running solana CLI");
    assert!(
        out.status.success(),
        "solana {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Parse the base58 signature out of `solana transfer` output:
/// "Signature: <base58>"
fn parse_signature(output: &str) -> String {
    output
        .lines()
        .find(|l| l.trim_start().starts_with("Signature:"))
        .and_then(|l| l.split_whitespace().last())
        .unwrap_or_else(|| panic!("no Signature line in transfer output:\n{output}"))
        .to_string()
}

#[test]
#[ignore = "requires solana-test-validator + solana + solana-keygen on PATH"]
fn observe_local_validator_tx_to_valid_arrow_event() {
    if !tool_available("solana-test-validator")
        || !tool_available("solana")
        || !tool_available("solana-keygen")
    {
        eprintln!("skipping: solana-test-validator/solana/solana-keygen not found on PATH");
        return;
    }

    let rpc_port = free_port();
    let url = format!("http://127.0.0.1:{rpc_port}");
    let addr = format!("127.0.0.1:{rpc_port}");

    let child = Command::new("solana-test-validator")
        .args(["--rpc-port", &rpc_port.to_string(), "--quiet"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning solana-test-validator");
    let _guard = ValidatorGuard(child);

    // Wait for the TCP port to open.
    let mut ready = false;
    for _ in 0..60 {
        if TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(100)).is_ok() {
            sleep(Duration::from_millis(500)); // give RPC time to initialise
            ready = true;
            break;
        }
        sleep(Duration::from_millis(500));
    }
    assert!(ready, "solana-test-validator did not open {addr}");

    let observer = SolObserver::new(&url);
    let registries = Registries::load_embedded().expect("embedded registries");
    let pipeline = ValidationPipeline::new(&registries);

    let tmpdir = std::env::temp_dir();
    let sender_path = tmpdir.join("arrow_sol_sender.json");
    let recipient_path = tmpdir.join("arrow_sol_recipient.json");

    for path in [&sender_path, &recipient_path] {
        Command::new("solana-keygen")
            .args([
                "new",
                "--no-bip39-passphrase",
                "--force",
                "--outfile",
                path.to_str().unwrap(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("solana-keygen new");
    }

    let sender_pubkey = sol_cli(
        &url,
        &["address", "--keypair", sender_path.to_str().unwrap()],
    );
    let recipient_pubkey = sol_cli(
        &url,
        &["address", "--keypair", recipient_path.to_str().unwrap()],
    );

    // Airdrop 2 SOL to sender and wait for confirmation.
    sol_cli(&url, &["airdrop", "2", &sender_pubkey]);
    sleep(Duration::from_secs(2));

    // Transfer 0.1 SOL to recipient at confirmed commitment.
    let transfer_out = sol_cli(
        &url,
        &[
            "transfer",
            "--keypair",
            sender_path.to_str().unwrap(),
            &recipient_pubkey,
            "0.1",
            "--commitment",
            "confirmed",
        ],
    );
    let signature = parse_signature(&transfer_out);

    // Observe at `Confirmed` → should be pre-final (pending).
    let obs_confirmed = observer
        .observe(&signature, SolCommitment::Confirmed)
        .expect("observe Confirmed")
        .expect("tx not yet at Confirmed; increase wait time");

    assert!(
        !obs_confirmed.commitment.is_final(),
        "confirmed commitment must be pre-final"
    );
    let pending = map_to_arrow(&obs_confirmed);
    assert_eq!(pending.arrow_code, "1:SOL:PD:HL:0:B");
    assert!(
        pipeline.validate_event(&pending).passed,
        "confirmed event must validate"
    );

    // Poll for `Finalized` (test validator usually finalises in ~10–30s).
    let mut final_obs = None;
    for _ in 0..60 {
        if let Ok(Some(obs)) = observer.observe(&signature, SolCommitment::Finalized) {
            final_obs = Some(obs);
            break;
        }
        sleep(Duration::from_millis(500));
    }
    let obs_final = final_obs.expect("tx never reached Finalized on test validator within 30s");

    let final_ev = map_to_arrow(&obs_final);
    assert_eq!(final_ev.arrow_code, "1:SOL:FN:PR:1:A");
    assert!(
        pipeline.validate_event(&final_ev).passed,
        "finalized event must validate"
    );

    assert!(
        obs_final.genesis_hash.is_some(),
        "genesis_hash must be populated by the observer"
    );
}
