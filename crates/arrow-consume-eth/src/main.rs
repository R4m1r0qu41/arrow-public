//! `arrow-consume-eth` — verify a signed Arrow envelope's provenance, reduce its
//! payload, and settle on a destination chain when the disposition permits.
//!
//! ```text
//! arrow-consume-eth \
//!     --envelope signed-envelope.json \
//!     --allow did:key:z6Mk... \
//!     --receiver did:arrow:consumer \
//!     --chain-b-rpc http://127.0.0.1:8546 \
//!     --from 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
//! ```
//!
//! The envelope signature and sender allow-list are verified first; the settlement
//! recipient and amount are then derived from typed normalized source facts, not
//! free-form metadata or CLI input. The operator supplies only the chain-B RPC,
//! funding account, and the trusted senders. Non-dry-run settlement records the
//! envelope sender/nonce in a durable replay DB. Use `--dry-run` to verify +
//! print the decision without sending a chain-B transaction or recording replay
//! state; use `--record-only` to record idempotency without sending.
//!
//! ⚠️ Prototype: off-chain settlement via an unlocked account and 1:1 address
//! mapping. See the crate docs for the full trust-boundary notes.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use arrow_adapter::{ConsumerExecutionMode, InMemoryDestinationExecutionStore};
use arrow_consume_eth::{
    verify_and_consume_with_actions, OperatorConfig, ProvenancePolicy, TransportEnvelope,
};
use arrow_transport::replay::{InMemoryReplayStore, ReplayStore, SqliteReplayStore};

#[derive(Parser, Debug)]
#[command(
    name = "arrow-consume-eth",
    about = "Arrow Consumption Adapter: verify a signed envelope, reduce, and settle (event-bound)."
)]
struct Cli {
    /// Path to the signed transport envelope JSON (as emitted by arrow-origin-eth --out).
    #[arg(long)]
    envelope: PathBuf,

    /// Trusted sender DID(s). Repeatable. Fail-closed: at least one is required and
    /// the envelope's signer must be among them.
    #[arg(long = "allow")]
    allow: Vec<String>,

    /// This consumer's DID. The envelope receiver_id must include it.
    #[arg(long)]
    receiver: String,

    /// Durable replay database used to reject repeated envelope nonces.
    #[arg(long, default_value = ".arrow-consume-eth-replay.db")]
    replay_db: PathBuf,

    /// Destination chain JSON-RPC endpoint.
    #[arg(long, required_unless_present_any = ["dry_run", "record_only"])]
    chain_b_rpc: Option<String>,

    /// Funding account on chain B (0x address; must be unlocked on the node).
    /// The settlement recipient and amount come from the event, not the CLI.
    #[arg(long, required_unless_present_any = ["dry_run", "record_only"])]
    from: Option<String>,

    /// Verify + print the disposition and event-bound intent without sending a
    /// chain-B transaction.
    #[arg(long)]
    dry_run: bool,

    /// Verify, reduce, and record replay/execution idempotency without sending a
    /// chain-B transaction.
    #[arg(long, conflicts_with = "dry_run")]
    record_only: bool,
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

/// Returns `Ok(true)` when the run succeeded (settled, or a non-settling
/// disposition was correctly reported, or dry-run printed).
fn run() -> Result<bool> {
    let cli = Cli::parse();

    let raw = std::fs::read_to_string(&cli.envelope)
        .with_context(|| format!("cannot read {}", cli.envelope.display()))?;
    let envelope: TransportEnvelope = serde_json::from_str(&raw).with_context(|| {
        format!(
            "cannot parse {} as a signed transport envelope",
            cli.envelope.display()
        )
    })?;

    let policy = ProvenancePolicy::new(cli.allow.clone(), cli.receiver.clone());
    let operator = OperatorConfig {
        rpc_url: cli.chain_b_rpc.clone().unwrap_or_default(),
        from: cli.from.clone().unwrap_or_default(),
        ..Default::default()
    };

    let mode = if cli.dry_run {
        ConsumerExecutionMode::DryRun
    } else if cli.record_only {
        ConsumerExecutionMode::RecordOnly
    } else {
        ConsumerExecutionMode::Execute
    };

    // Dry-run never records replay state, so an ephemeral in-memory store
    // suffices there; record-only and execute use the durable SQLite DB.
    let replay: Box<dyn ReplayStore> = if mode == ConsumerExecutionMode::DryRun {
        Box::new(InMemoryReplayStore::with_24h_retention())
    } else {
        Box::new(
            SqliteReplayStore::open_with_24h_retention(&cli.replay_db)
                .with_context(|| format!("cannot open replay DB {}", cli.replay_db.display()))?,
        )
    };
    let execution_store = InMemoryDestinationExecutionStore::new();
    let outcome = verify_and_consume_with_actions(
        &envelope,
        &policy,
        replay.as_ref(),
        &execution_store,
        &operator,
        mode,
    )?;
    let r = &outcome.reduction;

    eprintln!("provenance OK: signed by {}", envelope.sender_id);
    eprintln!(
        "reduced {} event(s) of lifecycle {} → {:?} (action {})\n  {}",
        r.event_count, r.lifecycle_id, r.disposition, r.action, r.reason
    );
    if let Some(intent) = &outcome.intent {
        eprintln!(
            "  action intent: {} {} -> {}:{}",
            intent.amount.value,
            intent.amount.unit,
            intent.destination.system,
            intent.destination.address
        );
    }
    if let Some(execution_id) = &outcome.destination_execution_id {
        eprintln!("  destination execution id: {execution_id}");
    }

    if outcome.settled {
        let tx = outcome.chain_b_tx.as_deref().unwrap_or("—");
        println!("[SETTLED on chain B]  tx: {tx}");
    } else if mode == ConsumerExecutionMode::DryRun {
        println!(
            "[DRY-RUN] disposition {:?}; no chain-B transaction sent",
            r.disposition
        );
    } else if mode == ConsumerExecutionMode::RecordOnly {
        println!(
            "[RECORDED] disposition {:?}; idempotency recorded and no chain-B transaction sent",
            r.disposition
        );
    } else {
        println!(
            "[NO ACTION] disposition {:?} does not permit settlement",
            r.disposition
        );
    }

    Ok(true)
}
