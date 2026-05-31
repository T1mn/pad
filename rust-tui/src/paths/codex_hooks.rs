use std::fs;
use std::io;

use super::{codex_hook_bridge_path, pad_codex_config_path, pad_codex_hooks_path};

mod toml_edit;
mod version;

use toml_edit::{remove_toml_key_in_section, set_toml_bool_in_section};
use version::{codex_hooks_feature_key_for_version, detect_codex_cli_version};

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

fn ensure_codex_feature_enabled() -> io::Result<()> {
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

fn ensure_codex_hooks_json() -> io::Result<()> {
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
