mod archive {
    use super::super::super::util::{normalize_path, now_ts, to_io_error};
    use super::super::schema::{ensure_schema, open_index_db};
    use rusqlite::params;
    use std::io;
    use std::path::Path;

    pub(crate) fn mutate_thread_archive_state_at(
        root: &Path,
        db_path: &Path,
        session_id: &str,
        archive: bool,
    ) -> io::Result<()> {
        let root_key = normalize_path(root).to_string_lossy().to_string();
        let connection = open_index_db(db_path)?;
        ensure_schema(&connection)?;
        let updated_at = now_ts();
        let archived_value = if archive { 1_i64 } else { 0_i64 };

        let changed = connection
            .execute(
                "UPDATE claude_threads
                 SET archived = ?3,
                     archived_at = CASE WHEN ?3 = 1 THEN ?4 ELSE NULL END,
                     updated_at = MAX(updated_at, ?4)
                 WHERE root = ?1
                   AND session_id = ?2
                   AND archived <> ?3",
                params![root_key, session_id, archived_value, updated_at],
            )
            .map_err(to_io_error)?;
        if changed == 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "thread {} was not found or is already in the requested state",
                    session_id
                ),
            ));
        }

        Ok(())
    }
}
mod hook;
mod scan {
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
}

pub(crate) use archive::mutate_thread_archive_state_at;
pub(crate) use hook::upsert_hook_session_at;
pub(crate) use scan::{next_scan_seq, upsert_thread_row};
