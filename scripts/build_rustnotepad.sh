#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BIN_DIR="${REPO_ROOT}/bin"
TARGET_BIN="${REPO_ROOT}/target/release/rustnotepad"

if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "${HOME}/.cargo/env" ]; then
    # shellcheck disable=SC1090
    . "${HOME}/.cargo/env"
  else
    echo "cargo is not available on PATH and ${HOME}/.cargo/env was not found." >&2
    exit 1
  fi
fi

cargo build --release -p rustnotepad_gui

mkdir -p "${BIN_DIR}"
install -m 755 "${TARGET_BIN}" "${BIN_DIR}/rustnotepad"

echo "RustNotePad binary written to ${BIN_DIR}/rustnotepad"
