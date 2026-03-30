use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn pad_home_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pad")
}

pub fn config_path() -> PathBuf {
    pad_home_dir().join("config.toml")
}

pub fn pad_db_path() -> PathBuf {
    pad_home_dir().join("pad.db")
}

pub fn legacy_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        })
        .join("pad")
        .join("config.toml")
}

pub fn logs_dir() -> PathBuf {
    pad_home_dir().join("logs")
}

pub fn log_path() -> PathBuf {
    logs_dir().join("pad.log")
}

pub fn telegram_bot_log_path() -> PathBuf {
    logs_dir().join("telegram-bot.log")
}

pub fn hook_events_path() -> PathBuf {
    logs_dir().join("hook-events.jsonl")
}

pub fn scripts_dir() -> PathBuf {
    pad_home_dir().join("scripts")
}

pub fn sessions_dir() -> PathBuf {
    pad_home_dir().join("sessions")
}

pub fn sessions_index_path() -> PathBuf {
    sessions_dir().join("index.json")
}

pub fn claude_hook_bridge_path() -> PathBuf {
    scripts_dir().join("claude_hook_bridge.py")
}

pub fn codex_hook_bridge_path() -> PathBuf {
    scripts_dir().join("codex_hook_bridge.py")
}

pub fn hook_socket_path() -> PathBuf {
    pad_home_dir().join("pad-hook.sock")
}

pub fn pad_status_path() -> PathBuf {
    pad_home_dir().join("pad-status.json")
}

pub fn telegram_bot_status_path() -> PathBuf {
    pad_home_dir().join("telegram-bot-status.json")
}

pub fn telegram_state_path() -> PathBuf {
    pad_home_dir().join("telegram-state.json")
}

pub fn telegram_hook_socket_path() -> PathBuf {
    pad_home_dir().join("telegram-hook.sock")
}

pub fn ensure_runtime_layout() -> io::Result<()> {
    fs::create_dir_all(pad_home_dir())?;
    fs::create_dir_all(logs_dir())?;
    fs::create_dir_all(scripts_dir())?;
    fs::create_dir_all(sessions_dir())?;
    if !hook_events_path().exists() {
        fs::write(hook_events_path(), "")?;
    }
    install_bridge_script(&claude_hook_bridge_path(), CLAUDE_HOOK_BRIDGE_TEMPLATE)?;
    install_bridge_script(&codex_hook_bridge_path(), CODEX_HOOK_BRIDGE_TEMPLATE)?;
    ensure_codex_hook_support()?;
    crate::thread_meta::ensure_db()?;
    Ok(())
}

fn install_bridge_script(path: &Path, desired: &str) -> io::Result<()> {
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

    Ok(())
}

fn ensure_codex_hook_support() -> io::Result<()> {
    ensure_codex_feature_enabled()?;
    ensure_codex_hooks_json()?;
    Ok(())
}

fn ensure_codex_feature_enabled() -> io::Result<()> {
    let path = codex_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = fs::read_to_string(&path).unwrap_or_default();
    let updated = set_toml_bool_in_section(&existing, "features", "codex_hooks", true);

    if updated != existing {
        fs::write(path, updated)?;
    }

    Ok(())
}

fn ensure_codex_hooks_json() -> io::Result<()> {
    let path = codex_hooks_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut root = serde_json::from_str::<serde_json::Value>(&existing)
        .unwrap_or_else(|_| serde_json::json!({}));

    if !root.is_object() {
        root = serde_json::json!({});
    }

    let hooks_obj = root
        .as_object_mut()
        .expect("root object")
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    if !hooks_obj.is_object() {
        *hooks_obj = serde_json::json!({});
    }

    let hooks_map = hooks_obj.as_object_mut().expect("hooks object");
    ensure_codex_hook_entry(hooks_map, "SessionStart", 8);
    ensure_codex_hook_entry(hooks_map, "UserPromptSubmit", 5);
    ensure_codex_hook_entry(hooks_map, "Stop", 5);

    let formatted = serde_json::to_string_pretty(&root)?;
    if formatted != existing {
        fs::write(path, formatted)?;
    }

    Ok(())
}

fn ensure_codex_hook_entry(
    hooks_map: &mut serde_json::Map<String, serde_json::Value>,
    event: &str,
    timeout: u64,
) {
    let command = format!(
        "python3 \"{}\" {}",
        codex_hook_bridge_path().to_string_lossy(),
        event
    );

    let entries = hooks_map
        .entry(event.to_string())
        .or_insert_with(|| serde_json::json!([]));

    if !entries.is_array() {
        *entries = serde_json::json!([]);
    }

    let arr = entries.as_array_mut().expect("array");
    let already_present = arr.iter().any(|entry| {
        entry
            .get("hooks")
            .and_then(|v| v.as_array())
            .map(|hooks| {
                hooks.iter().any(|hook| {
                    hook.get("type").and_then(|v| v.as_str()) == Some("command")
                        && hook.get("command").and_then(|v| v.as_str()) == Some(command.as_str())
                })
            })
            .unwrap_or(false)
    });

    if !already_present {
        arr.push(serde_json::json!({
            "hooks": [
                {
                    "type": "command",
                    "command": command,
                    "timeout": timeout
                }
            ]
        }));
    }
}

fn codex_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("config.toml")
}

fn codex_hooks_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("hooks.json")
}

fn set_toml_bool_in_section(content: &str, section: &str, key: &str, value: bool) -> String {
    let target_header = format!("[{}]", section);
    let key_prefix = format!("{} =", key);
    let new_line = format!("{} = {}", key, value);

    let mut lines: Vec<String> = Vec::new();
    let mut in_target = false;
    let mut section_found = false;
    let mut key_written = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let is_section = trimmed.starts_with('[') && trimmed.ends_with(']');

        if is_section && in_target && !key_written {
            lines.push(new_line.clone());
            key_written = true;
        }

        if trimmed == target_header {
            section_found = true;
            in_target = true;
            lines.push(line.to_string());
            continue;
        }

        if is_section {
            in_target = false;
        }

        if in_target && trimmed.starts_with(&key_prefix) {
            lines.push(new_line.clone());
            key_written = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if section_found {
        if !key_written {
            if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
                lines.push(String::new());
            }
            lines.push(new_line);
        }
    } else {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(target_header);
        lines.push(new_line);
    }

    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

const CLAUDE_HOOK_BRIDGE_TEMPLATE: &str = r###"#!/usr/bin/env python3
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


def normalized_event_name(name):
    if name == "UserPromptSubmit":
        return "user_prompt_submit"
    if name == "SessionStart":
        return "session_start"
    if name == "Stop":
        return "stop"
    return (name or "unknown").lower()


def main():
    raw = sys.stdin.read()
    payload = json.loads(raw)
    tmux = tmux_info_from_env()
    event_name = payload.get("hook_event_name")

    message = {
        "event": normalized_event_name(event_name),
        "hook_event_name": event_name,
        "session_id": payload.get("session_id"),
        "transcript_path": payload.get("transcript_path"),
        "cwd": payload.get("cwd"),
        "prompt": payload.get("prompt"),
        "last_assistant_message": payload.get("last_assistant_message"),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "tmux": tmux,
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
"###;

const CODEX_HOOK_BRIDGE_TEMPLATE: &str = r###"#!/usr/bin/env python3
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


def normalized_event_name(name):
    if name == "UserPromptSubmit":
        return "user_prompt_submit"
    if name == "SessionStart":
        return "session_start"
    if name == "Stop":
        return "stop"
    return (name or "unknown").lower()


def load_payload():
    if sys.stdin.isatty():
        return {}
    try:
        return json.load(sys.stdin)
    except Exception:
        return {}


def main():
    payload = load_payload()
    hook_type = sys.argv[1] if len(sys.argv) > 1 else payload.get("hook_event_name")
    event_name = payload.get("hook_event_name") or hook_type
    tmux = tmux_info_from_env()

    message = {
        "event": normalized_event_name(event_name),
        "hook_event_name": event_name,
        "session_id": payload.get("session_id"),
        "transcript_path": payload.get("transcript_path"),
        "cwd": payload.get("cwd"),
        "prompt": payload.get("prompt"),
        "last_assistant_message": payload.get("last_assistant_message"),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "tmux": tmux,
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

    print(json.dumps({"suppressOutput": True}))


if __name__ == "__main__":
    main()
"###;
