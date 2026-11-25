#!/usr/bin/env zsh
# Start a pruned Bitcoin Core mainnet node for BTC lifecycle observation.
# Paths are environment-driven (no operator-specific defaults baked in).
set -euo pipefail

conf="${ARROW_BTC_MAINNET_CONF:?set ARROW_BTC_MAINNET_CONF to your bitcoin.conf path}"
datadir="${ARROW_BTC_MAINNET_DATADIR:?set ARROW_BTC_MAINNET_DATADIR to your data directory}"
rpcport="${ARROW_BTC_MAINNET_RPCPORT:-28335}"

mkdir -p "$datadir"

if bitcoin-cli -datadir="$datadir" -conf="$conf" getblockchaininfo >/dev/null 2>&1; then
  echo "Stopping currently running Bitcoin Core node..."
  bitcoin-cli -datadir="$datadir" -conf="$conf" stop
  echo "Waiting for shutdown..."
  while bitcoin-cli -datadir="$datadir" -conf="$conf" getblockchaininfo >/dev/null 2>&1; do
    sleep 2
  done
fi

echo "Starting Bitcoin Core mainnet in pruned mode."
echo "Config: ${conf}"
echo "Data dir: ${datadir}"
echo "Pruned mode stores ~7GB of block/undo data but still downloads and verifies mainnet history during IBD."
bitcoind -datadir="$datadir" -conf="$conf" -daemon
echo "Started. Check progress with:"
echo "  bitcoin-cli -datadir='${datadir}' -conf='${conf}' -rpcport=${rpcport} -getinfo"
