#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

IMAGE="${IMAGE:?IMAGE is required}"
BOOTSTRAP="${BOOTSTRAP:?BOOTSTRAP is required}"

docker run --rm \
  -v "${REPO_ROOT}:/workspace" \
  -w /workspace \
  -e "BOOTSTRAP=${BOOTSTRAP}" \
  "${IMAGE}" \
  /bin/sh -lc '
    set -eu
    export DEBIAN_FRONTEND=noninteractive

    printf "%s\n" "$BOOTSTRAP" >/tmp/pad-bootstrap.sh
    /bin/sh /tmp/pad-bootstrap.sh

    curl --proto \"=https\" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . \"$HOME/.cargo/env\"

    PAD_INSTALL_FORCE_SOURCE=1 \
    PAD_INSTALL_ASSUME_YES=1 \
    INSTALL_DIR=\"$HOME/.local/bin\" \
    bash ./install.sh

    \"$HOME/.local/bin/pad\" --help >/dev/null
    \"$HOME/.local/bin/pad\" --version
    \"$HOME/.local/bin/pad\" --tmux-doctor
    tmux -V
  '
