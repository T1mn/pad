pub(super) const HOOK_BRIDGE_TEMPLATE_BASE: &str = r###"#!/usr/bin/env python3
# pad-bridge-version: __PAD_BRIDGE_VERSION__
import json
import os
import socket
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

PAD_HOME = Path.home() / ".pad"
SOCKET_PATHS = [
    PAD_HOME / "pad-hook.sock",
    PAD_HOME / "telegram-hook.sock",
]


__PAD_SILENCE_STDIO_BLOCK__
def terminal_info_from_env():
    return {
        "runtime": "native",
        "pane_id": os.environ.get("PAD_PANE_ID"),
    }


def normalized_event_name(name):
    if name == "UserPromptSubmit":
        return "user_prompt_submit"
    if name == "SessionStart":
        return "session_start"
    if name == "Stop":
        return "stop"
    return (name or "unknown").lower()


__PAD_LOAD_PAYLOAD_BLOCK__
__PAD_RECORD_TURN_DIFF_BLOCK__
def main():
__PAD_MAIN_START_LINE__
    if "codex-" in "__PAD_BRIDGE_VERSION__" and not pad_codex_hooks_enabled():
        return
    payload = __PAD_PAYLOAD_EXPR__
__PAD_HOOK_TYPE_LINE__
    terminal = terminal_info_from_env()
    event_name = __PAD_EVENT_NAME_EXPR__

    message = {
        "event": normalized_event_name(event_name),
        "hook_event_name": event_name,
        "turn_id": payload.get("turn_id"),
        "session_id": payload.get("session_id"),
        "transcript_path": payload.get("transcript_path"),
        "cwd": payload.get("cwd"),
        "prompt": payload.get("prompt"),
        "last_assistant_message": payload.get("last_assistant_message"),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "terminal": terminal,
    }

    record_codex_turn_diff(message)

    for socket_path in SOCKET_PATHS:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(0.5)
        try:
            sock.connect(str(socket_path))
            sock.sendall((json.dumps(message, ensure_ascii=False) + "\n").encode("utf-8"))
        except Exception:
            pass
        finally:
            try:
                sock.close()
            except Exception:
                pass


if __name__ == "__main__":
    main()
"###;
