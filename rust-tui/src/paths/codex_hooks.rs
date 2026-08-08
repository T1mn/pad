use std::io;

mod feature {
    use super::toml_edit::{remove_toml_key_in_section, set_toml_bool_in_section};
    use super::version::{codex_hooks_feature_key_for_version, detect_codex_cli_version};
    use crate::paths::pad_codex_config_path;
    use std::fs;
    use std::io;

    pub(super) fn ensure_codex_feature_enabled() -> io::Result<()> {
        let path = pad_codex_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let existing = fs::read_to_string(&path).unwrap_or_default();
        let key = codex_hooks_feature_key_for_version(detect_codex_cli_version().as_deref());
        let mut updated = set_toml_bool_in_section(&existing, "features", key, true);
        if key == "hooks" {
            updated = remove_toml_key_in_section(&updated, "features", "codex_hooks");
        }

        if updated != existing {
            fs::write(path, updated)?;
        }

        Ok(())
    }
}
mod hooks_json {
    use crate::paths::{codex_hook_bridge_path, pad_codex_hooks_path};
    use std::fs;
    use std::io;

    pub(super) fn ensure_codex_hooks_json() -> io::Result<()> {
        let path = pad_codex_hooks_path();
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
        ensure_codex_hook_entry(hooks_map, "UserPromptSubmit", 15);
        ensure_codex_hook_entry(hooks_map, "Stop", 15);

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
        let mut already_present = false;
        for entry in arr.iter_mut() {
            let Some(hooks) = entry.get_mut("hooks").and_then(|v| v.as_array_mut()) else {
                continue;
            };
            for hook in hooks {
                let is_command = hook.get("type").and_then(|v| v.as_str()) == Some("command")
                    && hook.get("command").and_then(|v| v.as_str()) == Some(command.as_str());
                if is_command {
                    if let Some(obj) = hook.as_object_mut() {
                        obj.insert("timeout".into(), serde_json::json!(timeout));
                    }
                    already_present = true;
                }
            }
        }

        let already_present = already_present
            || arr.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|v| v.as_array())
                    .map(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("type").and_then(|v| v.as_str()) == Some("command")
                                && hook.get("command").and_then(|v| v.as_str())
                                    == Some(command.as_str())
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
}
mod toml_edit {
    pub(crate) fn set_toml_bool_in_section(
        content: &str,
        section: &str,
        key: &str,
        value: bool,
    ) -> String {
        let target_header = format!("[{}]", section);
        let new_line = format!("{} = {}", key, value);

        let mut result =
            String::with_capacity(content.len() + target_header.len() + new_line.len() + 4);
        let mut wrote_line = false;
        let mut in_target = false;
        let mut section_found = false;
        let mut key_written = false;
        let mut last_line_empty = true;

        for line in content.lines() {
            let trimmed = line.trim();
            let is_section = trimmed.starts_with('[') && trimmed.ends_with(']');

            if is_section && in_target && !key_written {
                push_toml_line(&mut result, &mut wrote_line, &new_line);
                key_written = true;
            }

            if trimmed == target_header {
                section_found = true;
                in_target = true;
                push_toml_line(&mut result, &mut wrote_line, line);
                last_line_empty = line.is_empty();
                continue;
            }

            if is_section {
                in_target = false;
            }

            if in_target && is_bare_key_assignment(trimmed, key) {
                push_toml_line(&mut result, &mut wrote_line, &new_line);
                key_written = true;
                last_line_empty = false;
            } else {
                push_toml_line(&mut result, &mut wrote_line, line);
                last_line_empty = line.is_empty();
            }
        }

        if section_found {
            if !key_written {
                if wrote_line && !last_line_empty {
                    push_toml_line(&mut result, &mut wrote_line, "");
                }
                push_toml_line(&mut result, &mut wrote_line, &new_line);
            }
        } else {
            if wrote_line && !last_line_empty {
                push_toml_line(&mut result, &mut wrote_line, "");
            }
            push_toml_line(&mut result, &mut wrote_line, &target_header);
            push_toml_line(&mut result, &mut wrote_line, &new_line);
        }

        finish_toml_result(result)
    }

    pub(crate) fn remove_toml_key_in_section(content: &str, section: &str, key: &str) -> String {
        let target_header = format!("[{}]", section);

        let mut result = String::with_capacity(content.len());
        let mut wrote_line = false;
        let mut in_target = false;

        for line in content.lines() {
            let trimmed = line.trim();
            let is_section = trimmed.starts_with('[') && trimmed.ends_with(']');

            if trimmed == target_header {
                in_target = true;
                push_toml_line(&mut result, &mut wrote_line, line);
                continue;
            }

            if is_section {
                in_target = false;
            }

            if in_target && is_bare_key_assignment(trimmed, key) {
                continue;
            }

            push_toml_line(&mut result, &mut wrote_line, line);
        }

        finish_toml_result(result)
    }

    fn is_bare_key_assignment(line: &str, key: &str) -> bool {
        line.strip_prefix(key)
            .is_some_and(|rest| rest.trim_start_matches([' ', '\t']).starts_with('='))
    }

    fn push_toml_line(result: &mut String, wrote_line: &mut bool, line: &str) {
        if *wrote_line {
            result.push('\n');
        }
        result.push_str(line);
        *wrote_line = true;
    }

    fn finish_toml_result(mut result: String) -> String {
        if !result.ends_with('\n') {
            result.push('\n');
        }
        result
    }
}
mod version {
    use std::process::Command;

    const NEW_HOOKS_FEATURE_VERSION: (u64, u64, u64) = (0, 130, 0);

    pub(crate) fn codex_hooks_feature_key_for_version(version: Option<&str>) -> &'static str {
        match version.and_then(parse_codex_cli_version) {
            Some(version) if version >= NEW_HOOKS_FEATURE_VERSION => "hooks",
            _ => "codex_hooks",
        }
    }

    pub(super) fn detect_codex_cli_version() -> Option<String> {
        Command::new("codex")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|raw| {
                raw.split_whitespace()
                    .rev()
                    .find(|token| parse_codex_cli_version(token).is_some())
                    .map(str::to_string)
            })
    }

    pub(crate) fn parse_codex_cli_version(raw: &str) -> Option<(u64, u64, u64)> {
        let clean = raw.trim().trim_start_matches('v');
        let mut parts = clean.split(['.', '-']);
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    }
}

use feature::ensure_codex_feature_enabled;
use hooks_json::ensure_codex_hooks_json;

#[cfg(test)]
pub(super) use toml_edit::{
    remove_toml_key_in_section as test_remove_toml_key_in_section,
    set_toml_bool_in_section as test_set_toml_bool_in_section,
};
#[cfg(test)]
pub(super) use version::{
    codex_hooks_feature_key_for_version as test_codex_hooks_feature_key_for_version,
    parse_codex_cli_version as test_parse_codex_cli_version,
};

pub(super) fn ensure_codex_hook_support() -> io::Result<()> {
    ensure_codex_feature_enabled()?;
    ensure_codex_hooks_json()?;
    Ok(())
}
