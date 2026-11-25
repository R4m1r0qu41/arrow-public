//! `arrow-consume-leg` — verify a signed Arrow LEG envelope, reduce it, and route
//! it to a human review queue. Never auto-acts on a legal assertion.
//!
//! ```text
//! arrow-consume-leg \
//!     --envelope signed-envelope.json \
//!     --allow did:key:z6Mk... \
//!     --receiver did:arrow:leg-consumer
//! ```
//!
//! The envelope signature and sender allow-list are verified first; the event is
//! then re-validated (live LEG rules), reduced, and mapped to a fail-closed review
//! decision. A review-eligible event is enqueued for a human; a blocked/reversed/
//! closed event is denied. There is no autonomous destination action. Use
//! `--dry-run` to compute the decision without recording replay state or enqueuing;
//! use `--record-only` to record idempotency and enqueue without marking the
//! receipt as executed.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use arrow_adapter::ConsumerExecutionMode;
use arrow_consume_leg::{
    review::InMemoryReviewQueue, verify_and_review, LegReviewDecision, ProvenancePolicy,
    TransportEnvelope,
};
use arrow_transport::replay::{InMemoryReplayStore, ReplayStore, SqliteReplayStore};

#[derive(Parser, Debug)]
#[command(
    name = "arrow-consume-leg",
    about = "Fail-closed, human-in-the-loop LEG Consumption Adapter: verify a signed envelope, reduce, and route to human review."
)]
struct Cli {
    /// Path to the signed transport envelope JSON (as emitted by arrow-origin-leg --out).
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
    #[arg(long, default_value = ".arrow-consume-leg-replay.db")]
    replay_db: PathBuf,

    /// Compute the review decision without recording replay state or enqueuing.
    #[arg(long)]
    dry_run: bool,

    /// Record idempotency and enqueue the review item without marking the receipt
    /// as executed.
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

    let mode = if cli.dry_run {
        ConsumerExecutionMode::DryRun
    } else if cli.record_only {
        ConsumerExecutionMode::RecordOnly
    } else {
        ConsumerExecutionMode::Execute
    };

    // Dry-run never records replay state, so an ephemeral store suffices there;
    // record-only and execute use the durable SQLite DB.
    let replay: Box<dyn ReplayStore> = if mode == ConsumerExecutionMode::DryRun {
        Box::new(InMemoryReplayStore::with_24h_retention())
    } else {
        Box::new(
            SqliteReplayStore::open_with_24h_retention(&cli.replay_db)
                .with_context(|| format!("cannot open replay DB {}", cli.replay_db.display()))?,
        )
    };

    let queue = InMemoryReviewQueue::new();
    let outcome = verify_and_review(&envelope, &policy, replay.as_ref(), &queue, mode)?;
    let r = &outcome.reduction;

    eprintln!("provenance OK: signed by {}", envelope.sender_id);
    eprintln!(
        "reduced {} event(s) of lifecycle {} → {:?} (action {})\n  {}",
        r.event_count, r.lifecycle_id, r.disposition, r.action, r.reason
    );

    match outcome.decision {
        LegReviewDecision::Deny => {
            println!(
                "[DENY] disposition {:?} is terminal; no action and nothing queued for review",
                r.disposition
            );
        }
        LegReviewDecision::HoldForReview => {
            if let Some(item) = &outcome.review_item {
                eprintln!("  review item: {}", serde_json::to_string_pretty(item)?);
            }
            if outcome.queued {
                println!(
                    "[HELD FOR REVIEW] enqueued review item for a human operator (status {:?})",
                    outcome.receipt.status
                );
            } else {
                println!(
                    "[DRY-RUN] would hold for review; nothing enqueued (disposition {:?})",
                    r.disposition
                );
            }
        }
    }

    Ok(true)
}
