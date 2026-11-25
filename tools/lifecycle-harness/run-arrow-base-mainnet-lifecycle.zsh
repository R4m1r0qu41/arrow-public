#!/usr/bin/env zsh
# Thin wrapper: BASE mainnet lifecycle test via the generalized EVM runner.
exec "$(dirname "$0")/run-arrow-evm-mainnet-lifecycle.zsh" BASE
