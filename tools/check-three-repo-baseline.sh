#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: tools/check-three-repo-baseline.sh [--fast|--full]

Verifies the local three-repository Arrow/Orlog/Tarquin baseline from the
expected sibling checkout layout:

  Dev/
    Arrow/
    Orlog/
    Tarquin/

--fast  Runs structural, registry, compatibility-pin, Arrow fmt, and Tarquin parity checks.
--full  Runs --fast plus Arrow cargo check/test/clippy. This includes
        local listener tests that bind 127.0.0.1 and may be blocked by
        restricted sandboxes; run in a normal local shell or CI worker.
USAGE
}

MODE="fast"
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
elif [[ "${1:-}" == "--full" ]]; then
  MODE="full"
elif [[ "${1:-}" == "--fast" || "${1:-}" == "" ]]; then
  MODE="fast"
else
  usage >&2
  exit 2
fi

ARROW_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEV_DIR="$(dirname "${ARROW_DIR}")"
ORLOG_DIR="${DEV_DIR}/Orlog"
TARQUIN_DIR="${DEV_DIR}/Tarquin"

require_dir() {
  local dir="$1"
  local label="$2"
  if [[ ! -e "${dir}/.git" ]]; then
    echo "missing ${label} git checkout at ${dir}" >&2
    exit 1
  fi
}

run() {
  local label="$1"
  shift
  echo "==> ${label}"
  "$@"
}

require_dir "${ARROW_DIR}" "Arrow"
require_dir "${ORLOG_DIR}" "Orlog"
require_dir "${TARQUIN_DIR}" "Tarquin"

run "Arrow registry and rule-surface validation" \
  node "${ARROW_DIR}/tools/validate-registries.js"

run "Orlog structural and Arrow compatibility validation" \
  npm --prefix "${ORLOG_DIR}" run check

run "Arrow format check" \
  bash -c 'cd "$1" && cargo fmt --all --check' bash "${ARROW_DIR}"

run "Tarquin format check" \
  cargo fmt --manifest-path "${TARQUIN_DIR}/Cargo.toml" --all --check

run "Tarquin Arrow rule-surface pin check" \
  cargo test --manifest-path "${TARQUIN_DIR}/Cargo.toml" -p tarquin-lex-exe --test arrow_rule_surface_pin

run "Tarquin cross-target parity" \
  bash -c 'cd "$1" && bash tests/parity/run.sh' bash "${TARQUIN_DIR}"

if [[ "${MODE}" == "full" ]]; then
  run "Arrow workspace check" \
    bash -c 'cd "$1" && cargo check --workspace --all-targets' bash "${ARROW_DIR}"
  run "Arrow workspace tests" \
    bash -c 'cd "$1" && cargo test --workspace --all-targets' bash "${ARROW_DIR}"
  run "Arrow clippy" \
    bash -c 'cd "$1" && cargo clippy --workspace --all-targets --all-features -- -D warnings' bash "${ARROW_DIR}"
fi

echo "three-repo baseline check passed (${MODE})"
