//! Endpoint and chain-identity expectations for lifecycle testing.
//!
//! These constants are used by the lifecycle harness to locate public
//! RPC endpoints and verify the observed chain id before minting an artifact.
//!
//! **Important:** testnet parameters must NOT be cited as evidence for mainnet
//! `protocol_required` values. Artifacts carry an explicit `network` scope
//! (`"testnet"` or `"mainnet"`) to prevent accidental derivation.

/// Known EIP-155 chain IDs for supported EVM networks.
pub mod chain_ids {
    pub const ETH_MAINNET: u64 = 1;
    pub const ETH_SEPOLIA: u64 = 11155111;
    pub const BASE_MAINNET: u64 = 8453;
    pub const BASE_SEPOLIA: u64 = 84532;
    pub const ARB_MAINNET: u64 = 42161;
    pub const ARB_SEPOLIA: u64 = 421614;
}

/// Well-known Solana cluster genesis hashes.
pub mod sol_genesis {
    /// Solana devnet genesis hash (stable identifier for the devnet cluster).
    pub const DEVNET: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";
    /// Solana mainnet-beta genesis hash.
    pub const MAINNET_BETA: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
    /// Solana testnet genesis hash.
    pub const TESTNET: &str = "4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY";
}

/// Bitcoin Core `getblockchaininfo.chain` values for known networks.
pub mod btc_chain {
    pub const MAINNET: &str = "main";
    pub const SIGNET: &str = "signet";
    pub const TESTNET4: &str = "test";
    pub const REGTEST: &str = "regtest";
}

/// Default public RPC endpoints (overridable via env vars or CLI `--rpc`).
pub mod default_rpc {
    /// Ethereum mainnet. Prefer `--rpc` or `ARROW_ETH_MAINNET_RPC` in scripts.
    pub const ETH_MAINNET: &str = "https://ethereum-rpc.publicnode.com";
    /// Ethereum Sepolia. Override with `ARROW_SEPOLIA_RPC`.
    pub const ETH_SEPOLIA: &str = "https://rpc.sepolia.org";
    /// Base mainnet. Prefer `--rpc` or `ARROW_BASE_MAINNET_RPC` in scripts.
    pub const BASE_MAINNET: &str = "https://mainnet.base.org";
    /// Base Sepolia. Override with `ARROW_BASE_SEPOLIA_RPC`.
    pub const BASE_SEPOLIA: &str = "https://sepolia.base.org";
    /// Arbitrum One mainnet. Prefer `--rpc` or `ARROW_ARB_MAINNET_RPC` in scripts.
    pub const ARB_MAINNET: &str = "https://arb1.arbitrum.io/rpc";
    /// Arbitrum Sepolia. Override with `ARROW_ARB_SEPOLIA_RPC`.
    pub const ARB_SEPOLIA: &str = "https://sepolia-rollup.arbitrum.io/rpc";
    /// Solana mainnet-beta. Prefer `--rpc` or `ARROW_SOL_MAINNET_RPC` in scripts.
    pub const SOL_MAINNET: &str = "https://api.mainnet-beta.solana.com";
    /// Solana devnet. Override with `ARROW_SOL_DEVNET_RPC`.
    pub const SOL_DEVNET: &str = "https://api.devnet.solana.com";
}

/// Resolve a testnet RPC URL from env or fall back to the built-in default.
pub fn resolve_rpc(env_var: &str, default: &str) -> String {
    std::env::var(env_var).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sepolia_chain_id_is_correct() {
        assert_eq!(chain_ids::ETH_SEPOLIA, 11155111);
    }

    #[test]
    fn mainnet_chain_ids_are_correct() {
        assert_eq!(chain_ids::ETH_MAINNET, 1);
        assert_eq!(chain_ids::BASE_MAINNET, 8453);
        assert_eq!(chain_ids::ARB_MAINNET, 42161);
    }

    #[test]
    fn base_sepolia_chain_id_is_correct() {
        assert_eq!(chain_ids::BASE_SEPOLIA, 84532);
    }

    #[test]
    fn arb_sepolia_chain_id_is_correct() {
        assert_eq!(chain_ids::ARB_SEPOLIA, 421614);
    }
}
