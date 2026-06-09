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
