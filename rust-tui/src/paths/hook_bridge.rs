use std::fs;
use std::io;
use std::path::Path;

use super::{claude_hook_bridge_path, codex_hook_bridge_path};

pub(super) const CLAUDE_BRIDGE_VERSION: &str = "claude-2026-04-08.1";
pub(super) const CODEX_BRIDGE_VERSION: &str = "codex-2026-05-31.1";
const BRIDGE_VERSION_PREFIX: &str = "# pad-bridge-version: ";

pub(super) fn install_bridge_scripts() -> io::Result<()> {
    let claude_bridge = claude_hook_bridge_template();
    let codex_bridge = codex_hook_bridge_template();
    install_bridge_script(&claude_hook_bridge_path(), claude_bridge.as_str(), false)?;
    install_bridge_script(&codex_hook_bridge_path(), codex_bridge.as_str(), true)?;
    Ok(())
}

pub(super) fn log_bridge_statuses() {
    log_bridge_status(
        "claude",
        &claude_hook_bridge_path(),
        CLAUDE_BRIDGE_VERSION,
        false,
    );
    log_bridge_status(
        "codex",
        &codex_hook_bridge_path(),
        CODEX_BRIDGE_VERSION,
        true,
    );
}

fn install_bridge_script(path: &Path, desired: &str, require_turn_id: bool) -> io::Result<()> {
    let existing = fs::read_to_string(path).ok();

    if existing.as_deref() != Some(desired) {
        fs::write(path, desired)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    let actual = fs::read_to_string(path)?;
    if actual != desired {
        return Err(io::Error::other(format!(
            "bridge script verify failed at {}",
            path.display()
        )));
    }
    if require_turn_id && !actual.contains("\"turn_id\": payload.get(\"turn_id\")") {
        return Err(io::Error::other(format!(
            "bridge script missing turn_id forwarding at {}",
            path.display()
        )));
    }

    Ok(())
}

fn log_bridge_status(role: &str, path: &Path, expected_version: &str, expect_turn_id: bool) {
    match fs::read_to_string(path) {
        Ok(content) => {
            let actual_version = bridge_version(&content).unwrap_or("missing");
            let has_turn_id = content.contains("\"turn_id\": payload.get(\"turn_id\")");
            crate::log_debug!(
                "runtime_layout: bridge role={} path={} expected_version={} actual_version={} has_turn_id={}",
                role,
                path.display(),
                expected_version,
                actual_version,
                has_turn_id
            );
            if actual_version != expected_version {
                crate::log_debug!(
                    "runtime_layout: bridge version mismatch role={} expected={} actual={}",
                    role,
                    expected_version,
                    actual_version
                );
            }
            if expect_turn_id && !has_turn_id {
                crate::log_debug!(
                    "runtime_layout: bridge missing turn_id forwarding role={} path={}",
                    role,
                    path.display()
                );
            }
        }
        Err(err) => {
            crate::log_debug!(
                "runtime_layout: bridge status read failed role={} path={} err={}",
                role,
                path.display(),
                err
            );
        }
    }
}

fn bridge_version(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.strip_prefix(BRIDGE_VERSION_PREFIX))
        .map(str::trim)
}

struct HookBridgeTemplateOptions {
    version: &'static str,
    silence_stdio_block: &'static str,
    tmux_stderr_arg: &'static str,
    load_payload_block: &'static str,
    main_start_line: &'static str,
    payload_expr: &'static str,
    hook_type_line: &'static str,
    event_name_expr: &'static str,
    record_turn_diff_block: String,
}

pub(super) fn claude_hook_bridge_template() -> String {
    build_hook_bridge_template(HookBridgeTemplateOptions {
        version: CLAUDE_BRIDGE_VERSION,
        silence_stdio_block: "",
        tmux_stderr_arg: "",
        load_payload_block: "",
        main_start_line: "    raw = sys.stdin.read()",
        payload_expr: "json.loads(raw)",
        hook_type_line: "",
        event_name_expr: "payload.get(\"hook_event_name\")",
        record_turn_diff_block: "def record_codex_turn_diff(message):\n    pass\n".into(),
    })
}

pub(super) fn codex_hook_bridge_template() -> String {
    let pad_binary = std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    build_hook_bridge_template(HookBridgeTemplateOptions {
        version: CODEX_BRIDGE_VERSION,
        silence_stdio_block: r#"def silence_stdio():
    devnull = open(os.devnull, "w")
    sys.stdout = devnull
    sys.stderr = devnull
"#,
        tmux_stderr_arg: ",\n            stderr=subprocess.DEVNULL",
        load_payload_block: r#"def load_payload():
    if sys.stdin.isatty():
        return {}
    try:
        return json.load(sys.stdin)
    except Exception:
        return {}
"#,
        main_start_line: "    silence_stdio()",
        payload_expr: "load_payload()",
        hook_type_line:
            "    hook_type = sys.argv[1] if len(sys.argv) > 1 else payload.get(\"hook_event_name\")",
        event_name_expr: "payload.get(\"hook_event_name\") or hook_type",
        record_turn_diff_block: format!(
            r#"PAD_BINARY = {pad_binary:?}


def record_codex_turn_diff(message):
    if message.get("event") not in ("user_prompt_submit", "stop"):
        return
    if not PAD_BINARY:
        return
    try:
        subprocess.run(
            [PAD_BINARY, "__internal", "codex-turn-diff", "hook"],
            input=json.dumps(message, ensure_ascii=False),
            text=True,
            timeout=12,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass
"#
        ),
    })
}

fn build_hook_bridge_template(options: HookBridgeTemplateOptions) -> String {
    HOOK_BRIDGE_TEMPLATE_BASE
        .replace("__PAD_BRIDGE_VERSION__", options.version)
        .replace("__PAD_SILENCE_STDIO_BLOCK__", options.silence_stdio_block)
        .replace("__PAD_TMUX_STDERR_ARG__", options.tmux_stderr_arg)
        .replace("__PAD_LOAD_PAYLOAD_BLOCK__", options.load_payload_block)
        .replace("__PAD_MAIN_START_LINE__", options.main_start_line)
        .replace("__PAD_PAYLOAD_EXPR__", options.payload_expr)
        .replace("__PAD_HOOK_TYPE_LINE__", options.hook_type_line)
        .replace("__PAD_EVENT_NAME_EXPR__", options.event_name_expr)
        .replace(
            "__PAD_RECORD_TURN_DIFF_BLOCK__",
            &options.record_turn_diff_block,
        )
}

const HOOK_BRIDGE_TEMPLATE_BASE: &str = r###"#!/usr/bin/env python3
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
            text=True__PAD_TMUX_STDERR_ARG__,
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
    payload = __PAD_PAYLOAD_EXPR__
__PAD_HOOK_TYPE_LINE__
    tmux = tmux_info_from_env()
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
        "tmux": tmux,
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
