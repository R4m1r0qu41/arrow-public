//! On-chain lifecycle orchestration (R-009).
//!
//! The lifecycle harness drives an end-to-end test for a single chain entry:
//! 1. Observe the transaction using the appropriate Origin Adapter.
//! 2. Derive the Arrow event.
//! 3. Build and hash the R-011 [`ExtractionArtifact`].
//!
//! Each run produces an artifact that is independent, reproducible from public
//! data, and can be anchored on-chain (see [`crate::anchor`]).

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::artifact::{
    build_artifact, ArtifactCore, ArtifactObservation, ExtractionArtifact, FinalityTiming, TxRef,
};

/// Chain family discriminator — drives observer selection and `tx_ref` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainFamily {
    Evm,
    Utxo,
    Svm,
}

impl ChainFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Evm => "evm",
            Self::Utxo => "utxo",
            Self::Svm => "svm",
        }
    }
}

/// Parameters for a single lifecycle run.
#[derive(Debug, Clone)]
pub struct LifecycleParams {
    /// Chain Registry `code` (e.g. `"ETH"`, `"BASE"`).
    pub registry_code: String,
    /// Registry entry version (e.g. `"1.0"`).
    pub entry_version: String,
    /// Registry version (e.g. `"2.0"`).
    pub registry_version: String,
    /// `"mainnet"` or `"testnet"`.
    pub network: String,
    /// Human-readable network name (e.g. `"sepolia"`, `"base-sepolia"`).
    pub network_name: String,
    /// Chain family driving observer selection.
    pub family: ChainFamily,
    /// JSON-RPC endpoint URL.
    pub rpc_url: String,
    /// Chain-native transaction reference (e.g. `0x…` for EVM).
    pub tx_ref_value: String,
    /// For BTC: vout index.
    pub btc_vout: Option<u32>,
    /// For BTC: RPC username.
    pub btc_rpc_user: Option<String>,
    /// For BTC: RPC password.
    pub btc_rpc_pass: Option<String>,
    /// List of `protocol_required.*` field paths this test corroborates.
    pub maps_to_fields: Vec<String>,
    /// Expected EIP-155 chain id (EVM only). Checked before minting.
    pub expected_chain_id: Option<u64>,
    /// Expected BTC network identity (e.g. `"signet"`, `"main"`, `"regtest"`).
    /// When set, mismatches abort before artifact generation (fail-closed).
    pub expected_btc_network: Option<String>,
    /// Expected Solana genesis hash (base58). When set, mismatches abort before
    /// artifact generation (fail-closed).
    pub expected_sol_genesis: Option<String>,
}

/// Run a full lifecycle test and return the extraction artifact.
pub fn run_lifecycle(params: &LifecycleParams) -> Result<ExtractionArtifact> {
    match params.family {
        ChainFamily::Evm => run_evm_lifecycle(params),
        ChainFamily::Utxo => run_btc_lifecycle(params),
        ChainFamily::Svm => run_sol_lifecycle(params),
    }
}

fn run_evm_lifecycle(params: &LifecycleParams) -> Result<ExtractionArtifact> {
    use arrow_origin_evm::{
        config::EvmChainConfig,
        mapping::{try_map_to_arrow, ObservedTx},
        observer::EvmObserver,
    };

    let observer = EvmObserver::new(&params.rpc_url);
    let obs: ObservedTx = observer
        .observe(&params.tx_ref_value)
        .context("EVM observation failed")?;

    // Build a config for the specific origin. For testnet runs, use a config
    // without expected_eip155_chain_id so the harness can supply it from params.
    let mut config = match params.registry_code.as_str() {
        "ETH" => EvmChainConfig::eth(),
        "BASE" => EvmChainConfig::base(),
        "ARB" => EvmChainConfig::arb(),
        other => bail!("unsupported EVM registry code: {other}"),
    };

    // Override expected chain id from params — allows testnet runs without
    // changing the mainnet config constructors.
    if let Some(id) = params.expected_chain_id {
        config.expected_eip155_chain_id = Some(id);
    }

    let event = try_map_to_arrow(&obs, &config).context("EVM mapping failed")?;

    let chain_id_observed = obs.observed_chain_id.to_string();
    let tx_ref = TxRef::evm(&obs.tx_hash);

    let finality_timing = match (event.finality_eta, event.finality_eta_type.as_deref()) {
        (Some(v), Some(t)) => Some(FinalityTiming {
            timing_type: t.to_string(),
            value: v,
        }),
        _ => None,
    };

    let core = ArtifactCore {
        artifact_version: "arrow-lifecycle-artifact-v0".to_string(),
        registry_code: params.registry_code.clone(),
        entry_version: params.entry_version.clone(),
        registry_version: params.registry_version.clone(),
        network: params.network.clone(),
        network_name: params.network_name.clone(),
        chain_family: ChainFamily::Evm.as_str().to_string(),
        chain_id_observed,
        tx_ref,
        finality_model: evm_finality_model(&params.registry_code),
        finality_basis: event.finality_basis.clone(),
        arrow_code: event.arrow_code.clone(),
        maps_to_fields: params.maps_to_fields.clone(),
        value: Some(obs.value_wei.clone()),
        settlement_chain: config.settlement_chain.clone(),
    };

    let observation = ArtifactObservation {
        observed_at: Utc::now().to_rfc3339(),
        observer_head: Some(obs.head_block),
        confirmations_at_observation: Some(obs.confirmations()),
        finality_timing,
    };

    build_artifact(core, observation)
}

fn run_btc_lifecycle(params: &LifecycleParams) -> Result<ExtractionArtifact> {
    use arrow_origin_btc::{mapping::map_to_arrow, observer::BtcObserver};

    let vout = params
        .btc_vout
        .context("btc_vout required for BTC lifecycle")?;
    let user = params
        .btc_rpc_user
        .as_deref()
        .context("btc_rpc_user required")?;
    let pass = params
        .btc_rpc_pass
        .as_deref()
        .context("btc_rpc_pass required")?;

    let observer = BtcObserver::new(&params.rpc_url, user, pass);
    let obs = observer
        .observe(&params.tx_ref_value, vout)
        .context("BTC observation failed")?;

    // Fail closed: if the caller specified an expected network, the observed
    // identity must match before any artifact is minted.
    match (&params.expected_btc_network, &obs.observed_network) {
        (Some(expected), Some(observed)) if observed != expected => {
            bail!(
                "BTC network mismatch: expected '{}', observed '{}'",
                expected,
                observed
            );
        }
        (Some(expected), None) => {
            bail!(
                "BTC network identity unavailable from node; expected '{}'",
                expected
            );
        }
        _ => {}
    }

    let event = map_to_arrow(&obs);
    let tx_ref = TxRef::btc(&obs.txid, obs.vout);
    let chain_id_observed = obs
        .observed_network
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let core = ArtifactCore {
        artifact_version: "arrow-lifecycle-artifact-v0".to_string(),
        registry_code: "BTC".to_string(),
        entry_version: params.entry_version.clone(),
        registry_version: params.registry_version.clone(),
        network: params.network.clone(),
        network_name: params.network_name.clone(),
        chain_family: ChainFamily::Utxo.as_str().to_string(),
        chain_id_observed,
        tx_ref,
        finality_model: "probabilistic".to_string(),
        finality_basis: event.finality_basis.clone(),
        arrow_code: event.arrow_code.clone(),
        maps_to_fields: params.maps_to_fields.clone(),
        value: Some(obs.amount_sats.to_string()),
        settlement_chain: None,
    };

    let observation = ArtifactObservation {
        observed_at: Utc::now().to_rfc3339(),
        observer_head: None,
        confirmations_at_observation: Some(obs.confirmations),
        finality_timing: None,
    };

    build_artifact(core, observation)
}

fn run_sol_lifecycle(params: &LifecycleParams) -> Result<ExtractionArtifact> {
    use arrow_origin_sol::{
        mapping::{map_to_arrow, SolCommitment},
        observer::SolObserver,
    };

    let observer = SolObserver::new(&params.rpc_url);
    let obs = observer
        .observe(&params.tx_ref_value, SolCommitment::Finalized)
        .context("Solana observation failed")?
        .context("transaction not yet finalized on Solana")?;

    // Fail closed: if the caller specified an expected genesis hash, the
    // observed cluster identity must match before any artifact is minted.
    match (&params.expected_sol_genesis, &obs.genesis_hash) {
        (Some(expected), Some(observed)) if observed != expected => {
            bail!(
                "Solana genesis hash mismatch: expected '{}', observed '{}'",
                expected,
                observed
            );
        }
        (Some(expected), None) => {
            bail!(
                "Solana genesis hash unavailable from node; expected '{}'",
                expected
            );
        }
        _ => {}
    }

    let event = map_to_arrow(&obs);
    let tx_ref = TxRef::sol(&obs.signature);
    let chain_id_observed = obs
        .genesis_hash
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let core = ArtifactCore {
        artifact_version: "arrow-lifecycle-artifact-v0".to_string(),
        registry_code: "SOL".to_string(),
        entry_version: params.entry_version.clone(),
        registry_version: params.registry_version.clone(),
        network: params.network.clone(),
        network_name: params.network_name.clone(),
        chain_family: ChainFamily::Svm.as_str().to_string(),
        chain_id_observed,
        tx_ref,
        finality_model: "bft_consensus".to_string(),
        finality_basis: event.finality_basis.clone(),
        arrow_code: event.arrow_code.clone(),
        maps_to_fields: params.maps_to_fields.clone(),
        value: Some(obs.lamports.to_string()),
        settlement_chain: None,
    };

    let observation = ArtifactObservation {
        observed_at: Utc::now().to_rfc3339(),
        observer_head: Some(obs.slot),
        confirmations_at_observation: None,
        finality_timing: None,
    };

    build_artifact(core, observation)
}

fn evm_finality_model(registry_code: &str) -> String {
    match registry_code {
        "ETH" => "probabilistic".to_string(),
        "BASE" | "ARB" => "optimistic_rollup".to_string(),
        _ => "probabilistic".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_family_as_str() {
        assert_eq!(ChainFamily::Evm.as_str(), "evm");
        assert_eq!(ChainFamily::Utxo.as_str(), "utxo");
        assert_eq!(ChainFamily::Svm.as_str(), "svm");
    }
}
