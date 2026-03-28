#!/usr/bin/env python3
import json
import os
import socket
import subprocess
import sys
from pathlib import Path
from datetime import datetime, timezone

SOCKET_PATH = "/tmp/pad-hook.sock"


def tmux_info_from_env():
    tmux_pane = os.environ.get("TMUX_PANE")
    if not tmux_pane:
        return {
            "inside_tmux": False,
            "pane_id": None,
        }

    fmt = json.dumps({
        "session_name": "#{session_name}",
        "session_id": "#{session_id}",
        "window_index": "#{window_index}",
        "window_name": "#{window_name}",
        "pane_index": "#{pane_index}",
        "pane_id": "#{pane_id}",
        "pane_current_command": "#{pane_current_command}",
        "pane_current_path": "#{pane_current_path}",
    })

    try:
        out = subprocess.check_output(
            ["tmux", "display-message", "-p", "-t", tmux_pane, fmt],
            text=True,
        ).strip()
        info = json.loads(out)
        info["inside_tmux"] = True
        info["tmux_pane_env"] = tmux_pane
        return info
    except Exception as e:
        return {
            "inside_tmux": True,
            "tmux_pane_env": tmux_pane,
            "tmux_error": str(e),
            "pane_id": tmux_pane,
        }


def main():
    raw = sys.stdin.read()
    payload = json.loads(raw)
    tmux = tmux_info_from_env()

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
        "tmux": tmux,
    }

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.settimeout(0.5)
    try:
        sock.connect(SOCKET_PATH)
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
