//! `arrow-origin-eth` — observe an Ethereum transaction and emit a signed Arrow
//! event to a node.
//!
//! ```text
//! arrow-origin-eth \
//!     --rpc http://127.0.0.1:8545 \
//!     --tx 0x<hash> \
//!     --to http://127.0.0.1:8080 \
//!     --key-file dev-key.json \
//!     --receiver did:web:node-b.example
//! ```
//!
//! Use `--dry-run` to print the minted event JSON without contacting a node, or
//! `--out <file>` to write the signed envelope for a Consumption Adapter to verify.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::Parser;

use arrow_origin_eth::emit::{build_signed_envelope, post_envelope};
use arrow_origin_eth::mapping::{map_to_arrow, ETH_REORG_SAFETY_CONFIRMATIONS};
use arrow_origin_eth::observer::EthObserver;

#[derive(Parser, Debug)]
#[command(
    name = "arrow-origin-eth",
    about = "Ethereum Origin Adapter: observe a tx and emit a signed Arrow event."
)]
struct Cli {
    /// Ethereum JSON-RPC endpoint (e.g. http://127.0.0.1:8545).
    #[arg(long)]
    rpc: String,

    /// Transaction hash to observe (0x-prefixed).
    #[arg(long)]
    tx: String,

    /// Arrow node base URL to POST the event to (e.g. http://127.0.0.1:8080).
    /// Optional; provide --to and/or --out (at least one) unless --dry-run.
    #[arg(long)]
    to: Option<String>,

    /// Write the signed envelope JSON to this path (for a Consumption Adapter to
    /// verify). May be combined with --to.
    #[arg(long)]
    out: Option<PathBuf>,

    /// Dev key file ({ private_key_b64, key_ref, did }) used to sign the envelope.
    /// Required unless --dry-run is set.
    #[arg(long, required_unless_present = "dry_run")]
    key_file: Option<PathBuf>,

    /// Receiver id (the destination node's DID).
    /// Required unless --dry-run is set.
    #[arg(long, required_unless_present = "dry_run")]
    receiver: Option<String>,

    /// Sender id. Overridden by the DID in the key file when signing.
    #[arg(long, default_value = "did:origin:eth-adapter")]
    sender: String,

    /// Reorg-safety confirmation depth (default: 12, Ethereum mainnet).
    #[arg(long, default_value_t = ETH_REORG_SAFETY_CONFIRMATIONS)]
    reorg_safety: u64,

    /// Print the minted event JSON and exit without contacting a node.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Returns `Ok(true)` when the run succeeded (event accepted, or dry-run printed).
fn run() -> Result<bool> {
    let cli = Cli::parse();

    let observer = EthObserver::new(&cli.rpc);
    let observed = observer
        .observe(&cli.tx)
        .with_context(|| format!("observing tx {}", cli.tx))?;
    let confirmations = observed.confirmations();

    let event = map_to_arrow(&observed, cli.reorg_safety);
    eprintln!(
        "observed {} — {} confirmation(s) → arrow_code {}",
        cli.tx, confirmations, event.arrow_code
    );

    if cli.dry_run {
        println!("{}", serde_json::to_string_pretty(&event)?);
        return Ok(true);
    }

    if cli.to.is_none() && cli.out.is_none() {
        bail!("provide --to (POST to a node) and/or --out (write signed envelope), or --dry-run");
    }

    // Required-unless-dry-run flags are guaranteed present here by clap.
    let key_file = cli
        .key_file
        .expect("--key-file required when not --dry-run");
    let receiver = cli
        .receiver
        .expect("--receiver required when not --dry-run");

    let envelope = build_signed_envelope(event, &cli.sender, &receiver, Some(&key_file))?;

    if let Some(out) = &cli.out {
        std::fs::write(out, serde_json::to_string_pretty(&envelope)?)
            .with_context(|| format!("writing signed envelope to {}", out.display()))?;
        eprintln!("wrote signed envelope → {}", out.display());
    }

    let mut accepted = true;
    if let Some(to) = &cli.to {
        let outcome = post_envelope(&envelope, to)?;
        let status = if outcome.accepted {
            "ACCEPTED"
        } else {
            "REJECTED"
        };
        println!(
            "[{status}]  ref: {}",
            outcome.ref_envelope_id.as_deref().unwrap_or("—")
        );
        for v in &outcome.violations {
            println!("  {v}");
        }
        accepted = outcome.accepted;
    }

    Ok(accepted)
}
