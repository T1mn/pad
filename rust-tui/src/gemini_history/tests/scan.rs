use super::super::{all_threads_at, sync_index_at};
use super::support::{sample_session_json, temp_db, temp_root, write_project_session};
use std::fs;

#[test]
fn invalid_snapshot_does_not_break_sync() {
    let root = temp_root("invalid-snapshot");
    let db = temp_db("invalid-snapshot");
    let valid = sample_session_json(
        "session-valid",
        "main",
        Some("Valid summary"),
        "2026-03-28T06:14:54.080Z",
        "prompt",
        "answer",
    );
    write_project_session(&root, "valid", "session-valid.json", &valid);
    write_project_session(&root, "broken", "session-broken.json", "{not-json");

    sync_index_at(&root, &db).unwrap();
    let threads = all_threads_at(&root, &db).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].session_id, "session-valid");
}

#[test]
fn indexed_rows_survive_when_source_snapshots_disappear() {
    let root = temp_root("source-snapshots-disappear");
    let db = temp_db("source-snapshots-disappear");
    let json = sample_session_json(
        "session-4",
        "main",
        Some("Persist me"),
        "2026-03-28T09:14:54.080Z",
        "prompt",
        "answer",
    );
    let path = write_project_session(&root, "rust-tui", "session-main.json", &json);

    sync_index_at(&root, &db).unwrap();
    assert_eq!(all_threads_at(&root, &db).unwrap().len(), 1);

    fs::remove_file(path).unwrap();
    sync_index_at(&root, &db).unwrap();
    let threads = all_threads_at(&root, &db).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].session_id, "session-4");
}
