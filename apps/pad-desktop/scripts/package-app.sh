#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="${0:A:h}"
APP_DIR="${SCRIPT_DIR:h}"
BUILD_CONFIG="${BUILD_CONFIG:-release}"
DIST_DIR="${APP_DIR}/dist"
APP_BUNDLE="${DIST_DIR}/PAD Desktop.app"

cd "${APP_DIR}"
swift build --configuration "${BUILD_CONFIG}"

# The Swift shell talks to PAD's private JSONL control plane.  Bundle the
# native host so a copied .app does not depend on the checkout's working
# directory.  Set PAD_RUST_BINARY to reuse an already-built binary.
PAD_REPO_DIR="${APP_DIR}/../.."
PAD_RUST_BINARY_OVERRIDE="${PAD_RUST_BINARY:-}"
if [[ -n "${PAD_RUST_BINARY_OVERRIDE}" ]]; then
  PAD_RUST_BINARY="${PAD_RUST_BINARY_OVERRIDE}"
else
  cargo build --release --locked --manifest-path "${PAD_REPO_DIR}/rust-tui/Cargo.toml"
  PAD_RUST_BINARY="${PAD_REPO_DIR}/rust-tui/target/release/pad"
fi
if [[ ! -x "${PAD_RUST_BINARY}" ]]; then
  echo "PAD Rust host binary not found: ${PAD_RUST_BINARY}" >&2
  exit 1
fi

EXECUTABLE="$(swift build --configuration "${BUILD_CONFIG}" --show-bin-path)/PADDesktop"
CONTENTS="${APP_BUNDLE}/Contents"
rm -rf "${APP_BUNDLE}"
mkdir -p "${CONTENTS}/MacOS" "${CONTENTS}/Resources"
cp "${EXECUTABLE}" "${CONTENTS}/MacOS/PADDesktop"
cp "${PAD_RUST_BINARY}" "${CONTENTS}/Resources/pad"
cp "${APP_DIR}/Resources/Info.plist" "${CONTENTS}/Info.plist"
cp "${APP_DIR}/Resources/PADDesktop.icns" "${CONTENTS}/Resources/PADDesktop.icns"
chmod +x "${CONTENTS}/MacOS/PADDesktop"
chmod +x "${CONTENTS}/Resources/pad"

# Bundle the Pi runtime when the build machine has a global Pi install.  Bun is
# preferred because its arm64 binary only depends on macOS system libraries;
# copying Homebrew's Node alone would leave /opt/homebrew/opt/* dylib paths in
# a supposedly standalone app.  A static node shim keeps PiLogin's existing
# SDK entrypoint working while the Pi sidecar uses dist/bun/cli.js.
# Development builds continue to use the user's PATH when these sources are
# unavailable.
PI_PACKAGE_SOURCE="${PAD_PI_PACKAGE:-/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent}"
NODE_SOURCE="${PAD_NODE_BIN:-/opt/homebrew/bin/node}"
BUN_SOURCE="${PAD_BUN_BIN:-/opt/homebrew/bin/bun}"
if [[ -d "${PI_PACKAGE_SOURCE}" && -x "${BUN_SOURCE}" ]]; then
  mkdir -p "${CONTENTS}/Resources/pi" "${CONTENTS}/Resources/bin"
  cp -R "${PI_PACKAGE_SOURCE}/." "${CONTENTS}/Resources/pi/"
  cp "${BUN_SOURCE}" "${CONTENTS}/Resources/bin/bun"
  cp "${APP_DIR}/Resources/node-bun-shim.sh" "${CONTENTS}/Resources/bin/node"
  cp "${APP_DIR}/Resources/pi" "${CONTENTS}/Resources/bin/pi"
  chmod +x "${CONTENTS}/Resources/bin/bun" "${CONTENTS}/Resources/bin/node" "${CONTENTS}/Resources/bin/pi"
elif [[ -d "${PI_PACKAGE_SOURCE}" && -x "${NODE_SOURCE}" ]]; then
  mkdir -p "${CONTENTS}/Resources/pi" "${CONTENTS}/Resources/bin" "${CONTENTS}/Resources/lib"
  cp -R "${PI_PACKAGE_SOURCE}/." "${CONTENTS}/Resources/pi/"
  cp "${NODE_SOURCE}" "${CONTENTS}/Resources/bin/node"
  NODE_REAL="$(realpath "${NODE_SOURCE}")"
  NODE_LIB_DIR="$(dirname "${NODE_REAL}")/../lib"
  for node_library in "${NODE_LIB_DIR}"/libnode*.dylib; do
    [[ -f "${node_library}" ]] || continue
    cp "${node_library}" "${CONTENTS}/Resources/lib/"
  done
  cp "${APP_DIR}/Resources/pi" "${CONTENTS}/Resources/bin/pi"
  chmod +x "${CONTENTS}/Resources/bin/node" "${CONTENTS}/Resources/bin/pi"
else
  echo "Pi runtime not bundled; using the system Pi installation at runtime." >&2
fi

# Give the copied bundle a valid local signature so Finder/LaunchServices can
# open it as a normal macOS app. This is intentionally ad-hoc; distribution
# outside the local machine still requires the developer's Apple signing and
# notarization identity.
if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "${APP_BUNDLE}" >/dev/null
fi

echo "Created: ${APP_BUNDLE}"
echo "Run: open \"${APP_BUNDLE}\""
