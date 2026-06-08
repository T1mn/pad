use super::text::normalize_text;
use crate::thread_meta::db::{ensure_schema_at, open_db, to_io_error};
use rusqlite::params;
use std::io;
use std::path::Path;

pub(in crate::thread_meta) fn upsert_thread_meta_at(
    db_path: &Path,
    agent_type: &str,
    thread_id: &str,
    title_override: Option<&str>,
    note: Option<&str>,
    pinned: bool,
) -> io::Result<()> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let now = crate::app::unix_now_ts();
    connection
        .execute(
            "INSERT INTO thread_meta (
                agent_type, thread_id, title_override, note, pinned, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(agent_type, thread_id) DO UPDATE SET
                title_override = excluded.title_override,
                note = excluded.note,
                pinned = excluded.pinned,
                updated_at = excluded.updated_at",
            params![
                agent_type,
                thread_id,
                normalize_text(title_override),
                normalize_text(note),
                if pinned { 1_i64 } else { 0_i64 },
                now,
            ],
        )
        .map_err(to_io_error)?;
    Ok(())
}
