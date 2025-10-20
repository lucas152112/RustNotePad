#!/usr/bin/env bash
set -euo pipefail

# Ensure we are operating from the workspace root.
if [[ ! -f Cargo.toml ]]; then
  echo "Please run this script from the repository root (where Cargo.toml lives)." >&2
  exit 1
fi

echo "==> Installing Node dependencies"
npm ci --no-optional

echo "==> Building Rust workspace (debug profile)"
cargo build --locked

echo "==> Running Rust unit tests"
cargo test --locked

# Launch a virtual framebuffer so GUI binaries can start without a hardware display.
XVFB_DISPLAY=${XVFB_DISPLAY:-:99}
XVFB_RESOLUTION=${XVFB_RESOLUTION:-1280x720x24}
XVFB_LOG=${XVFB_LOG:-/tmp/xvfb.log}

echo "==> Starting Xvfb on display ${XVFB_DISPLAY}"
Xvfb "${XVFB_DISPLAY}" -screen 0 "${XVFB_RESOLUTION}" >"${XVFB_LOG}" 2>&1 &
XVFB_PID=$!
export DISPLAY=${XVFB_DISPLAY}

cleanup() {
  if [[ -n "${TAURI_DRIVER_PID:-}" ]]; then
    kill "${TAURI_DRIVER_PID}" 2>/dev/null || true
  fi
  kill "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

# Wait momentarily to ensure the framebuffer is ready.
sleep 1

echo "==> Starting tauri-driver on port 4444"
tauri-driver --port 4444 > /tmp/tauri-driver.log 2>&1 &
TAURI_DRIVER_PID=$!

# Give tauri-driver a brief moment to bind to the socket.
for _ in {1..20}; do
  if nc -z localhost 4444 >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

echo "==> Running Playwright E2E tests"
npx playwright install --with-deps >/dev/null 2>&1 || true
npx playwright test "$@"

echo "==> Tests completed successfully"
