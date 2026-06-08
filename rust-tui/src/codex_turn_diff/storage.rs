mod completed;
mod json;
mod pending;

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
    let root_label = root.to_string_lossy().to_string();

    let mut entries = Vec::new();
    entries.extend(
        pending::list_pending_all()?
            .into_iter()
            .filter(|pending| pending.repo_root == root_label)
            .map(TurnDiffEntry::from),
    );
    entries.extend(
        completed::list_completed_all()?
            .into_iter()
            .filter(|record| record.repo_root == root_label)
            .map(TurnDiffEntry::from),
    );

    entries.sort_by(|left, right| {
        right
            .sort_key()
            .cmp(&left.sort_key())
            .then_with(|| right.id.cmp(&left.id))
    });
    if entries.len() > limit {
        entries.truncate(limit);
    }
    Ok(entries)
}
