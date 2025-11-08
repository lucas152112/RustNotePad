#!/usr/bin/env bash
# Packages RustNotePad binaries into bin/<platform> directories per target.
# 針對不同平臺建置並打包 RustNotePad，可於 bin/<platform> 目錄取得成品。

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

declare -A PLATFORM_TARGETS=(
  ["linux-x86_64"]="x86_64-unknown-linux-gnu"
  ["windows-x86_64"]="x86_64-pc-windows-gnu"
  ["macos-x86_64"]="x86_64-apple-darwin"
)

if [[ -n "${RNP_PLATFORMS:-}" ]]; then
  read -r -a SELECTED <<<"${RNP_PLATFORMS}"
else
  SELECTED=("linux-x86_64" "windows-x86_64" "macos-x86_64")
fi

if [[ -n "${RNP_BINARIES:-}" ]]; then
  read -r -a PACKAGES <<<"${RNP_BINARIES}"
else
  PACKAGES=("rustnotepad_gui:rustnotepad_gui" "rustnotepad_cli:rustnotepad-cli")
fi

echo "[package] Preparing platforms: ${SELECTED[*]}"
echo "[package] 目標平臺：${SELECTED[*]}"

cargo metadata --format-version 1 >/dev/null

for platform in "${SELECTED[@]}"; do
  target="${PLATFORM_TARGETS[$platform]:-}"
  if [[ -z "$target" ]]; then
    echo "[package] Unknown platform '${platform}', skipping."
    echo "[package] 未知平臺 ${platform}，略過。"
    continue
  fi

  echo ""
  echo "[package] Ensuring Rust target ${target} is installed"
  echo "[package] 確認已安裝 Rust 目標 ${target}"
  if ! rustup target list --installed | grep -q "^${target}$"; then
    rustup target add "$target"
  fi

  output_dir="${REPO_ROOT}/bin/${platform}"
  mkdir -p "$output_dir"

  for pkg_pair in "${PACKAGES[@]}"; do
    package="${pkg_pair%%:*}"
    binary="${pkg_pair##*:}"
    echo ""
    echo "[package] Building ${package} for ${platform} (${target})"
    echo "[package] 建置 ${package} - 目標 ${platform} (${target})"
    cargo build --release --target "$target" -p "$package"

    ext=""
    if [[ "$platform" == windows-* ]]; then
      ext=".exe"
    fi

    src="target/${target}/release/${binary}${ext}"
    if [[ ! -f "$src" ]]; then
      echo "[package] Missing artifact ${src}"
      echo "[package] 找不到產物 ${src}"
      exit 1
    fi
    dest="${output_dir}/${binary}${ext}"
    cp "$src" "$dest"
    echo "[package] Copied ${src} -> ${dest}"
    echo "[package] 已將 ${src} 複製至 ${dest}"
  done
done

echo ""
echo "[package] All requested platforms processed."
echo "[package] 所有指定平臺均已處理完成。"
