mod completed {
    use super::json::{read_json_dir, write_json};
    use super::paths::{index_path, patches_dir, record_path, records_dir};
    use crate::codex_turn_diff::model::CompletedTurnDiff;
    use std::fs::{self, OpenOptions};
    use std::io::{self, Write};

    pub(super) fn save_completed(
        mut record: CompletedTurnDiff,
        patch: &str,
    ) -> io::Result<CompletedTurnDiff> {
        fs::create_dir_all(records_dir())?;
        fs::create_dir_all(patches_dir())?;

        let patch_path = patches_dir().join(format!("{}.patch", record.id));
        fs::write(&patch_path, patch)?;
        record.patch_path = patch_path.to_string_lossy().into_owned();

        write_json(&record_path(&record.id), &record)?;
        append_index(&record)?;
        Ok(record)
    }

    pub(super) fn list_completed_all() -> io::Result<Vec<CompletedTurnDiff>> {
        let from_index = read_index_records()?;
        if !from_index.is_empty() {
            return Ok(from_index);
        }
        read_json_dir(&records_dir())
    }

    fn read_index_records() -> io::Result<Vec<CompletedTurnDiff>> {
        let path = index_path();
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        Ok(content
            .lines()
            .filter_map(|line| serde_json::from_str::<CompletedTurnDiff>(line).ok())
            .collect())
    }

    fn append_index(record: &CompletedTurnDiff) -> io::Result<()> {
        if let Some(parent) = index_path().parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(index_path())?;
        writeln!(file, "{}", serde_json::to_string(record)?)?;
        Ok(())
    }
}
mod json {
    use serde::de::DeserializeOwned;
    use std::fs;
    use std::io;
    use std::path::Path;

    pub(super) fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(value)?)
    }

    pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(io::Error::other)
    }

    pub(super) fn read_json_dir<T: DeserializeOwned>(dir: &Path) -> io::Result<Vec<T>> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            if let Ok(value) = read_json(&path) {
                out.push(value);
            }
        }
        Ok(out)
    }
}
mod pending {
    use super::json::{read_json, read_json_dir, write_json};
    use super::paths::{event_key, pending_dir, pending_path};
    use crate::codex_turn_diff::model::PendingTurnDiff;
    use crate::hook::HookEvent;
    use std::fs;
    use std::io;

    pub fn save_pending(pending: &PendingTurnDiff) -> io::Result<()> {
        fs::create_dir_all(pending_dir())?;
        write_json(&pending_path(&pending.id), pending)
    }

    pub fn load_pending_for_stop(event: &HookEvent) -> io::Result<Option<PendingTurnDiff>> {
        if let Some(key) = event_key(event) {
            let path = pending_path(&key);
            if path.exists() {
                return read_json(&path).map(Some);
            }
        }

        Ok(list_pending_all()?
            .into_iter()
            .filter(|pending| pending_matches_event(pending, event))
            .max_by(|left, right| left.started_at.cmp(&right.started_at)))
    }

    pub fn remove_pending(id: &str) -> io::Result<()> {
        let path = pending_path(id);
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub(super) fn list_pending_all() -> io::Result<Vec<PendingTurnDiff>> {
        read_json_dir(&pending_dir())
    }

    fn pending_matches_event(pending: &PendingTurnDiff, event: &HookEvent) -> bool {
        if let Some(turn_id) = event.turn_id.as_deref().filter(|value| !value.is_empty()) {
            return pending.turn_id.as_deref() == Some(turn_id);
        }
        if let Some(session_id) = event
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            if pending.session_id.as_deref() != Some(session_id) {
                return false;
            }
        }
        if let Some(pane_id) = event
            .tmux
            .pane_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            if pending.pane_id.as_deref() != Some(pane_id) {
                return false;
            }
        }
        event.session_id.is_some() || event.tmux.pane_id.is_some()
    }
}

use super::git::repo_root_for_cwd;
use super::model::TurnDiffEntry;
use super::storage_paths as paths;
use std::fs;
use std::io;
use std::path::Path;

pub use paths::{event_key, new_record_id, now_stamp};
pub use pending::{load_pending_for_stop, remove_pending, save_pending};

pub fn save_completed(
    record: super::model::CompletedTurnDiff,
    patch: &str,
) -> io::Result<super::model::CompletedTurnDiff> {
    completed::save_completed(record, patch)
}

pub fn list_for_cwd(cwd: &Path, limit: usize) -> io::Result<Vec<TurnDiffEntry>> {
    let root = repo_root_for_cwd(cwd).or_else(|_| fs::canonicalize(cwd))?;
    let root_label = root.to_string_lossy();

    let mut entries = Vec::new();
    entries.extend(
        pending::list_pending_all()?
            .into_iter()
            .filter(|pending| pending.repo_root == root_label.as_ref())
            .map(TurnDiffEntry::from),
    );
    entries.extend(
        completed::list_completed_all()?
            .into_iter()
            .filter(|record| record.repo_root == root_label.as_ref())
            .map(TurnDiffEntry::from),
    );

    entries.sort_by(|left, right| {
        right
            .sort_key()
            .cmp(left.sort_key())
            .then_with(|| right.id.cmp(&left.id))
    });
    if entries.len() > limit {
        entries.truncate(limit);
    }
    Ok(entries)
}
