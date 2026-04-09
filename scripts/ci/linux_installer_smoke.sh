#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

IMAGE="${IMAGE:?IMAGE is required}"
BOOTSTRAP="${BOOTSTRAP:?BOOTSTRAP is required}"
INSTALL_MODE="${INSTALL_MODE:-prebuilt}"
PAD_TEST_VERSION="${PAD_TEST_VERSION:-v0.0.0-ci}"
PAD_TEST_RELEASE_BASE_URL="${PAD_TEST_RELEASE_BASE_URL:-file:///workspace/.pad-release}"

docker run --rm \
  -v "${REPO_ROOT}:/workspace" \
  -w /workspace \
  -e "BOOTSTRAP=${BOOTSTRAP}" \
  -e "INSTALL_MODE=${INSTALL_MODE}" \
  -e "PAD_TEST_VERSION=${PAD_TEST_VERSION}" \
  -e "PAD_TEST_RELEASE_BASE_URL=${PAD_TEST_RELEASE_BASE_URL}" \
  "${IMAGE}" \
  /bin/sh -lc '
    set -eu
    export DEBIAN_FRONTEND=noninteractive

    printf "%s\n" "$BOOTSTRAP" >/tmp/pad-bootstrap.sh
    /bin/sh /tmp/pad-bootstrap.sh

    case "${INSTALL_MODE}" in
      source)
        curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
        . "$HOME/.cargo/env"
        PAD_INSTALL_FORCE_SOURCE=1 \
        PAD_INSTALL_ASSUME_YES=1 \
        INSTALL_DIR="$HOME/.local/bin" \
        bash ./install.sh
        ;;
      prebuilt)
        PAD_INSTALL_ASSUME_YES=1 \
        PAD_INSTALL_DISABLE_SOURCE_FALLBACK=1 \
        PAD_RELEASE_BASE_URL="${PAD_TEST_RELEASE_BASE_URL}" \
        VERSION="${PAD_TEST_VERSION}" \
        INSTALL_DIR="$HOME/.local/bin" \
        bash ./install.sh 2>&1 | tee /tmp/pad-install.log
        grep -q "Installed binary to" /tmp/pad-install.log
        if grep -q "Falling back to source build" /tmp/pad-install.log; then
          echo "unexpected source fallback during prebuilt smoke" >&2
          exit 1
        fi
        ;;
      *)
        echo "unsupported INSTALL_MODE: ${INSTALL_MODE}" >&2
        exit 1
        ;;
    esac

    "$HOME/.local/bin/pad" --help >/dev/null
    "$HOME/.local/bin/pad" --version
    "$HOME/.local/bin/pad" --tmux-doctor
    tmux -V
  '
