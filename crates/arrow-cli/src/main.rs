use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

mod commands;

#[derive(Parser)]
#[command(name = "arrow", about = "Arrow Protocol CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Validate an ArrowEvent JSON file against the embedded registries
    Validate {
        /// Path to the event JSON file
        path: PathBuf,
        /// Emit raw JSON ValidationResult instead of human-readable output
        #[arg(long)]
        json: bool,
        /// Prior-event JSON files loaded into a LifecycleContext before
        /// validation (repeatable). Enables context-gated rules
        /// (LFC-001/003/007) exactly as validate_event_with_context does.
        #[arg(long = "context")]
        context: Vec<PathBuf>,
    },
    /// Decode and display the fields of an Arrow code string
    Inspect {
        /// Arrow code string, e.g. "2:CST:FN:PR:1:C"
        arrow_code: String,
    },
    /// Wrap an ArrowEvent in a TransportEnvelope and POST it to a node
    Emit {
        /// Base URL of the receiving node (e.g. http://127.0.0.1:8080)
        #[arg(long)]
        to: String,
        /// Path to the ArrowEvent JSON file
        #[arg(long)]
        event: PathBuf,
        /// sender_id to use in the envelope (overridden by --key-file DID when signing)
        #[arg(long, default_value = "did:arrow:pilot:cli")]
        from: String,
        /// receiver_id to use in the envelope
        #[arg(long, default_value = "did:arrow:pilot:node")]
        to_id: String,
        /// Path to a dev key file (produced by `arrow identity generate`).
        /// When provided, the envelope is signed with the key's Ed25519 private key
        /// and sender_id is set to the key file's DID.
        #[arg(long)]
        key_file: Option<PathBuf>,
    },
    /// Identity and signing operations
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },
    /// Acknowledgment inspection operations
    Ack {
        #[command(subcommand)]
        action: AckAction,
    },
}

#[derive(Subcommand)]
enum IdentityAction {
    /// Resolve a DID and print the DID Document
    Resolve { did: String },
    /// Generate a dev/test Ed25519 keypair and write a key file
    Generate {
        #[arg(long, default_value = "dev-ed25519.json")]
        output: PathBuf,
    },
    /// Sign an envelope with a dev key file
    Sign {
        #[arg(long)]
        key_file: PathBuf,
        #[arg(long)]
        envelope: PathBuf,
        #[arg(long, default_value = "signed-envelope.json")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum AckAction {
    /// Inspect an AckEnvelope JSON file
    Inspect {
        /// Path to the AckEnvelope JSON file
        ack: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Cmd::Validate {
            path,
            json,
            context,
        } => match commands::validate::run(path, *json, context) {
            Ok(passed) => process::exit(if passed { 0 } else { 1 }),
            Err(e) => {
                eprintln!("error: {e:#}");
                process::exit(2);
            }
        },
        Cmd::Inspect { arrow_code } => match commands::inspect::run(arrow_code) {
            Ok(()) => process::exit(0),
            Err(e) => {
                eprintln!("error: {e:#}");
                process::exit(1);
            }
        },
        Cmd::Emit {
            to,
            event,
            from,
            to_id,
            key_file,
        } => match commands::emit::run(to, event, from, to_id, key_file.as_deref()) {
            Ok(accepted) => process::exit(if accepted { 0 } else { 1 }),
            Err(e) => {
                eprintln!("error: {e:#}");
                process::exit(2);
            }
        },
        Cmd::Identity { action } => match action {
            IdentityAction::Resolve { did } => match commands::identity::resolve(did) {
                Ok(()) => process::exit(0),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(1);
                }
            },
            IdentityAction::Generate { output } => match commands::identity::generate(output) {
                Ok(()) => process::exit(0),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(2);
                }
            },
            IdentityAction::Sign {
                key_file,
                envelope,
                output,
            } => match commands::identity::sign(key_file, envelope, output) {
                Ok(()) => process::exit(0),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(2);
                }
            },
        },
        Cmd::Ack { action } => match action {
            AckAction::Inspect { ack } => match commands::ack::run(ack) {
                Ok(()) => process::exit(0),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(2);
                }
            },
        },
    };
}
