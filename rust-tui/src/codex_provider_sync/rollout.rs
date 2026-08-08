mod apply;
mod collect {
    use super::line::split_first_line;
    use super::rewrite::rewrite_rollout_first_line;
    use super::RolloutChange;
    use std::fs::{self, DirEntry};
    use std::io;
    use std::path::{Path, PathBuf};

    const SESSION_DIRS: [&str; 2] = ["sessions", "archived_sessions"];

    pub(in crate::codex_provider_sync) fn collect_rollout_changes(
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

    fn collect_rollout_changes_in_dir(
        dir: &Path,
        target_provider: &str,
        changes: &mut Vec<RolloutChange>,
    ) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_dir() {
                collect_rollout_changes_in_dir(&path, target_provider, changes)?;
                continue;
            }
            if !kind.is_file() || !is_rollout_jsonl(&entry) {
                continue;
            }
            if let Some(change) = rollout_change_for_file(path, target_provider)? {
                changes.push(change);
            }
        }

        Ok(())
    }

    fn rollout_change_for_file(
        path: PathBuf,
        target_provider: &str,
    ) -> io::Result<Option<RolloutChange>> {
        let content = fs::read_to_string(&path)?;
        let (first_line, separator, _rest) = split_first_line(&content);
        let Some(updated_first_line) = rewrite_rollout_first_line(first_line, target_provider)?
        else {
            return Ok(None);
        };
        Ok(Some(RolloutChange {
            path,
            original_first_line: first_line.to_string(),
            original_separator: separator.to_string(),
            updated_first_line,
        }))
    }

    fn is_rollout_jsonl(entry: &DirEntry) -> bool {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return false;
        };
        name.starts_with("rollout-") && name.ends_with(".jsonl")
    }
}
mod line {
    pub(super) fn split_first_line(content: &str) -> (&str, &str, &str) {
        if let Some(index) = content.find("\r\n") {
            let rest = &content[index + 2..];
            return (&content[..index], "\r\n", rest);
        }
        if let Some(index) = content.find('\n') {
            let rest = &content[index + 1..];
            return (&content[..index], "\n", rest);
        }
        (content, "", "")
    }
}
mod model {
    use std::path::PathBuf;

    #[derive(Clone, Debug)]
    pub(in crate::codex_provider_sync) struct RolloutChange {
        pub(in crate::codex_provider_sync) path: PathBuf,
        pub(in crate::codex_provider_sync::rollout) original_first_line: String,
        pub(in crate::codex_provider_sync::rollout) original_separator: String,
        pub(in crate::codex_provider_sync::rollout) updated_first_line: String,
    }
}
mod rewrite {
    use serde_json::Value;
    use std::io;

    pub(super) fn rewrite_rollout_first_line(
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
        if !is_session_meta(&value) {
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

    fn is_session_meta(value: &Value) -> bool {
        value
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|value| value == "session_meta")
    }
}

pub(in crate::codex_provider_sync) use apply::apply_rollout_changes;
pub(in crate::codex_provider_sync) use collect::collect_rollout_changes;
pub(in crate::codex_provider_sync::rollout) use model::RolloutChange;
