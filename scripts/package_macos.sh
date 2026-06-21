#!/usr/bin/env bash
# Build jiosaavn for macOS and wrap it in a proper .app bundle.
# Run on a Mac or from a macOS cross-compile host.
#
# Usage:
#   ./scripts/package_macos.sh            # builds current arch (arm64 or x86_64)
#   ./scripts/package_macos.sh universal  # builds a fat universal binary

set -euo pipefail

BINARY_NAME="jiosaavn"
APP_NAME="JioSaavn Downloader"
BUNDLE_ID="com.habitual69.jiosaavn"
VERSION="1.0.0"
ICON_SRC="jsd.icns"
OUT_DIR="dist/macos"

# ── Build ─────────────────────────────────────────────────────────────────────
if [[ "${1:-}" == "universal" ]]; then
    echo "→ Building universal binary (arm64 + x86_64)…"
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-apple-darwin
    mkdir -p "${OUT_DIR}"
    lipo -create -output "${OUT_DIR}/${BINARY_NAME}" \
        target/aarch64-apple-darwin/release/${BINARY_NAME} \
        target/x86_64-apple-darwin/release/${BINARY_NAME}
    echo "   Universal binary created at ${OUT_DIR}/${BINARY_NAME}"
else
    echo "→ Building for native arch…"
    cargo build --release
    mkdir -p "${OUT_DIR}"
    cp "target/release/${BINARY_NAME}" "${OUT_DIR}/${BINARY_NAME}"
fi

# ── .app bundle ───────────────────────────────────────────────────────────────
APP="${OUT_DIR}/${APP_NAME}.app"
rm -rf "${APP}"

install -d "${APP}/Contents/MacOS"
install -d "${APP}/Contents/Resources"

# Binary
install -m 0755 "${OUT_DIR}/${BINARY_NAME}" "${APP}/Contents/MacOS/${BINARY_NAME}"

# Icon
cp "${ICON_SRC}" "${APP}/Contents/Resources/${BINARY_NAME}.icns"

# Info.plist
cat > "${APP}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>             <string>${BINARY_NAME}</string>
    <key>CFBundleDisplayName</key>      <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>       <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>          <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundlePackageType</key>      <string>APPL</string>
    <key>CFBundleExecutable</key>       <string>${BINARY_NAME}</string>
    <key>CFBundleIconFile</key>         <string>${BINARY_NAME}</string>
    <key>LSMinimumSystemVersion</key>   <string>10.15</string>
    <key>NSHighResolutionCapable</key>  <true/>
    <key>LSUIElement</key>              <true/>
</dict>
</plist>
PLIST

# Tell macOS Finder to refresh its icon cache for this bundle
if command -v SetFile &>/dev/null; then
    SetFile -a B "${APP}"
fi
touch "${APP}"

echo ""
echo "✓ App bundle: ${APP}"
echo "  Drag to /Applications to install."
echo ""
echo "  CLI usage (add to PATH):"
echo "    sudo ln -sf \"\$(pwd)/${APP}/Contents/MacOS/${BINARY_NAME}\" /usr/local/bin/${BINARY_NAME}"
