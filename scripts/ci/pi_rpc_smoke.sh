#!/usr/bin/env bash
# Verify the Pi sidecar contract used by PAD Desktop on macOS.
#
# This is intentionally a credential-free smoke test.  It exercises the real
# installed Pi process, RPC framing, the private agent root, and
# --session-dir.  Set PAD_PI_SMOKE_PROMPT=1 to run a paid/provider-backed
# prompt after the handshake (the caller must provide Pi credentials).
set -euo pipefail

if [ "$(uname -s)" != "Darwin" ]; then
  echo "pi_rpc_smoke: macOS only" >&2
  exit 2
fi

PI_BIN="${PAD_PI_BIN:-pi}"
EXPECTED_VERSION="${PAD_PI_EXPECTED_VERSION:-0.84.4}"

if ! command -v "${PI_BIN}" >/dev/null 2>&1; then
  echo "pi_rpc_smoke: Pi executable not found: ${PI_BIN}" >&2
  echo "Install @earendil-works/pi-coding-agent or set PAD_PI_BIN to the bundled sidecar." >&2
  exit 1
fi

actual_version="$("${PI_BIN}" --version 2>/dev/null | tr -d '\r\n')"
if [ "${actual_version}" != "${EXPECTED_VERSION}" ] && [ "${PAD_PI_ALLOW_VERSION_MISMATCH:-0}" != "1" ]; then
  echo "pi_rpc_smoke: expected Pi ${EXPECTED_VERSION}, found ${actual_version}" >&2
  echo "Set PAD_PI_ALLOW_VERSION_MISMATCH=1 only for an intentional compatibility check." >&2
  exit 1
fi

smoke_root="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/pad-pi-rpc-smoke.XXXXXX")" && pwd -P)"
trap 'rm -rf "${smoke_root}"' EXIT
agent_root="${smoke_root}/agent"
session_root="${smoke_root}/sessions"
mkdir -p "${agent_root}" "${session_root}"

rpc_args=(
  --mode rpc
  --offline
  --session-dir "${session_root}"
  --no-approve
  --no-context-files
  --no-extensions
  --no-skills
  --no-prompt-templates
  --no-themes
)

{
  # Give the RPC loop a short scheduling window between state-changing
  # commands.  Pi may process stdin asynchronously and does not promise that
  # a burst containing new_session is handled before the next command.
  printf '%s\n' '{"id":"state-1","type":"get_state"}'
  sleep 0.5
  printf '%s\n' '{"id":"new-1","type":"new_session"}'
  sleep 0.5
  printf '%s\n' '{"id":"state-2","type":"get_state"}'
  sleep 0.5
} | env \
      PI_CODING_AGENT_DIR="${agent_root}" \
      PI_CODING_AGENT_SESSION_DIR="${session_root}" \
      "${PI_BIN}" "${rpc_args[@]}" \
      >"${smoke_root}/stdout" 2>"${smoke_root}/stderr"

if ! grep -Fq '"id":"state-1","type":"response","command":"get_state","success":true' "${smoke_root}/stdout"; then
  echo "pi_rpc_smoke: get_state handshake failed" >&2
  sed -n '1,80p' "${smoke_root}/stdout" >&2
  sed -n '1,80p' "${smoke_root}/stderr" >&2
  exit 1
fi
if ! grep -Fq '"id":"new-1","type":"response","command":"new_session","success":true' "${smoke_root}/stdout"; then
  echo "pi_rpc_smoke: new_session failed" >&2
  sed -n '1,80p' "${smoke_root}/stdout" >&2
  exit 1
fi
if ! grep -Fq "\"sessionFile\":\"${session_root}/" "${smoke_root}/stdout"; then
  echo "pi_rpc_smoke: Pi did not honor --session-dir=${session_root}" >&2
  sed -n '1,80p' "${smoke_root}/stdout" >&2
  exit 1
fi
if [ ! -f "${agent_root}/auth.json" ] || [ ! -f "${agent_root}/models-store.json" ]; then
  echo "pi_rpc_smoke: Pi did not use the PAD-owned PI_CODING_AGENT_DIR" >&2
  find "${smoke_root}" -maxdepth 3 -print >&2
  exit 1
fi

if [ "${PAD_PI_SMOKE_PROMPT:-0}" = "1" ]; then
  exec node "$(dirname "${BASH_SOURCE[0]}")/pi_rpc_prompt_smoke.mjs"
fi

echo "pi_rpc_smoke: Pi ${actual_version} RPC handshake passed"
echo "pi_rpc_smoke: private agent root and session-dir passed"
