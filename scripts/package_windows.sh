#!/usr/bin/env bash
# Cross-compile a Windows .exe from Linux with the icon embedded.
# Requires: rustup target add x86_64-pc-windows-gnu
#           sudo apt install mingw-w64
#
# Usage:
#   ./scripts/package_windows.sh

set -euo pipefail

TARGET="x86_64-pc-windows-gnu"
OUT_DIR="dist/windows"

echo "→ Checking cross-compile toolchain…"

if ! rustup target list --installed | grep -q "${TARGET}"; then
    echo "  Adding target ${TARGET}…"
    rustup target add "${TARGET}"
fi

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "Error: mingw-w64 not found. Install with:"
    echo "  sudo apt install mingw-w64"
    exit 1
fi

echo "→ Building for ${TARGET}…"
cargo build --release --target "${TARGET}"

mkdir -p "${OUT_DIR}"
cp "target/${TARGET}/release/jiosaavn.exe" "${OUT_DIR}/jiosaavn.exe"

SIZE=$(du -sh "${OUT_DIR}/jiosaavn.exe" | cut -f1)
echo ""
echo "✓ Windows binary: ${OUT_DIR}/jiosaavn.exe (${SIZE})"
echo "  The icon is embedded — it will appear in File Explorer."
echo ""
echo "  Verify with:"
echo "    file ${OUT_DIR}/jiosaavn.exe"
