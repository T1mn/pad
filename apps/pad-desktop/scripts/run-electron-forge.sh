#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="${0:A:h}"
APP_DIR="${SCRIPT_DIR:h}"
FORGE_CLI="${APP_DIR}/node_modules/@electron-forge/cli/dist/electron-forge.js"
EXPECTED_NODE_VERSION="24.20.0"

if [[ ! -f "${FORGE_CLI}" ]]; then
  echo "Electron Forge is not installed. Run npm ci in ${APP_DIR} before the offline build." >&2
  exit 1
fi

select_node() {
  local candidate version cached_candidate
  local -a candidates
  candidates=()
  if [[ -n "${PAD_FORGE_NODE_BIN:-}" ]]; then
    candidates+=("${PAD_FORGE_NODE_BIN}")
  fi
  if command -v node >/dev/null 2>&1; then
    candidates+=("$(command -v node)")
  fi
  candidates+=(
    "/opt/homebrew/opt/node@24/bin/node"
    "/usr/local/opt/node@24/bin/node"
  )
  cached_candidate="$(npx --offline --yes "node@${EXPECTED_NODE_VERSION}" -p 'process.execPath' 2>/dev/null || true)"
  if [[ -n "${cached_candidate}" ]]; then
    candidates+=("${cached_candidate}")
  fi

  for candidate in "${candidates[@]}"; do
    [[ -x "${candidate}" ]] || continue
    version="$(${candidate} -p 'process.versions.node' 2>/dev/null || true)"
    if [[ "${version}" == "${EXPECTED_NODE_VERSION}" ]]; then
      print -r -- "${candidate}"
      return 0
    fi
  done
  return 1
}

# Electron Forge, Electron Rebuild 4 and the hardened extractor are validated
# together on one exact Node release. The final fallback resolves only an
# already cached binary in npm offline mode, so packaging cannot download it.
FORGE_NODE="$(select_node || true)"
if [[ -z "${FORGE_NODE}" ]]; then
  echo "The pinned Node 24.20.0 runtime is required for Electron Forge." >&2
  echo "Set PAD_FORGE_NODE_BIN or cache node@24.20.0 before running this offline build." >&2
  exit 1
fi

if [[ "${1:-}" == "--print-node" ]]; then
  print -r -- "${FORGE_NODE}"
  exit 0
fi

exec "${FORGE_NODE}" "${FORGE_CLI}" "$@"
