use super::super::model::ThreadRow;
use super::super::util::{to_io_error, unix_now_ts};
use rusqlite::Connection;
use std::io;
use std::path::Path;

pub(super) fn read_thread_for_update(
    connection: &Connection,
    thread_id: &str,
) -> io::Result<ThreadRow> {
    connection
        .query_row(
            "SELECT rollout_path, archived FROM threads WHERE id = ?1",
            [thread_id],
            |row| {
                Ok(ThreadRow {
                    rollout_path: row.get::<_, String>(0)?,
                    archived: row.get::<_, i64>(1)? != 0,
                })
            },
        )
        .map_err(to_io_error)
}

pub(super) fn update_archived_thread(
    connection: &Connection,
    thread_id: &str,
    target_path: &Path,
) -> rusqlite::Result<usize> {
    let archived_at = unix_now_ts();
    connection.execute(
        "UPDATE threads
         SET archived = 1, archived_at = ?1, rollout_path = ?2
         WHERE id = ?3 AND archived = 0",
        (
            archived_at,
            target_path.to_string_lossy().to_string(),
            thread_id.to_string(),
        ),
    )
}

pub(super) fn update_unarchived_thread(
    connection: &Connection,
    thread_id: &str,
    target_path: &Path,
) -> rusqlite::Result<usize> {
    let updated_at = unix_now_ts();
    connection.execute(
        "UPDATE threads
         SET archived = 0, archived_at = NULL, rollout_path = ?1, updated_at = ?2
         WHERE id = ?3 AND archived = 1",
        (
            target_path.to_string_lossy().to_string(),
            updated_at,
            thread_id.to_string(),
        ),
    )
}
