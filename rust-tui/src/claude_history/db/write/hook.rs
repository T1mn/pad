use super::super::super::util::{normalize_path, to_io_error};
use super::super::schema::{ensure_schema, open_index_db};
use rusqlite::{params, OptionalExtension};
use std::io;
use std::path::Path;

pub(crate) fn upsert_hook_session_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
    title: Option<&str>,
    updated_at: i64,
) -> io::Result<()> {
    let started_at = std::time::Instant::now();
    let root_key = normalize_path(root).to_string_lossy().to_string();
    let transcript_key = transcript_path.to_string_lossy().to_string();
    let cwd_key = normalize_path(cwd).to_string_lossy().to_string();

    let mut connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let tx = connection.transaction().map_err(to_io_error)?;

    let existing = tx
        .query_row(
            "SELECT transcript_path
             FROM claude_threads
             WHERE root = ?1 AND session_id = ?2
             ORDER BY updated_at DESC, transcript_path DESC
             LIMIT 1",
            params![root_key, session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_io_error)?;

    if let Some(existing_transcript) = existing {
        tx.execute(
            "UPDATE claude_threads
             SET transcript_path = ?3,
                 cwd = ?4,
                 title = COALESCE(?5, title),
                 updated_at = MAX(updated_at, ?6),
                 last_assistant_at = MAX(last_assistant_at, ?6),
                 file_mtime = MAX(file_mtime, ?6),
                 last_seen_at = ?6
             WHERE root = ?1 AND transcript_path = ?2",
            params![
                root_key,
                existing_transcript,
                transcript_key,
                cwd_key,
                title,
                updated_at,
            ],
        )
        .map_err(to_io_error)?;
    } else {
        tx.execute(
            "INSERT INTO claude_threads (
                root, transcript_path, session_id, cwd, title,
                updated_at, last_assistant_at, file_mtime, last_seen_seq,
                last_seen_at, is_sidechain
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?6, 0, ?6, 0)",
            params![
                root_key,
                transcript_key,
                session_id,
                cwd_key,
                title,
                updated_at
            ],
        )
        .map_err(to_io_error)?;
    }

    tx.commit().map_err(to_io_error)?;
    if started_at.elapsed().as_millis() >= 8 {
        crate::log_debug!(
            "claude_history.hook_upsert: session_id={} elapsed_ms={} transcript={}",
            session_id,
            started_at.elapsed().as_millis(),
            transcript_path.display()
        );
    }
    Ok(())
}
