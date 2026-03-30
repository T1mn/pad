#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PAD_BIN="${PAD_BIN:-${REPO_ROOT}/rust-tui/target/debug/pad}"
SOCK="${TMPDIR:-/tmp}/pad-smoke-$$.sock"
SMOKE_HOME="$(mktemp -d)"
LOG_FILE="${SMOKE_HOME}/.pad/logs/pad.log"

cleanup() {
  tmux -S "${SOCK}" kill-server >/dev/null 2>&1 || true
  rm -rf "${SMOKE_HOME}"
}
trap cleanup EXIT

tm() {
  tmux -S "${SOCK}" "$@"
}

wait_for_file() {
  local path="$1"
  local timeout="${2:-20}"
  local i=0
  while [ "${i}" -lt "${timeout}" ]; do
    [ -f "${path}" ] && return 0
    sleep 1
    i=$((i + 1))
  done
  return 1
}

wait_for_capture() {
  local target="$1"
  local needle="$2"
  local timeout="${3:-20}"
  local i=0
  while [ "${i}" -lt "${timeout}" ]; do
    if tm capture-pane -p -t "${target}" 2>/dev/null | grep -Fq "${needle}"; then
      return 0
    fi
    sleep 1
    i=$((i + 1))
  done
  return 1
}

wait_for_log() {
  local needle="$1"
  local timeout="${2:-20}"
  local i=0
  while [ "${i}" -lt "${timeout}" ]; do
    if [ -f "${LOG_FILE}" ] && grep -Fq "${needle}" "${LOG_FILE}"; then
      return 0
    fi
    sleep 1
    i=$((i + 1))
  done
  return 1
}

if [ ! -x "${PAD_BIN}" ]; then
  echo "pad binary not found: ${PAD_BIN}" >&2
  exit 1
fi

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required for smoke test" >&2
  exit 1
fi

tm kill-server >/dev/null 2>&1 || true
tm -f /dev/null new-session -d -s agents -n agents -x 160 -y 48 \
  "${REPO_ROOT}/scripts/ci/mock_agent.sh codex"
tm split-window -t agents:0 -h "${REPO_ROOT}/scripts/ci/mock_agent.sh claude"
tm new-session -d -s pad -x 160 -y 48 \
  "/bin/sh -lc 'export HOME=${SMOKE_HOME}; export TERM=xterm-256color; cd ${REPO_ROOT}/rust-tui && ${PAD_BIN} --debug'"

wait_for_file "${LOG_FILE}" 20
wait_for_capture "pad:0.0" "在线" 20
wait_for_capture "pad:0.0" "CODEX" 20
wait_for_capture "pad:0.0" "agents:0.0" 20

tm send-keys -t pad:0.0 Enter
sleep 1
tm send-keys -t pad:0.0 2
sleep 1
tm send-keys -t pad:0.0 Enter

wait_for_log "attach.cross_session: handoff complete" 20
wait_for_log "install_return_bindings: start" 20

return_cmd="$(tm list-keys -T root | awk '/ F12 / { sub(/^.*run-shell /, ""); print; exit }')"
if [ -z "${return_cmd}" ]; then
  echo "missing F12 return binding" >&2
  exit 1
fi
return_cmd="${return_cmd#\"}"
return_cmd="${return_cmd%\"}"
tm run-shell "${return_cmd}"

wait_for_log "[return] after_return_select" 20

echo "tmux smoke passed"
