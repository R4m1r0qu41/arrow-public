# Lifecycle Test Harness

Reference operator harness for the **RFC-006 §5.4 / R-009 / R-010** on-chain lifecycle
tests: send a minimal native-asset transfer on a real network, observe it, and emit the
Arrow-native R-011 extraction artifact via `arrow-registry-test`.

These scripts are **scrubbed and environment-driven** — no operator addresses, keys, or
machine paths are baked in. They are the committed reference. Operator-specific copies
(with real addresses) live outside version control under `tools/lifecycle-tests/`
(gitignored).

## The originator / observer boundary

The harness deliberately splits along the protocol's own role separation:

| Role | What it does | Surface |
|---|---|---|
| **Originator** | Spend-capable. Signs and broadcasts the lifecycle transaction. | Human-gated CLI only. **Never** an agent/MCP surface — a spend key must not flow through an automated relay. Every broadcast requires typing an explicit approval phrase. |
| **Observer** | Read-only. Verifies the transaction, derives the Arrow event, hashes the artifact, monitors `next_state_test_due`. | Safe to expose to agents / MCP / dashboards — carries zero spend authority. |

This mirrors the watch-only vs. key-bearing wallet split used for BTC (an observer must
never require spend authority). If you build agent or MCP tooling on top of this, wire the
**observer half only**; keep the originator half behind the human approval gate.

## Scripts

| Script | Purpose |
|---|---|
| `run-arrow-evm-mainnet-lifecycle.zsh` | Generalized EVM runner — `ETH` / `BASE` / `ARB`. Validates the key derives to the expected address, checks chain id, previews gas/balance, requires an approval phrase, broadcasts 1 wei, waits for confirmations, emits + hashes the artifact. |
| `run-arrow-arb-mainnet-lifecycle.zsh` | Thin wrapper → EVM runner with `ARB`. |
| `run-arrow-base-mainnet-lifecycle.zsh` | Thin wrapper → EVM runner with `BASE`. |
| `run-arrow-sol-mainnet-lifecycle.zsh` | Solana mainnet-beta runner. Falls back to an observe-only path (accepts a finalized signature) when no local signer is configured. |
| `start-bitcoin-mainnet-pruned.zsh` | Start a pruned Bitcoin Core mainnet node for BTC observation. |
| `start-bitcoin-signet.zsh` | Start a Bitcoin Core signet node for testnet work. |

## Environment

EVM (required):

```
ARROW_MAINNET_FROM        operator (sending) address — the key is verified to derive to this
ARROW_MAINNET_RECIPIENT   destination address
```

EVM (optional, with sane defaults): `ARROW_ETH_MAINNET_RPC`, `ARROW_BASE_MAINNET_RPC`,
`ARROW_ARB_MAINNET_RPC`, `ARROW_*_MAINNET_ARTIFACT`. The signing key is supplied via
`ARROW_MAINNET_KEY` or pasted at the prompt (never echoed, never logged, unset after use).

Solana: `ARROW_SOL_MAINNET_RECIPIENT` (required); `ARROW_SOL_MAINNET_RPC`,
`ARROW_SOL_MAINNET_ARTIFACT` (optional).

Bitcoin: `ARROW_BTC_MAINNET_CONF`, `ARROW_BTC_MAINNET_DATADIR` (+ `ARROW_BTC_MAINNET_RPCPORT`,
default `28335`); `ARROW_BTC_SIGNET_CONF`, `ARROW_BTC_SIGNET_DATADIR`.

`ARROW_REPO` overrides repo-root autodetection (defaults to two levels up from the script).

## BTC note (pruned node)

A pruned mainnet node has no `txindex`, so the observer resolves confirmed transactions
through a **watch-only wallet** via the `/wallet/<name>` RPC path. See the BTC lifecycle
operator notes for the wallet split and the exact `lifecycle run` invocation.
