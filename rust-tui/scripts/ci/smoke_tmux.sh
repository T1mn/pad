#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
BIN_PATH=${BIN_PATH:-"$ROOT_DIR/target/debug/pad"}
SERVER_NAME=${PAD_CI_TMUX_SERVER:-"pad-ci-$$"}
TEMP_HOME=$(mktemp -d "${TMPDIR:-/tmp}/pad-ci-home.XXXXXX")
MOCK_SCRIPT="$TEMP_HOME/mock-codex"
RUN_LOG="$TEMP_HOME/pad-run.log"
STATUS_PATH="$TEMP_HOME/.pad/pad-status.json"
APP_LOG="$TEMP_HOME/.pad/logs/pad.log"

cleanup() {
  tmux -L "$SERVER_NAME" kill-server >/dev/null 2>&1 || true
  rm -rf "$TEMP_HOME"
}

trap cleanup EXIT INT TERM

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required for smoke tests" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for smoke tests" >&2
  exit 1
fi

if [ ! -x "$BIN_PATH" ]; then
  echo "pad binary not found at $BIN_PATH" >&2
  exit 1
fi

cat >"$MOCK_SCRIPT" <<'EOF'
#!/bin/sh
printf 'codex smoke agent ready\n'
while :; do
  printf 'codex> '
  sleep 5
done
EOF
chmod +x "$MOCK_SCRIPT"

tmux -L "$SERVER_NAME" -f /dev/null new-session \
  -d -s agents -n agents -x 160 -y 48 "$MOCK_SCRIPT"

PAD_CMD="export HOME='$TEMP_HOME'; export TERM='xterm-256color'; cd '$ROOT_DIR'; exec '$BIN_PATH' --debug"
tmux -L "$SERVER_NAME" -f /dev/null new-session \
  -d -s pad -n pad -x 160 -y 48 "/bin/sh -lc \"$PAD_CMD\" >'$RUN_LOG' 2>&1"

wait_for_file() {
  target=$1
  timeout_s=$2
  i=0
  while [ "$i" -lt "$timeout_s" ]; do
    if [ -f "$target" ]; then
      return 0
    fi
    i=$((i + 1))
    sleep 1
  done
  return 1
}

wait_for_grep() {
  target=$1
  pattern=$2
  timeout_s=$3
  i=0
  while [ "$i" -lt "$timeout_s" ]; do
    if [ -f "$target" ] && grep -q "$pattern" "$target"; then
      return 0
    fi
    i=$((i + 1))
    sleep 1
  done
  return 1
}

if ! wait_for_file "$STATUS_PATH" 15; then
  echo "pad status file was not created" >&2
  [ -f "$RUN_LOG" ] && cat "$RUN_LOG" >&2
  exit 1
fi

PAD_PID=$(python3 - "$STATUS_PATH" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)
print(data["pid"])
PY
)

if ! kill -0 "$PAD_PID" >/dev/null 2>&1; then
  echo "pad process is not alive: pid=$PAD_PID" >&2
  [ -f "$RUN_LOG" ] && cat "$RUN_LOG" >&2
  exit 1
fi

if ! wait_for_grep "$APP_LOG" "agent=Codex" 20; then
  echo "pad did not detect the mock codex pane" >&2
  [ -f "$APP_LOG" ] && cat "$APP_LOG" >&2
  exit 1
fi

if ! wait_for_grep "$APP_LOG" "scanner: 共检测到 1 个智能体面板" 20; then
  echo "pad did not report the expected agent count" >&2
  [ -f "$APP_LOG" ] && cat "$APP_LOG" >&2
  exit 1
fi

tmux -L "$SERVER_NAME" send-keys -t pad:0.0 q

i=0
while [ "$i" -lt 15 ]; do
  if ! kill -0 "$PAD_PID" >/dev/null 2>&1; then
    break
  fi
  i=$((i + 1))
  sleep 1
done

if kill -0 "$PAD_PID" >/dev/null 2>&1; then
  echo "pad did not exit after sending q" >&2
  [ -f "$APP_LOG" ] && cat "$APP_LOG" >&2
  exit 1
fi

if [ -f "$STATUS_PATH" ]; then
  echo "pad status file was not cleaned up on exit" >&2
  cat "$STATUS_PATH" >&2
  exit 1
fi

echo "tmux smoke test passed on server $SERVER_NAME"
