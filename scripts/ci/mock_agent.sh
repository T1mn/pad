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
  *)
    echo "${agent_name} mock ready"
    echo "Q: generic smoke prompt"
    echo "A: generic smoke reply"
    ;;
esac

while true; do
  sleep 5
done
