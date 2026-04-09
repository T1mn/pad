use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CLAUDE_BRIDGE_VERSION: &str = "claude-2026-04-08.1";
const CODEX_BRIDGE_VERSION: &str = "codex-2026-04-08.2";
const BRIDGE_VERSION_PREFIX: &str = "# pad-bridge-version: ";

pub fn pad_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
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
    let claude_bridge = claude_hook_bridge_template();
    let codex_bridge = codex_hook_bridge_template();
    install_bridge_script(&claude_hook_bridge_path(), claude_bridge.as_str(), false)?;
    install_bridge_script(&codex_hook_bridge_path(), codex_bridge.as_str(), true)?;
    ensure_codex_hook_support()?;
    crate::thread_meta::ensure_db()?;
    Ok(())
}

pub fn log_runtime_layout_status() {
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
}

fn claude_hook_bridge_template() -> String {
    build_hook_bridge_template(HookBridgeTemplateOptions {
        version: CLAUDE_BRIDGE_VERSION,
        silence_stdio_block: "",
        tmux_stderr_arg: "",
        load_payload_block: "",
        main_start_line: "    raw = sys.stdin.read()",
        payload_expr: "json.loads(raw)",
        hook_type_line: "",
        event_name_expr: "payload.get(\"hook_event_name\")",
    })
}

fn codex_hook_bridge_template() -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_bridge_template_stays_minimal_and_forwards_turn_id() {
        let template = claude_hook_bridge_template();
        assert!(template.contains(&format!("# pad-bridge-version: {}", CLAUDE_BRIDGE_VERSION)));
        assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
        assert!(!template.contains("def silence_stdio():"));
        assert!(!template.contains("def load_payload():"));
        assert!(!template.contains("stderr=subprocess.DEVNULL"));
    }

    #[test]
    fn codex_bridge_template_keeps_required_stdin_and_turn_id_handling() {
        let template = codex_hook_bridge_template();
        assert!(template.contains(&format!("# pad-bridge-version: {}", CODEX_BRIDGE_VERSION)));
        assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
        assert!(template.contains("def silence_stdio():"));
        assert!(template.contains("def load_payload():"));
        assert!(template.contains("stderr=subprocess.DEVNULL"));
        assert!(template.contains("payload.get(\"hook_event_name\") or hook_type"));
    }
}
