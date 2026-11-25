//! `arrow-origin-leg` — map a structured legal attestation to a signed Arrow LEG
//! event and emit it to a node.
//!
//! ```text
//! arrow-origin-leg \
//!     --attestation attestation.json \
//!     --to http://127.0.0.1:8080 \
//!     --key-file dev-key.json \
//!     --receiver did:web:node-b.example
//! ```
//!
//! The attestation file is a JSON object matching [`LegalAttestation`]. Use
//! `--dry-run` to print the minted event JSON without contacting a node, or
//! `--out <file>` to write the signed envelope for the LEG Consumption Adapter to
//! verify.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::Parser;

use arrow_origin_leg::attestation::{map_to_arrow, LegalAttestation, LEG_SUBTYPES};
use arrow_origin_leg::emit::{build_signed_envelope, post_envelope};

#[derive(Parser, Debug)]
#[command(
    name = "arrow-origin-leg",
    about = "Legal-attestation Origin Adapter: map a structured legal attestation to a signed Arrow LEG event."
)]
struct Cli {
    /// Path to the structured legal attestation JSON (a `LegalAttestation` object).
    #[arg(long)]
    attestation: PathBuf,

    /// Arrow node base URL to POST the event to (e.g. http://127.0.0.1:8080).
    /// Provide --to and/or --out (at least one) unless --dry-run.
    #[arg(long)]
    to: Option<String>,

    /// Write the signed envelope JSON to this path (for the LEG Consumption Adapter
    /// to verify). May be combined with --to.
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
    #[arg(long, default_value = "did:origin:leg-adapter")]
    sender: String,

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

    let raw = std::fs::read_to_string(&cli.attestation)
        .with_context(|| format!("cannot read {}", cli.attestation.display()))?;
    let att: LegalAttestation = serde_json::from_str(&raw).with_context(|| {
        format!(
            "cannot parse {} as a LegalAttestation",
            cli.attestation.display()
        )
    })?;

    if !LEG_SUBTYPES.contains(&att.leg_subtype.as_str()) {
        bail!(
            "leg_subtype `{}` is not one of {:?}",
            att.leg_subtype,
            LEG_SUBTYPES
        );
    }

    // Fail fast on LEG-009: a LEG event MUST carry a jurisdiction. Minting an event
    // with neither country nor supra would only be rejected downstream by the node.
    if att.country.is_none() && att.supra.is_none() {
        bail!("attestation must set `country` or `supra` (LEG-009: leg_jurisdiction is required)");
    }

    let event = map_to_arrow(&att);
    eprintln!(
        "mapped {} attestation (jurisdiction {}) → arrow_code {}",
        att.leg_subtype,
        event.jurisdiction.as_deref().unwrap_or("—"),
        event.arrow_code
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
