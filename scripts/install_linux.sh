#!/usr/bin/env bash
# Install jiosaavn binary, icon, and desktop entry on Linux.
# Run after `cargo build --release`.
#
# Usage:
#   ./scripts/install_linux.sh          # user install (~/.local)
#   sudo ./scripts/install_linux.sh     # system-wide (/usr/local)

set -euo pipefail

BINARY_NAME="jiosaavn"

if [[ "${EUID}" -eq 0 ]]; then
    BIN_DIR="/usr/local/bin"
    ICON_DIR="/usr/share/icons/hicolor/256x256/apps"
    DESKTOP_DIR="/usr/share/applications"
else
    BIN_DIR="${HOME}/.local/bin"
    ICON_DIR="${HOME}/.local/share/icons/hicolor/256x256/apps"
    DESKTOP_DIR="${HOME}/.local/share/applications"
fi

BINARY_SRC="target/release/${BINARY_NAME}"
ICON_SRC="jsd.png"
DESKTOP_SRC="assets/linux/${BINARY_NAME}.desktop"

# Verify the binary has been built
if [[ ! -f "${BINARY_SRC}" ]]; then
    echo "Error: ${BINARY_SRC} not found. Run 'cargo build --release' first."
    exit 1
fi

echo "→ Installing binary to ${BIN_DIR}/"
install -Dm 0755 "${BINARY_SRC}" "${BIN_DIR}/${BINARY_NAME}"

echo "→ Installing icon to ${ICON_DIR}/"
install -Dm 0644 "${ICON_SRC}" "${ICON_DIR}/${BINARY_NAME}.png"

echo "→ Installing .desktop entry to ${DESKTOP_DIR}/"
install -Dm 0644 "${DESKTOP_SRC}" "${DESKTOP_DIR}/${BINARY_NAME}.desktop"

# Refresh icon/desktop caches
if command -v gtk-update-icon-cache &>/dev/null; then
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
fi
if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true
fi
if command -v xdg-icon-resource &>/dev/null; then
    xdg-icon-resource install --size 256 "${ICON_SRC}" "${BINARY_NAME}" 2>/dev/null || true
fi

echo ""
echo "✓ Installation complete."
echo "  Run: ${BINARY_NAME} --help"
if [[ "${EUID}" -ne 0 && ":${PATH}:" != *":${BIN_DIR}:"* ]]; then
    echo ""
    echo "  Note: ${BIN_DIR} is not in your PATH."
    echo "  Add this to ~/.bashrc or ~/.zshrc:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
