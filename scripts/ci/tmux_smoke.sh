#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PAD_BIN="${PAD_BIN:-${REPO_ROOT}/rust-tui/target/debug/pad}"
SOCK="${TMPDIR:-/tmp}/pad-smoke-$$.sock"
SMOKE_HOME="$(mktemp -d)"
LOG_FILE="${SMOKE_HOME}/.pad/logs/pad.log"
STATUS_FILE="${SMOKE_HOME}/.pad/pad-status.json"
CONTROL_FIFO="${SMOKE_HOME}/tmux-control.in"
WRITABLE_CLIENT_PID=""

cleanup() {
  if [ -n "${WRITABLE_CLIENT_PID}" ]; then
    kill "${WRITABLE_CLIENT_PID}" >/dev/null 2>&1 || true
  fi
  tmux -S "${SOCK}" kill-server >/dev/null 2>&1 || true
  rm -rf "${SMOKE_HOME}"
}
trap cleanup EXIT

tm() {
  tmux -S "${SOCK}" "$@"
}

dump_diagnostics() {
  echo "=== tmux smoke diagnostics ===" >&2
  echo "PAD_BIN=${PAD_BIN}" >&2
  echo "SMOKE_HOME=${SMOKE_HOME}" >&2
  echo "--- tmux list-sessions ---" >&2
  tm list-sessions >&2 || true
  echo "--- tmux list-panes ---" >&2
  tm list-panes -a -F '#{session_name}:#{window_index}.#{pane_index} #{pane_id} #{pane_current_command} #{pane_current_path}' >&2 || true
  echo "--- pad pane capture ---" >&2
  tm capture-pane -p -t "pad:0.0" >&2 || true
  echo "--- agents pane 0 capture ---" >&2
  tm capture-pane -p -t "agents:0.0" >&2 || true
  echo "--- agents pane 1 capture ---" >&2
  tm capture-pane -p -t "agents:0.1" >&2 || true
  echo "--- agents panes ---" >&2
  tm list-panes -t agents:0 -F '#{pane_id}' 2>/dev/null | while read -r pane; do
    echo "--- ${pane} ---" >&2
    tm capture-pane -p -t "${pane}" >&2 || true
  done
  if [ -f "${STATUS_FILE}" ]; then
    echo "--- status file ---" >&2
    cat "${STATUS_FILE}" >&2
  fi
  if [ -f "${LOG_FILE}" ]; then
    echo "--- pad.log ---" >&2
    cat "${LOG_FILE}" >&2
  fi
  if [ -f "${SMOKE_HOME}/tmux-control.log" ]; then
    echo "--- writable control client ---" >&2
    cat "${SMOKE_HOME}/tmux-control.log" >&2
  fi
}

fail() {
  trap - ERR
  echo "$1" >&2
  dump_diagnostics
  exit 1
}

trap 'fail "tmux smoke command failed at line ${LINENO}: ${BASH_COMMAND}"' ERR

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

wait_for_root_binding() {
  local key="$1"
  local needle="${2:-}"
  local timeout="${3:-20}"
  local i=0
  local line=""
  while [ "${i}" -lt "${timeout}" ]; do
    line="$(tm list-keys -T root 2>/dev/null | awk "/ ${key} / { print; exit }")"
    if [ -n "${line}" ] && { [ -z "${needle}" ] || grep -Fq "${needle}" <<<"${line}"; }; then
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
tm split-window -t agents:0 -v "${REPO_ROOT}/scripts/ci/mock_agent.sh grok"
tm split-window -t agents:0 -v "${REPO_ROOT}/scripts/ci/mock_agent.sh opencode"
tm select-layout -t agents:0 tiled
tm new-session -d -s pad -x 160 -y 48 \
  "/bin/sh -lc 'export HOME=${SMOKE_HOME}; export TERM=xterm-256color; cd ${REPO_ROOT}/rust-tui && ${PAD_BIN} --debug'"
# A real PAD run has a writable terminal client. Keep one here so newer tmux
# versions do not select PAD's read-only event client for switch-client.
mkfifo "${CONTROL_FIFO}"
exec 9<>"${CONTROL_FIFO}"
tmux -S "${SOCK}" -C attach-session -t pad -f ignore-size,no-output \
  <"${CONTROL_FIFO}" >"${SMOKE_HOME}/tmux-control.log" 2>&1 &
WRITABLE_CLIENT_PID=$!

if ! wait_for_file "${LOG_FILE}" 20; then
  fail "pad log file was not created"
fi
if ! wait_for_file "${STATUS_FILE}" 20; then
  fail "pad status file was not created"
fi
if ! wait_for_log "agent=Codex" 20; then
  fail "pad did not scan the mock codex pane"
fi
if ! wait_for_log "agent=Claude" 20; then
  fail "pad did not scan the mock claude pane"
fi
if ! wait_for_log "agent=Grok" 20; then
  fail "pad did not scan the mock grok pane"
fi
if ! wait_for_log "agent=OpenCode" 20; then
  fail "pad did not scan the mock opencode pane"
fi
if ! wait_for_capture "pad:0.0" "CODEX" 20; then
  fail "pad UI did not render the codex panel"
fi

tm send-keys -t pad:0.0 Enter
sleep 1
tm send-keys -t pad:0.0 2
sleep 1
tm send-keys -t pad:0.0 Enter

if ! wait_for_log "attach.cross_session: handoff complete" 20; then
  fail "pad did not complete cross-session attach"
fi
if ! wait_for_root_binding "F12" "PAD_RETURN_BINDING=1;" 20; then
  fail "pad did not install return bindings"
fi

return_cmd="$(sed -n 's/^.*stage=attach\.return_cmd cmd=//p' "${LOG_FILE}" | tail -n 1)"
if [ -z "${return_cmd}" ]; then
  fail "missing logged return command"
fi
tm run-shell "${return_cmd}"

if ! wait_for_log "[return] after_return_select" 20; then
  fail "pad did not return to the original pane"
fi

echo "tmux smoke passed"
