#!/usr/bin/env zsh
# RFC-006 §5.4 / R-009 lifecycle test — Solana mainnet-beta.
# ORIGINATOR-SIDE: spend-capable, human-gated. Never run from an agent/MCP surface.
# If no local Solana signer is configured, the script prints send instructions and
# accepts a finalized signature as its argument (observe-only path).
set -euo pipefail

repo="${ARROW_REPO:-$(cd "$(dirname "$0")/../.." && pwd)}"
rpc="${ARROW_SOL_MAINNET_RPC:-https://api.mainnet-beta.solana.com}"
recipient="${ARROW_SOL_MAINNET_RECIPIENT:?set ARROW_SOL_MAINNET_RECIPIENT to the destination address}"
out="${ARROW_SOL_MAINNET_ARTIFACT:-${repo}/docs/test/evidence/arrow-sol-mainnet-lifecycle.json}"
tx="${ARROW_SOL_MAINNET_TX_FINAL:-${1:-}}"
amount="0.000000001"

if ! command -v solana >/dev/null 2>&1; then
  echo "solana is not available on PATH." >&2
  exit 1
fi

if [[ -n "$tx" ]]; then
  echo "Using supplied Solana mainnet signature: ${tx}"
else
  if ! from="$(NO_DNA=1 solana address 2>/dev/null)"; then
    echo "No local Solana CLI signer is configured." >&2
    echo >&2
    echo "Use your wallet UI to send exactly ${amount} SOL on Solana mainnet:" >&2
    echo "  from:      <your operator address>" >&2
    echo "  to:        ${recipient}" >&2
    echo "  amount:    ${amount} SOL (1 lamport)" >&2
    echo "  network:   mainnet-beta" >&2
    echo >&2
    echo "Then rerun this script with the finalized signature:" >&2
    echo "  $0 <SIGNATURE>" >&2
    exit 2
  fi

  balance="$(NO_DNA=1 solana balance "$from" --url "$rpc")"
  genesis="$(NO_DNA=1 solana genesis-hash --url "$rpc")"

  echo "SOL mainnet lifecycle preflight"
  echo "  from:      ${from}"
  echo "  to:        ${recipient}"
  echo "  amount:    ${amount} SOL (1 lamport)"
  echo "  rpc:       ${rpc}"
  echo "  genesis:   ${genesis}"
  echo "  balance:   ${balance}"
  echo
  echo "Type SEND-SOL-MAINNET to broadcast this real mainnet transaction:"
  IFS= read -r approval
  if [[ "$approval" != "SEND-SOL-MAINNET" ]]; then
    echo "Cancelled before broadcast."
    unset approval
    exit 0
  fi

  send_log="$(mktemp /tmp/arrow-sol-mainnet-send.XXXXXX.txt)"
  NO_DNA=1 solana transfer "$recipient" "$amount" \
    --allow-unfunded-recipient \
    --url "$rpc" \
    --output json | tee "$send_log"

  tx="$(sed -n 's/.*"signature"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$send_log" | head -n 1)"
  if [[ -z "$tx" ]]; then
    tx="$(tail -n 1 "$send_log" | tr -d '[:space:]')"
  fi
fi

if [[ -z "$tx" ]]; then
  echo "No Solana signature available." >&2
  exit 1
fi

echo "Confirming ${tx} at finalized commitment..."
NO_DNA=1 solana confirm -v "$tx" --url "$rpc"

mkdir -p "$(dirname "$out")"
(
  cd "$repo"
  cargo run -p arrow-registry-test -- lifecycle run \
    --origin SOL \
    --network mainnet \
    --tx "$tx" \
    --rpc "$rpc" \
    --out "$out"
  cargo run -p arrow-registry-test -- artifact hash "$out"
)

echo "SOL mainnet lifecycle artifact: ${out}"
echo "ARROW_SOL_MAINNET_TX_FINAL=${tx}"
