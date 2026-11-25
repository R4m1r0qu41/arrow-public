#!/usr/bin/env zsh
# Start a Bitcoin Core signet node for BTC testnet lifecycle work.
# Paths are environment-driven (no operator-specific defaults baked in).
set -euo pipefail

conf="${ARROW_BTC_SIGNET_CONF:?set ARROW_BTC_SIGNET_CONF to your signet bitcoin.conf path}"
datadir="${ARROW_BTC_SIGNET_DATADIR:?set ARROW_BTC_SIGNET_DATADIR to your signet data directory}"

# If a mainnet pruned node is running, stop it first (single Bitcoin Core instance per machine).
mainnet_conf="${ARROW_BTC_MAINNET_CONF:-}"
mainnet_datadir="${ARROW_BTC_MAINNET_DATADIR:-}"

mkdir -p "$datadir"

if [[ -n "$mainnet_conf" && -n "$mainnet_datadir" ]]; then
  if bitcoin-cli -datadir="$mainnet_datadir" -conf="$mainnet_conf" getblockchaininfo >/dev/null 2>&1; then
    echo "Stopping currently running Bitcoin Core mainnet node..."
    bitcoin-cli -datadir="$mainnet_datadir" -conf="$mainnet_conf" stop
    echo "Waiting for shutdown..."
    while bitcoin-cli -datadir="$mainnet_datadir" -conf="$mainnet_conf" getblockchaininfo >/dev/null 2>&1; do
      sleep 2
    done
  fi
fi

echo "Starting Bitcoin Core signet."
echo "Config: ${conf}"
echo "Data dir: ${datadir}"
bitcoind -datadir="$datadir" -conf="$conf" -daemon
echo "Started. Check progress with:"
echo "  bitcoin-cli -datadir='${datadir}' -conf='${conf}' -getinfo"
