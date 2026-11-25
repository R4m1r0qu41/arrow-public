# On-Chain Lifecycle Test Runbook

This runbook describes the operational steps for producing real lifecycle-test
evidence for the Chain Registry entries.

Run the testnet/devnet pass first. Do not use testnet evidence as proof for
mainnet `protocol_required` values. Testnet evidence proves the harness,
transaction observation, artifact hashing, and registry backfill mechanics.
Mainnet evidence is required before populating mainnet `chain_registry.json`
entries as RFC-006 graduation evidence.

## 1. Safety Rules

1. Never paste private keys into issues, commits, chat, logs, or artifacts.
2. Keep signing keys in local environment variables or wallet tooling only.
3. Use minimal value transfers.
4. Send only to a maintainer-controlled recipient address.
5. Record transaction hashes, signatures, and BTC outpoints only after the
   transaction is visible on the target network.
6. Do not edit `lifecycle_test` or `anchoring` with synthetic values.
7. Preserve `tx_ref` as the lifecycle source reference:

```json
{
  "type": "evm_tx_hash",
  "value": "0x..."
}
```

Allowed `tx_ref.type` values are:

- `evm_tx_hash`
- `btc_outpoint`
- `sol_signature`

`anchoring.onchain_anchor.tx_hash` is separate. It refers to the anchor
transaction or timestamp commitment, not the lifecycle source transaction.

## 2. Local Prerequisites

Install or confirm:

```sh
cargo --version
node --version
cast --version
solana --version
bitcoin-cli --version
```

`bitcoin-cli` is required for BTC Signet/mainnet observation unless another
Bitcoin Core-compatible RPC client is used.

OpenTimestamps is the RFC-006 v0.5 baseline anchor mechanism. Install an
`ots` CLI before final anchoring, or document the equivalent OpenTimestamps
client used by the maintainer.

Run baseline checks:

```sh
node tools/validate-registries.js
cargo test -p arrow-registry-test
```

## 3. Common Environment Variables

Use local shell exports. Do not commit these values.

```sh
export ARROW_TESTNET_KEY="<hex private key, no 0x>"
export ARROW_TESTNET_RECIPIENT="0x..."

export ARROW_SEPOLIA_RPC="https://..."
export ARROW_BASE_SEPOLIA_RPC="https://..."
export ARROW_ARB_SEPOLIA_RPC="https://..."
export ARROW_SOL_DEVNET_RPC="https://api.devnet.solana.com"

export ARROW_BTC_SIGNET_RPC="http://127.0.0.1:38332"
export ARROW_BTC_SIGNET_RPC_USER="..."
export ARROW_BTC_SIGNET_RPC_PASS="..."
```

For mainnet, use separate variables so testnet and mainnet credentials are not
confused:

```sh
export ARROW_MAINNET_KEY="<hex private key, no 0x>"
export ARROW_MAINNET_RECIPIENT="0x..."

export ARROW_ETH_MAINNET_RPC="https://..."
export ARROW_BASE_MAINNET_RPC="https://..."
export ARROW_ARB_MAINNET_RPC="https://..."
export ARROW_SOL_MAINNET_RPC="https://api.mainnet-beta.solana.com"

export ARROW_BTC_MAINNET_RPC="http://127.0.0.1:8332"
export ARROW_BTC_MAINNET_RPC_USER="..."
export ARROW_BTC_MAINNET_RPC_PASS="..."
```

## 4. Testnet / Devnet Pass

### 4.1 ETH Sepolia

Send a 1 wei transfer:

```sh
cast send \
  --rpc-url "$ARROW_SEPOLIA_RPC" \
  --private-key "$ARROW_TESTNET_KEY" \
  "$ARROW_TESTNET_RECIPIENT" \
  --value 1wei
```

Record the returned transaction hash:

```sh
export ARROW_ETH_SEPOLIA_TX_FINAL="0x..."
```

Wait until the transaction has at least 12 confirmations. Then generate the
artifact:

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin ETH \
  --network sepolia \
  --tx "$ARROW_ETH_SEPOLIA_TX_FINAL" \
  --rpc "$ARROW_SEPOLIA_RPC" \
  --out /tmp/arrow-eth-sepolia-lifecycle.json
```

Verify the artifact hash:

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-eth-sepolia-lifecycle.json
```

Expected Arrow code: `1:ETH:FN:PR:1:A`.

### 4.2 Base Sepolia

Send a 1 wei transfer:

```sh
cast send \
  --rpc-url "$ARROW_BASE_SEPOLIA_RPC" \
  --private-key "$ARROW_TESTNET_KEY" \
  "$ARROW_TESTNET_RECIPIENT" \
  --value 1wei
```

Record the returned transaction hash:

```sh
export ARROW_BASE_SEPOLIA_TX_PENDING="0x..."
```

Generate the artifact:

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin BASE \
  --network base-sepolia \
  --tx "$ARROW_BASE_SEPOLIA_TX_PENDING" \
  --rpc "$ARROW_BASE_SEPOLIA_RPC" \
  --out /tmp/arrow-base-sepolia-lifecycle.json
```

Verify the artifact hash:

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-base-sepolia-lifecycle.json
```

Expected Arrow code: `1:BSE:PD:HL:0:B`.

Base is an optimistic L2. The lifecycle observation should remain pending with
`finality_basis = optimistic_window`; do not claim mainnet challenge-window
truth from Base Sepolia.

### 4.3 Arbitrum Sepolia

Send a 1 wei transfer:

```sh
cast send \
  --rpc-url "$ARROW_ARB_SEPOLIA_RPC" \
  --private-key "$ARROW_TESTNET_KEY" \
  "$ARROW_TESTNET_RECIPIENT" \
  --value 1wei
```

Record the returned transaction hash:

```sh
export ARROW_ARB_SEPOLIA_TX_PENDING="0x..."
```

Generate the artifact:

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin ARB \
  --network arb-sepolia \
  --tx "$ARROW_ARB_SEPOLIA_TX_PENDING" \
  --rpc "$ARROW_ARB_SEPOLIA_RPC" \
  --out /tmp/arrow-arb-sepolia-lifecycle.json
```

Verify the artifact hash:

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-arb-sepolia-lifecycle.json
```

Expected Arrow code: `1:ARB:PD:HL:0:B`.

### 4.4 Solana Devnet

Use a funded devnet keypair. Confirm the target cluster:

```sh
solana config set --url "$ARROW_SOL_DEVNET_RPC"
solana balance
```

Send a minimal lamport transfer to a maintainer-controlled recipient:

```sh
solana transfer <SOL_DEVNET_RECIPIENT_PUBKEY> 0.000000001 \
  --allow-unfunded-recipient \
  --url "$ARROW_SOL_DEVNET_RPC"
```

Record the returned signature:

```sh
export ARROW_SOL_DEVNET_TX_FINAL="..."
```

Wait until the signature is finalized:

```sh
solana confirm -v "$ARROW_SOL_DEVNET_TX_FINAL" --url "$ARROW_SOL_DEVNET_RPC"
```

Generate the artifact:

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin SOL \
  --network devnet \
  --tx "$ARROW_SOL_DEVNET_TX_FINAL" \
  --rpc "$ARROW_SOL_DEVNET_RPC" \
  --out /tmp/arrow-sol-devnet-lifecycle.json
```

Verify the artifact hash:

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-sol-devnet-lifecycle.json
```

Expected Arrow code: `1:SOL:FN:PR:1:A`.

### 4.5 Bitcoin Signet

Use Bitcoin Core on Signet. Confirm the node identity:

```sh
bitcoin-cli -signet \
  -rpcuser="$ARROW_BTC_SIGNET_RPC_USER" \
  -rpcpassword="$ARROW_BTC_SIGNET_RPC_PASS" \
  getblockchaininfo
```

Send a small non-dust output to a maintainer-controlled Signet address:

```sh
bitcoin-cli -signet \
  -rpcuser="$ARROW_BTC_SIGNET_RPC_USER" \
  -rpcpassword="$ARROW_BTC_SIGNET_RPC_PASS" \
  sendtoaddress "<SIGNET_RECIPIENT_ADDRESS>" 0.00001000
```

Record the returned txid:

```sh
export ARROW_BTC_SIGNET_TXID="..."
```

Find the relevant output index:

```sh
bitcoin-cli -signet \
  -rpcuser="$ARROW_BTC_SIGNET_RPC_USER" \
  -rpcpassword="$ARROW_BTC_SIGNET_RPC_PASS" \
  getrawtransaction "$ARROW_BTC_SIGNET_TXID" true
```

Set the output index:

```sh
export ARROW_BTC_SIGNET_VOUT="0"
```

Wait for at least 6 confirmations. Then generate the artifact:

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin BTC \
  --network signet \
  --tx "$ARROW_BTC_SIGNET_TXID" \
  --rpc "$ARROW_BTC_SIGNET_RPC" \
  --btc-vout "$ARROW_BTC_SIGNET_VOUT" \
  --btc-user "$ARROW_BTC_SIGNET_RPC_USER" \
  --btc-pass "$ARROW_BTC_SIGNET_RPC_PASS" \
  --out /tmp/arrow-btc-signet-lifecycle.json
```

Verify the artifact hash:

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-btc-signet-lifecycle.json
```

Expected Arrow code: `1:BTC:FN:PR:1:A`.

## 5. Testnet Registry Backfill

After the artifacts are generated, update `registries/chain_registry.testnet.json`
only.

For each populated entry:

1. Set `lifecycle_test.test_type` to `minimal_value_transfer`.
2. Set `lifecycle_test.performed_by` to the maintainer role or approved delegate.
3. Copy `core.tx_ref` from the artifact into `lifecycle_test.tx_ref`.
4. Copy `observation.observed_at` into `lifecycle_test.observed_at`.
5. Copy `core.maps_to_fields` into `lifecycle_test.maps_to_fields`.
6. Copy `extraction_artifact_hash` into
   `lifecycle_test.extraction_artifact_hash`.
7. Set `lifecycle_test.next_state_test_due` from the entry cadence.

Example:

```json
"lifecycle_test": {
  "test_type": "minimal_value_transfer",
  "performed_by": "Arrow Registry Maintainer",
  "tx_ref": {
    "type": "evm_tx_hash",
    "value": "0x..."
  },
  "observed_at": "2026-06-08T00:00:00Z",
  "maps_to_fields": [
    "code",
    "protocol_required.finality_model",
    "protocol_required.finality_basis_default",
    "protocol_required.reorg_safety_confirmations"
  ],
  "extraction_artifact_hash": "sha256:...",
  "next_state_test_due": "2026-07-08"
}
```

Then compute hashes:

```sh
cargo run -p arrow-registry-test -- anchor hash \
  registries/chain_registry.testnet.json \
  --entries
```

Compute the hashes over a copy of the file with every `anchoring.entry_hash`
and `anchoring.registry_version_hash` set to `null` (the R-004 canonical-hash
preimage rule — the `anchor hash` CLI does not null them, so feed it a nulled
copy). Populate each entry's `anchoring.entry_hash` from the matching
`entry_hash[CODE]` output, and `anchoring.registry_version_hash` (identical
across entries) from the top-level `registry_version_hash` output. The result is
a fixed point: a verifier nulls those two fields and recomputes.

Create an OpenTimestamps proof over the resulting registry file (`ots stamp
<file>`). **Leave `anchoring.onchain_anchor` `null`** — writing the anchor back
into the registry would change the bytes the proof commits to and invalidate
both the proof and the stored hashes (R-004). Record the anchor commitment
out-of-file instead: keep the sidecar `.ots` proof and log its file digest and
Bitcoin block in the on-chain lifecycle evidence log. If an Ethereum calldata
anchor is used as supplemental evidence, record it the same way (out-of-file);
do not replace the required OTS proof.

Run:

```sh
node tools/validate-registries.js
cargo test -p arrow-registry-test
```

## 6. Mainnet Pass

Mainnet evidence is required for `registries/chain_registry.json`.

Current implementation note: the `arrow-registry-test` CLI currently resolves
the supported public test networks (`sepolia`, `base-sepolia`, `arb-sepolia`,
`devnet`, `signet`). Before mainnet backfill, add mainnet resolver support or
use an equivalent maintainer harness that builds the same `ExtractionArtifact`
schema with the correct fail-closed chain identity checks.

Required mainnet identity checks:

- ETH mainnet EIP-155 chain id: `1`
- Base mainnet EIP-155 chain id: `8453`
- Arbitrum One EIP-155 chain id: `42161`
- Solana mainnet-beta genesis hash: must match the configured mainnet cluster
- Bitcoin mainnet chain identity: `main`

### 6.1 ETH Mainnet

Send a 1 wei transfer only after confirming gas cost and recipient:

```sh
cast send \
  --rpc-url "$ARROW_ETH_MAINNET_RPC" \
  --private-key "$ARROW_MAINNET_KEY" \
  "$ARROW_MAINNET_RECIPIENT" \
  --value 1wei
```

Record:

```sh
export ARROW_ETH_MAINNET_TX_FINAL="0x..."
```

Wait for at least 12 confirmations. Generate an artifact with the mainnet-capable
harness. Expected Arrow code: `1:ETH:FN:PR:1:A`.

### 6.2 Base Mainnet

Send a 1 wei transfer:

```sh
cast send \
  --rpc-url "$ARROW_BASE_MAINNET_RPC" \
  --private-key "$ARROW_MAINNET_KEY" \
  "$ARROW_MAINNET_RECIPIENT" \
  --value 1wei
```

Record:

```sh
export ARROW_BASE_MAINNET_TX_PENDING="0x..."
```

Generate an artifact with the mainnet-capable harness. Expected Arrow code:
`1:BSE:PD:HL:0:B` until the optimistic window is settled.

### 6.3 Arbitrum One Mainnet

Send a 1 wei transfer:

```sh
cast send \
  --rpc-url "$ARROW_ARB_MAINNET_RPC" \
  --private-key "$ARROW_MAINNET_KEY" \
  "$ARROW_MAINNET_RECIPIENT" \
  --value 1wei
```

Record:

```sh
export ARROW_ARB_MAINNET_TX_PENDING="0x..."
```

Generate an artifact with the mainnet-capable harness. Expected Arrow code:
`1:ARB:PD:HL:0:B` until the optimistic window is settled.

### 6.4 Solana Mainnet-Beta

Use a funded mainnet keypair and a maintainer-controlled recipient.

```sh
solana config set --url "$ARROW_SOL_MAINNET_RPC"
solana transfer <SOL_MAINNET_RECIPIENT_PUBKEY> 0.000000001 \
  --url "$ARROW_SOL_MAINNET_RPC"
```

Record:

```sh
export ARROW_SOL_MAINNET_TX_FINAL="..."
```

Wait for finalized commitment:

```sh
solana confirm -v "$ARROW_SOL_MAINNET_TX_FINAL" \
  --url "$ARROW_SOL_MAINNET_RPC"
```

Generate an artifact with the mainnet-capable harness. Expected Arrow code:
`1:SOL:FN:PR:1:A`.

### 6.5 Bitcoin Mainnet

Use a small non-dust output, not a one-satoshi output. Confirm fee rate and UTXO
selection before broadcasting.

```sh
bitcoin-cli \
  -rpcuser="$ARROW_BTC_MAINNET_RPC_USER" \
  -rpcpassword="$ARROW_BTC_MAINNET_RPC_PASS" \
  sendtoaddress "<BTC_MAINNET_RECIPIENT_ADDRESS>" 0.00001000
```

Record:

```sh
export ARROW_BTC_MAINNET_TXID="..."
```

Find the relevant output index:

```sh
bitcoin-cli \
  -rpcuser="$ARROW_BTC_MAINNET_RPC_USER" \
  -rpcpassword="$ARROW_BTC_MAINNET_RPC_PASS" \
  getrawtransaction "$ARROW_BTC_MAINNET_TXID" true
```

Set:

```sh
export ARROW_BTC_MAINNET_VOUT="0"
```

Wait for at least 6 confirmations. Generate an artifact with the
mainnet-capable harness. Expected Arrow code: `1:BTC:FN:PR:1:A`.

## 7. Mainnet Registry Backfill

After all five mainnet artifacts are generated:

1. Update `registries/chain_registry.json`.
2. Populate each `lifecycle_test` from the matching mainnet artifact.
3. Keep `provenance.state_test_cadence` at `30d` unless a separate registry
   change approves a different cadence.
4. Compute registry and entry hashes:

```sh
cargo run -p arrow-registry-test -- anchor hash \
  registries/chain_registry.json \
  --entries
```

5. Populate `anchoring.entry_hash` and `anchoring.registry_version_hash` from
   that output. Compute the hashes over a copy with both fields set to `null`
   first (R-004 canonical-hash preimage rule; the CLI does not null them). The
   values are then a fixed point: null the two fields and recompute to verify.
6. Create the required OpenTimestamps proof over the registry file
   (`ots stamp registries/chain_registry.json`).
7. **Leave `anchoring.onchain_anchor` `null`.** Record the anchor commitment
   out-of-file — the sidecar `.ots` proof plus its file digest and Bitcoin block
   in the on-chain lifecycle evidence log. Populating `onchain_anchor` in-file
   would change the bytes the proof commits to and invalidate the proof and the
   stored hashes (R-004).
8. Run validation and focused tests:

```sh
node tools/validate-registries.js
cargo test -p arrow-registry-test
cargo test -p arrow-core --test fixtures
```

For protocol-wide publication, run the broader workspace test if time permits:

```sh
cargo test --workspace
```

## 8. Evidence Checklist

For each chain, retain:

- Network name and chain identity observed by the harness.
- Transaction hash, signature, or BTC outpoint.
- Confirmation or finalized status at observation time.
- Full extraction artifact JSON.
- `extraction_artifact_hash`.
- Registry entry hash after lifecycle backfill.
- Registry version hash after lifecycle backfill.
- OpenTimestamps proof or committed timestamp evidence.
- Validation output from `node tools/validate-registries.js`.
- Relevant Cargo test output.

The final commit should include registry updates and any code or documentation
needed to reproduce the evidence. It should not include private keys, wallet
files, RPC credentials, or unredacted secret-bearing shell history.
