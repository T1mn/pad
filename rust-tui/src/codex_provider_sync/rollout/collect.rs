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
    let Some(updated_first_line) = rewrite_rollout_first_line(first_line, target_provider)? else {
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
