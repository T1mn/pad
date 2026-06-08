use rusqlite::Connection;
use std::fs;
use std::path::Path;

pub(super) fn temp_db_path() -> std::path::PathBuf {
    crate::test_support::temp_path("pad-codex-state", "db")
}

pub(super) fn temp_codex_home() -> std::path::PathBuf {
    crate::test_support::temp_path("pad-codex-home", "root")
}

pub(super) fn temp_rollout_path(name: &str) -> std::path::PathBuf {
    crate::test_support::temp_path("pad-codex-rollout", name)
}

pub(super) fn sample_rollout_name(thread_id: &str) -> String {
    format!("rollout-2026-03-27T14-05-10-{}.jsonl", thread_id)
}

pub(super) fn write_rollout(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, "{\"type\":\"message\"}\n").unwrap();
}

pub(super) fn cleanup_file(path: &Path) {
    fs::remove_file(path).ok();
}

pub(super) fn cleanup_dir(path: &Path) {
    fs::remove_dir_all(path).ok();
}

pub(super) fn create_threads_db(path: &Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE threads (
                id TEXT PRIMARY KEY,
                cwd TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                rollout_path TEXT NOT NULL,
                title TEXT,
                first_user_message TEXT,
                source TEXT,
                archived INTEGER NOT NULL DEFAULT 0,
                archived_at INTEGER
            );",
        )
        .unwrap();
}

pub(super) fn insert_thread(
    connection: &Connection,
    thread_id: &str,
    cwd: &str,
    updated_at: i64,
    rollout_path: &Path,
    archived: bool,
) {
    connection
        .execute(
            "INSERT INTO threads (
                id, cwd, updated_at, rollout_path, title, first_user_message, source, archived, archived_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                thread_id,
                cwd,
                updated_at,
                rollout_path.to_string_lossy().to_string(),
                "hello",
                "hello",
                "cli",
                if archived { 1_i64 } else { 0_i64 },
                Option::<i64>::None,
            ),
        )
        .unwrap();
}

pub(super) fn thread_rollout_path(connection: &Connection, thread_id: &str) -> String {
    connection
        .query_row(
            "SELECT rollout_path FROM threads WHERE id = ?1",
            [thread_id],
            |row| row.get(0),
        )
        .unwrap()
}

pub(super) fn thread_archive_state(
    connection: &Connection,
    thread_id: &str,
) -> (bool, Option<i64>, i64) {
    connection
        .query_row(
            "SELECT archived, archived_at, updated_at FROM threads WHERE id = ?1",
            [thread_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .unwrap()
}
