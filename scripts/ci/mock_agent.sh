#!/usr/bin/env bash
set -euo pipefail

agent_name="${1:-codex}"

case "${agent_name}" in
  codex)
    echo "codex mock ready"
    echo "Q: implement release flow"
    echo "A: smoke preview sentinel codex"
    ;;
  claude)
    echo "claude mock ready"
    echo "Q: inspect tmux pipeline"
    echo "A: smoke preview sentinel claude"
    ;;
  grok)
    echo "grok mock ready"
    echo "Q: inspect Grok history"
    echo "A: smoke preview sentinel grok"
    ;;
  opencode)
    echo "opencode mock ready"
    echo "Q: inspect OpenCode history"
    echo "A: smoke preview sentinel opencode"
    ;;
  *)
    echo "${agent_name} mock ready"
    echo "Q: generic smoke prompt"
    echo "A: generic smoke reply"
    ;;
esac

sleep_bin="$(command -v sleep)"
mock_process="${TMPDIR:-/tmp}/${agent_name}-mock-agent-$$"

cleanup() {
  rm -f "${mock_process}"
}
trap cleanup EXIT
trap 'cleanup; exit 0' HUP INT TERM

ln -sf "${sleep_bin}" "${mock_process}"

while true; do
  "${mock_process}" 3600 &
  wait "$!" || true
done
