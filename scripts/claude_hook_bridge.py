#!/usr/bin/env python3
import json
import os
import socket
import sys
from pathlib import Path
from datetime import datetime, timezone

PAD_HOME = Path(os.environ.get("PAD_HOME", Path.home() / ".pad")).expanduser()
SOCKET_PATHS = [
    PAD_HOME / "pad-hook.sock",
    PAD_HOME / "telegram-hook.sock",
]


def terminal_info_from_env():
    return {
        "runtime": "native",
        "pane_id": os.environ.get("PAD_PANE_ID"),
    }


def main():
    raw = sys.stdin.read()
    payload = json.loads(raw)
    terminal = terminal_info_from_env()

    event_name = payload.get("hook_event_name")
    if event_name == "UserPromptSubmit":
        event = "user_prompt_submit"
    elif event_name == "Stop":
        event = "stop"
    else:
        event = event_name or "unknown"

    message = {
        "event": event,
        "hook_event_name": event_name,
        "claude_session_id": payload.get("session_id"),
        "transcript_path": payload.get("transcript_path"),
        "cwd": payload.get("cwd"),
        "prompt": payload.get("prompt"),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "terminal": terminal,
    }

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
