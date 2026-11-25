//! `arrow-registry-test` CLI — on-chain lifecycle testing and registry anchoring.
//!
//! Subcommands:
//! - `lifecycle run` — observe a tx, derive the Arrow event, emit the R-011 artifact
//! - `artifact hash` — recompute the extraction_artifact_hash from a saved artifact
//! - `anchor registry` — anchor a registry version hash on-chain
//! - `anchor verify` — verify an anchor transaction against a registry file

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use arrow_registry_test::{
    anchor::{entry_hash, registry_version_hash, verify_anchor},
    artifact::{compute_hash, verify_hash, ArtifactCore},
    lifecycle::{run_lifecycle, ChainFamily, LifecycleParams},
    networks,
};

#[derive(Parser, Debug)]
#[command(
    name = "arrow-registry-test",
    about = "Arrow Protocol on-chain lifecycle test harness (RFC-006 R-009/R-011)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// On-chain lifecycle test commands.
    Lifecycle {
        #[command(subcommand)]
        sub: LifecycleCommands,
    },
    /// Artifact hashing commands.
    Artifact {
        #[command(subcommand)]
        sub: ArtifactCommands,
    },
    /// Registry anchoring commands.
    Anchor {
        #[command(subcommand)]
        sub: AnchorCommands,
    },
}

#[derive(Subcommand, Debug)]
enum LifecycleCommands {
    /// Observe a transaction, derive the Arrow event, and emit the R-011 artifact.
    Run {
        /// Chain Registry code (e.g. ETH, BASE, ARB, BTC, SOL).
        #[arg(long)]
        origin: String,

        /// Network name (e.g. sepolia, base-sepolia, arb-sepolia, devnet, signet).
        #[arg(long)]
        network: String,

        /// Transaction reference (0x-prefixed hash for EVM; base58 sig for SOL;
        /// txid for BTC).
        #[arg(long)]
        tx: String,

        /// Registry entry version.
        #[arg(long, default_value = "1.0")]
        entry_version: String,

        /// Registry version.
        #[arg(long, default_value = "2.0")]
        registry_version: String,

        /// JSON-RPC endpoint. If omitted, uses the default for the network.
        #[arg(long)]
        rpc: Option<String>,

        /// BTC vout index (BTC only).
        #[arg(long)]
        btc_vout: Option<u32>,

        /// BTC RPC username (BTC only, or env ARROW_BTC_RPC_USER).
        #[arg(long)]
        btc_user: Option<String>,

        /// BTC RPC password (BTC only, or env ARROW_BTC_RPC_PASS).
        #[arg(long)]
        btc_pass: Option<String>,

        /// Output file for the artifact JSON. If omitted, prints to stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum ArtifactCommands {
    /// Recompute extraction_artifact_hash from a saved artifact file.
    Hash {
        /// Path to the artifact JSON file or the core JSON file.
        file: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum AnchorCommands {
    /// Compute registry hashes (without submitting on-chain).
    Hash {
        /// Registry JSON file.
        file: PathBuf,
        /// Also compute per-entry hashes.
        #[arg(long)]
        entries: bool,
    },
    /// Verify an anchor transaction against a registry file.
    Verify {
        /// Registry JSON file to recompute the hash for.
        file: PathBuf,
        /// Anchor transaction hash (0x-prefixed).
        anchor_tx: String,
        /// JSON-RPC endpoint of the chain where the anchor was submitted.
        #[arg(long)]
        rpc: String,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Lifecycle { sub } => lifecycle_cmd(sub),
        Commands::Artifact { sub } => artifact_cmd(sub),
        Commands::Anchor { sub } => anchor_cmd(sub),
    }
}

fn lifecycle_cmd(sub: LifecycleCommands) -> Result<()> {
    match sub {
        LifecycleCommands::Run {
            origin,
            network,
            tx,
            entry_version,
            registry_version,
            rpc,
            btc_vout,
            btc_user,
            btc_pass,
            out,
        } => {
            let chain = resolve_chain_params(&origin, &network)?;
            let rpc_url = rpc.unwrap_or_else(|| chain.default_rpc.to_string());

            let params = LifecycleParams {
                registry_code: origin.clone(),
                entry_version,
                registry_version,
                network: chain.network_scope.to_string(),
                network_name: network.clone(),
                family: chain.family,
                rpc_url,
                tx_ref_value: tx,
                btc_vout,
                btc_rpc_user: btc_user.or_else(|| std::env::var("ARROW_BTC_RPC_USER").ok()),
                btc_rpc_pass: btc_pass.or_else(|| std::env::var("ARROW_BTC_RPC_PASS").ok()),
                maps_to_fields: default_maps_to_fields(&origin),
                expected_chain_id: chain.expected_chain_id,
                expected_btc_network: chain.expected_btc_network,
                expected_sol_genesis: chain.expected_sol_genesis,
            };

            let artifact = run_lifecycle(&params)?;
            let json = serde_json::to_string_pretty(&artifact)?;

            match out {
                Some(path) => std::fs::write(&path, &json)
                    .with_context(|| format!("writing artifact to {}", path.display()))?,
                None => println!("{json}"),
            }
            Ok(())
        }
    }
}

fn artifact_cmd(sub: ArtifactCommands) -> Result<()> {
    match sub {
        ArtifactCommands::Hash { file } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let json: serde_json::Value = serde_json::from_str(&content).context("invalid JSON")?;

            // Accept either a full artifact or just the core object.
            let core: ArtifactCore = if json.get("core").is_some() {
                // Full artifact — verify and print the stored hash.
                let artifact: arrow_registry_test::artifact::ExtractionArtifact =
                    serde_json::from_value(json).context("invalid artifact JSON")?;
                verify_hash(&artifact).context("artifact hash verification failed")?;
                println!("{}", artifact.extraction_artifact_hash);
                return Ok(());
            } else {
                serde_json::from_value(json).context("invalid artifact core JSON")?
            };

            let hash = compute_hash(&core)?;
            println!("{hash}");
            Ok(())
        }
    }
}

fn anchor_cmd(sub: AnchorCommands) -> Result<()> {
    match sub {
        AnchorCommands::Hash { file, entries } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let json: serde_json::Value = serde_json::from_str(&content).context("invalid JSON")?;
            let rh = registry_version_hash(&json)?;
            println!("registry_version_hash: {rh}");

            if entries {
                if let Some(arr) = json.get("entries").and_then(|v| v.as_array()) {
                    for entry in arr {
                        let code = entry.get("code").and_then(|v| v.as_str()).unwrap_or("?");
                        let eh = entry_hash(entry)?;
                        println!("entry_hash[{code}]: {eh}");
                    }
                }
            }
            Ok(())
        }
        AnchorCommands::Verify {
            file,
            anchor_tx,
            rpc,
        } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let json: serde_json::Value = serde_json::from_str(&content).context("invalid JSON")?;
            let hash = registry_version_hash(&json)?;
            verify_anchor(&rpc, &anchor_tx, &hash).context("anchor verification failed")?;
            println!("OK: anchor {anchor_tx} matches registry hash {hash}");
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChainParams {
    family: ChainFamily,
    network_scope: &'static str,
    default_rpc: &'static str,
    expected_chain_id: Option<u64>,
    expected_btc_network: Option<String>,
    expected_sol_genesis: Option<String>,
}

fn resolve_chain_params(origin: &str, network: &str) -> Result<ChainParams> {
    match (origin, network) {
        ("ETH", "mainnet") | ("ETH", "ethereum") => Ok(ChainParams {
            family: ChainFamily::Evm,
            network_scope: "mainnet",
            default_rpc: networks::default_rpc::ETH_MAINNET,
            expected_chain_id: Some(networks::chain_ids::ETH_MAINNET),
            expected_btc_network: None,
            expected_sol_genesis: None,
        }),
        ("ETH", "sepolia") => Ok(ChainParams {
            family: ChainFamily::Evm,
            network_scope: "testnet",
            default_rpc: networks::default_rpc::ETH_SEPOLIA,
            expected_chain_id: Some(networks::chain_ids::ETH_SEPOLIA),
            expected_btc_network: None,
            expected_sol_genesis: None,
        }),
        ("BASE", "mainnet") | ("BASE", "base") => Ok(ChainParams {
            family: ChainFamily::Evm,
            network_scope: "mainnet",
            default_rpc: networks::default_rpc::BASE_MAINNET,
            expected_chain_id: Some(networks::chain_ids::BASE_MAINNET),
            expected_btc_network: None,
            expected_sol_genesis: None,
        }),
        ("BASE", "base-sepolia") | ("BASE", "sepolia") => Ok(ChainParams {
            family: ChainFamily::Evm,
            network_scope: "testnet",
            default_rpc: networks::default_rpc::BASE_SEPOLIA,
            expected_chain_id: Some(networks::chain_ids::BASE_SEPOLIA),
            expected_btc_network: None,
            expected_sol_genesis: None,
        }),
        ("ARB", "mainnet") | ("ARB", "arbitrum-one") | ("ARB", "arbitrum") => Ok(ChainParams {
            family: ChainFamily::Evm,
            network_scope: "mainnet",
            default_rpc: networks::default_rpc::ARB_MAINNET,
            expected_chain_id: Some(networks::chain_ids::ARB_MAINNET),
            expected_btc_network: None,
            expected_sol_genesis: None,
        }),
        ("ARB", "arb-sepolia") | ("ARB", "sepolia") => Ok(ChainParams {
            family: ChainFamily::Evm,
            network_scope: "testnet",
            default_rpc: networks::default_rpc::ARB_SEPOLIA,
            expected_chain_id: Some(networks::chain_ids::ARB_SEPOLIA),
            expected_btc_network: None,
            expected_sol_genesis: None,
        }),
        ("SOL", "mainnet") | ("SOL", "mainnet-beta") => Ok(ChainParams {
            family: ChainFamily::Svm,
            network_scope: "mainnet",
            default_rpc: networks::default_rpc::SOL_MAINNET,
            expected_chain_id: None,
            expected_btc_network: None,
            expected_sol_genesis: Some(networks::sol_genesis::MAINNET_BETA.to_string()),
        }),
        ("SOL", "devnet") => Ok(ChainParams {
            family: ChainFamily::Svm,
            network_scope: "testnet",
            default_rpc: networks::default_rpc::SOL_DEVNET,
            expected_chain_id: None,
            expected_btc_network: None,
            expected_sol_genesis: Some(networks::sol_genesis::DEVNET.to_string()),
        }),
        ("BTC", "mainnet") | ("BTC", "main") => Ok(ChainParams {
            family: ChainFamily::Utxo,
            network_scope: "mainnet",
            default_rpc: "",
            expected_chain_id: None,
            expected_btc_network: Some(networks::btc_chain::MAINNET.to_string()),
            expected_sol_genesis: None,
        }),
        ("BTC", "signet") => Ok(ChainParams {
            family: ChainFamily::Utxo,
            network_scope: "testnet",
            default_rpc: "",
            expected_chain_id: None,
            expected_btc_network: Some(networks::btc_chain::SIGNET.to_string()),
            expected_sol_genesis: None,
        }),
        (o, n) => anyhow::bail!("unsupported origin/network combination: {o}/{n}"),
    }
}

fn default_maps_to_fields(registry_code: &str) -> Vec<String> {
    match registry_code {
        "ETH" => vec![
            "code".to_string(),
            "protocol_required.finality_model".to_string(),
            "protocol_required.finality_basis_default".to_string(),
            "protocol_required.reorg_safety_confirmations".to_string(),
        ],
        "BASE" | "ARB" => vec![
            "code".to_string(),
            "protocol_required.finality_model".to_string(),
            "protocol_required.finality_basis_default".to_string(),
            "protocol_required.challenge_window_seconds".to_string(),
        ],
        "BTC" => vec![
            "code".to_string(),
            "protocol_required.finality_model".to_string(),
            "protocol_required.reorg_safety_confirmations".to_string(),
        ],
        "SOL" => vec![
            "code".to_string(),
            "protocol_required.finality_model".to_string(),
        ],
        _ => vec!["code".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_sol_devnet_sets_fail_closed_genesis_expectation() {
        let params = resolve_chain_params("SOL", "devnet").unwrap();
        assert_eq!(params.family, ChainFamily::Svm);
        assert_eq!(params.network_scope, "testnet");
        assert_eq!(
            params.expected_sol_genesis.as_deref(),
            Some(networks::sol_genesis::DEVNET)
        );
        assert!(params.expected_btc_network.is_none());
    }

    #[test]
    fn cli_btc_signet_sets_fail_closed_network_expectation() {
        let params = resolve_chain_params("BTC", "signet").unwrap();
        assert_eq!(params.family, ChainFamily::Utxo);
        assert_eq!(params.network_scope, "testnet");
        assert_eq!(
            params.expected_btc_network.as_deref(),
            Some(networks::btc_chain::SIGNET)
        );
        assert!(params.expected_sol_genesis.is_none());
    }

    #[test]
    fn cli_rejects_btc_network_without_identity_expectation() {
        let err = resolve_chain_params("BTC", "testnet").unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported origin/network combination"),
            "unsupported BTC networks must not mint artifacts without fail-closed identity; got: {err}"
        );
    }

    #[test]
    fn cli_mainnet_networks_set_fail_closed_identities() {
        let eth = resolve_chain_params("ETH", "mainnet").unwrap();
        assert_eq!(eth.network_scope, "mainnet");
        assert_eq!(
            eth.expected_chain_id,
            Some(networks::chain_ids::ETH_MAINNET)
        );

        let base = resolve_chain_params("BASE", "mainnet").unwrap();
        assert_eq!(base.network_scope, "mainnet");
        assert_eq!(
            base.expected_chain_id,
            Some(networks::chain_ids::BASE_MAINNET)
        );

        let arb = resolve_chain_params("ARB", "arbitrum-one").unwrap();
        assert_eq!(arb.network_scope, "mainnet");
        assert_eq!(
            arb.expected_chain_id,
            Some(networks::chain_ids::ARB_MAINNET)
        );

        let sol = resolve_chain_params("SOL", "mainnet-beta").unwrap();
        assert_eq!(sol.network_scope, "mainnet");
        assert_eq!(
            sol.expected_sol_genesis.as_deref(),
            Some(networks::sol_genesis::MAINNET_BETA)
        );

        let btc = resolve_chain_params("BTC", "mainnet").unwrap();
        assert_eq!(btc.network_scope, "mainnet");
        assert_eq!(
            btc.expected_btc_network.as_deref(),
            Some(networks::btc_chain::MAINNET)
        );
    }
}
