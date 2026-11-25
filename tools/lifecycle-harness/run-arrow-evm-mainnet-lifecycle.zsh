#!/usr/bin/env zsh
# RFC-006 §5.4 / R-009 lifecycle test — EVM mainnet (ETH / BASE / ARB).
# ORIGINATOR-SIDE: spend-capable, human-gated. Never run from an agent/MCP surface.
# Addresses and keys are supplied by environment only — nothing operator-specific is hardcoded.
set -euo pipefail

repo="${ARROW_REPO:-$(cd "$(dirname "$0")/../.." && pwd)}"
origin="${1:-}"

# Required: the operator (from) address whose key will sign. The script verifies the
# pasted/exported key derives to exactly this address before it will broadcast.
expected_from="${ARROW_MAINNET_FROM:?set ARROW_MAINNET_FROM to the operator (sending) address}"
recipient="${ARROW_MAINNET_RECIPIENT:?set ARROW_MAINNET_RECIPIENT to the destination address}"

if [[ -z "$origin" ]]; then
  echo "Network to test? Type ETH, BASE, or ARB:"
  IFS= read -r origin
fi

origin="${origin:u}"

case "$origin" in
  ETH)
    rpc="${ARROW_ETH_MAINNET_RPC:-https://ethereum-rpc.publicnode.com}"
    expected_chain_id="1"
    confirmations_required="12"
    out="${ARROW_ETH_MAINNET_ARTIFACT:-${repo}/docs/test/evidence/arrow-eth-mainnet-lifecycle.json}"
    approval_phrase="SEND-ETH-MAINNET"
    ;;
  BASE)
    rpc="${ARROW_BASE_MAINNET_RPC:-https://mainnet.base.org}"
    expected_chain_id="8453"
    confirmations_required="1"
    out="${ARROW_BASE_MAINNET_ARTIFACT:-${repo}/docs/test/evidence/arrow-base-mainnet-lifecycle.json}"
    approval_phrase="SEND-BASE-MAINNET"
    ;;
  ARB)
    rpc="${ARROW_ARB_MAINNET_RPC:-https://arb1.arbitrum.io/rpc}"
    expected_chain_id="42161"
    confirmations_required="1"
    out="${ARROW_ARB_MAINNET_ARTIFACT:-${repo}/docs/test/evidence/arrow-arb-mainnet-lifecycle.json}"
    approval_phrase="SEND-ARB-MAINNET"
    ;;
  *)
    echo "Usage: $0 ETH|BASE|ARB" >&2
    exit 2
    ;;
esac

if ! command -v cast >/dev/null 2>&1; then
  echo "cast is not available on PATH." >&2
  exit 1
fi

if [[ -n "${ARROW_MAINNET_KEY:-}" ]]; then
  key="$(printf '%s' "$ARROW_MAINNET_KEY" | tr -d '[:space:]')"
else
  echo -n "Paste private key for ${expected_from}, then press Enter: "
  stty -echo
  IFS= read -r raw_key
  stty echo
  echo
  key="$(printf '%s' "$raw_key" | tr -d '[:space:]')"
fi

key="${key#0x}"
key="${key#0X}"

if [[ ${#key} -ne 64 ]]; then
  echo "Invalid private key length after cleanup: ${#key}" >&2
  echo "Expected 64 hex characters. The key was not printed." >&2
  unset key raw_key
  exit 1
fi

if [[ ! "$key" =~ '^[0-9A-Fa-f]{64}$' ]]; then
  echo "Invalid private key characters. Expected only 0-9, a-f, A-F." >&2
  echo "The key was not printed." >&2
  unset key raw_key
  exit 1
fi

derived="$(cast wallet address "0x${key}")"
if [[ "${derived:l}" != "${expected_from:l}" ]]; then
  echo "MISMATCH: derived ${derived}, expected ${expected_from}." >&2
  echo "The key was not used." >&2
  unset key raw_key derived
  exit 1
fi

chain_id="$(cast chain-id --rpc-url "$rpc")"
if [[ "$chain_id" != "$expected_chain_id" ]]; then
  echo "MISMATCH: RPC chain id ${chain_id}, expected ${expected_chain_id}." >&2
  unset key raw_key
  exit 1
fi

balance="$(cast balance "$expected_from" --rpc-url "$rpc" --ether)"
gas_price="$(cast gas-price --rpc-url "$rpc")"
gas_estimate="$(cast estimate "$recipient" --value 1wei --rpc-url "$rpc")"

echo "${origin} mainnet lifecycle preflight"
echo "  from:      ${expected_from}"
echo "  to:        ${recipient}"
echo "  value:     1 wei"
echo "  rpc:       ${rpc}"
echo "  chain id:  ${chain_id}"
echo "  balance:   ${balance} ETH"
echo "  gas price: ${gas_price} wei"
echo "  gas limit: ${gas_estimate}"
echo
echo "Type ${approval_phrase} to broadcast this real mainnet transaction:"
IFS= read -r approval
if [[ "$approval" != "$approval_phrase" ]]; then
  echo "Cancelled before broadcast."
  unset key raw_key approval
  exit 0
fi

send_log="$(mktemp "/tmp/arrow-${origin:l}-mainnet-send.XXXXXX.txt")"
cast send \
  --rpc-url "$rpc" \
  --private-key "0x${key}" \
  "$recipient" \
  --value 1wei | tee "$send_log"

tx_hash="$(awk '/transactionHash/{print $2}' "$send_log")"
tx_block="$(awk '/blockNumber/{print $2}' "$send_log")"
if [[ -z "$tx_hash" || -z "$tx_block" ]]; then
  echo "Could not parse transaction hash/block from ${send_log}" >&2
  unset key raw_key
  exit 1
fi

echo "Waiting for ${confirmations_required} confirmation(s) for ${tx_hash}..."
while true; do
  current_block="$(cast block-number --rpc-url "$rpc")"
  confirmations=$(( current_block - tx_block + 1 ))
  if (( confirmations >= confirmations_required )); then
    break
  fi
  echo "  confirmations: ${confirmations}/${confirmations_required}"
  sleep 3
done

mkdir -p "$(dirname "$out")"
(
  cd "$repo"
  cargo run -p arrow-registry-test -- lifecycle run \
    --origin "$origin" \
    --network mainnet \
    --tx "$tx_hash" \
    --rpc "$rpc" \
    --out "$out"
  cargo run -p arrow-registry-test -- artifact hash "$out"
)

unset key raw_key approval
echo "${origin} mainnet lifecycle artifact: ${out}"
echo "ARROW_${origin}_MAINNET_TX=${tx_hash}"
