use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SESSION_DIRS: [&str; 2] = ["sessions", "archived_sessions"];

#[derive(Clone, Debug)]
pub(super) struct RolloutChange {
    pub(super) path: PathBuf,
    original_first_line: String,
    original_separator: String,
    updated_first_line: String,
}

pub(super) fn collect_rollout_changes(
    codex_home: &Path,
    target_provider: &str,
) -> io::Result<Vec<RolloutChange>> {
    let mut changes = Vec::new();

    for scope in SESSION_DIRS {
        let root = codex_home.join(scope);
        if !root.exists() {
            continue;
        }
        collect_rollout_changes_in_dir(&root, target_provider, &mut changes)?;
    }

    Ok(changes)
}

pub(super) fn collect_rollout_changes_in_dir(
    dir: &Path,
    target_provider: &str,
    changes: &mut Vec<RolloutChange>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_rollout_changes_in_dir(&path, target_provider, changes)?;
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("rollout-") || !file_name.ends_with(".jsonl") {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let Some((first_line, separator, _rest)) = split_first_line(&content) else {
            continue;
        };
        let Some(updated_first_line) = rewrite_rollout_first_line(first_line, target_provider)?
        else {
            continue;
        };
        changes.push(RolloutChange {
            path,
            original_first_line: first_line.to_string(),
            original_separator: separator.to_string(),
            updated_first_line,
        });
    }

    Ok(())
}

fn rewrite_rollout_first_line(
    first_line: &str,
    target_provider: &str,
) -> io::Result<Option<String>> {
    if first_line.trim().is_empty() {
        return Ok(None);
    }

    let mut value = match serde_json::from_str::<Value>(first_line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let is_session_meta = value
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "session_meta");
    if !is_session_meta {
        return Ok(None);
    }

    let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut) else {
        return Ok(None);
    };

    let current_provider = payload
        .get("model_provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if current_provider == target_provider {
        return Ok(None);
    }

    payload.insert(
        "model_provider".to_string(),
        Value::String(target_provider.to_string()),
    );
    serde_json::to_string(&value)
        .map(Some)
        .map_err(|err| io::Error::other(err.to_string()))
}

pub(super) fn apply_rollout_changes(changes: &[RolloutChange]) -> io::Result<usize> {
    let mut updated = 0usize;
    for change in changes {
        apply_rollout_change(change)?;
        updated += 1;
    }
    Ok(updated)
}

fn apply_rollout_change(change: &RolloutChange) -> io::Result<()> {
    let current = fs::read_to_string(&change.path)?;
    let Some((first_line, separator, rest)) = split_first_line(&current) else {
        return Err(io::Error::other(format!(
            "rollout file is missing first line: {}",
            change.path.display()
        )));
    };

    if first_line != change.original_first_line || separator != change.original_separator {
        return Err(io::Error::other(format!(
            "rollout file changed during provider sync: {}",
            change.path.display()
        )));
    }

    let updated_content = format!(
        "{}{}{}",
        change.updated_first_line, change.original_separator, rest
    );
    let tmp_path = change
        .path
        .with_extension(format!("jsonl.pad-sync.{}", std::process::id()));
    fs::write(&tmp_path, updated_content)?;
    fs::rename(tmp_path, &change.path)?;
    Ok(())
}

fn split_first_line(content: &str) -> Option<(&str, &str, &str)> {
    if let Some(index) = content.find("\r\n") {
        let rest = &content[index + 2..];
        return Some((&content[..index], "\r\n", rest));
    }
    if let Some(index) = content.find('\n') {
        let rest = &content[index + 1..];
        return Some((&content[..index], "\n", rest));
    }
    Some((content, "", ""))
}
