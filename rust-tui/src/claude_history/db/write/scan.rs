use super::super::super::model::IndexedClaudeThread;
use super::super::super::util::{normalize_path, now_ts, to_io_error};
use rusqlite::{params, OptionalExtension};
use std::io;

pub(crate) fn next_scan_seq(tx: &rusqlite::Transaction<'_>, root_key: &str) -> io::Result<i64> {
    let current = tx
        .query_row(
            "SELECT scan_seq FROM claude_scan_state WHERE root = ?1",
            [root_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(to_io_error)?
        .unwrap_or(0);
    Ok(current.saturating_add(1))
}

pub(crate) fn upsert_thread_row(
    tx: &rusqlite::Transaction<'_>,
    root_key: &str,
    thread: &IndexedClaudeThread,
    file_mtime: i64,
    scan_seq: i64,
) -> io::Result<()> {
    tx.execute(
        "INSERT INTO claude_threads (
            root, transcript_path, session_id, cwd, title,
            updated_at, last_assistant_at, file_mtime, last_seen_seq,
            last_seen_at, is_sidechain
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)
         ON CONFLICT(root, transcript_path) DO UPDATE SET
            session_id = excluded.session_id,
            cwd = excluded.cwd,
            title = excluded.title,
            updated_at = excluded.updated_at,
            last_assistant_at = excluded.last_assistant_at,
            file_mtime = excluded.file_mtime,
            last_seen_seq = excluded.last_seen_seq,
            last_seen_at = excluded.last_seen_at,
               is_sidechain = excluded.is_sidechain",
        params![
            root_key,
            thread.transcript_path.to_string_lossy().to_string(),
            thread.session_id,
            normalize_path(&thread.cwd).to_string_lossy().to_string(),
            thread.title,
            thread.updated_at,
            thread.last_assistant_at,
            file_mtime,
            scan_seq,
            now_ts(),
        ],
    )
    .map_err(to_io_error)?;

    Ok(())
}
