#!/usr/bin/env bash
# Builds RustNotePad for multiple targets in one pass.
# 一次為多個目標平臺建置 RustNotePad。

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [[ -n "${RNP_TARGETS:-}" ]]; then
  read -r -a TARGETS <<<"${RNP_TARGETS}"
else
  TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
  )
fi

echo "[build-all] Selected targets: ${TARGETS[*]}"
echo "[build-all] 指定建置目標：${TARGETS[*]}"

cargo metadata --format-version 1 >/dev/null

for target in "${TARGETS[@]}"; do
  echo ""
  echo "[build-all] Ensuring target ${target} is installed"
  echo "[build-all] 確認已安裝目標 ${target}"
  if ! rustup target list --installed | grep -q "^${target}$"; then
    rustup target add "$target"
  fi

  echo "[build-all] Building release artifacts for ${target}"
  echo "[build-all] 建置 ${target} 的 release 產物"
  cargo build --release --target "$target"
done

echo ""
echo "[build-all] All targets built. Outputs reside under target/<triple>/release/."
echo "[build-all] 全部目標建置完成，產物位於 target/<三元組>/release/。"
