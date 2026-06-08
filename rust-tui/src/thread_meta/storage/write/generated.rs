use super::text::normalize_text;
use crate::thread_meta::db::{ensure_schema_at, open_db, to_io_error};
use rusqlite::params;
use std::io;
use std::path::Path;

pub(in crate::thread_meta) fn upsert_generated_title_at(
    db_path: &Path,
    agent_type: &str,
    thread_id: &str,
    generated_title: &str,
    generated_turn_count: usize,
) -> io::Result<()> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let now = crate::app::unix_now_ts();
    connection
        .execute(
            "INSERT INTO thread_meta (
                agent_type,
                thread_id,
                title_override,
                generated_title,
                generated_turn_count,
                generated_updated_at,
                note,
                pinned,
                updated_at
             ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL, 0, ?5)
             ON CONFLICT(agent_type, thread_id) DO UPDATE SET
                generated_title = excluded.generated_title,
                generated_turn_count = excluded.generated_turn_count,
                generated_updated_at = excluded.generated_updated_at,
                updated_at = excluded.updated_at",
            params![
                agent_type,
                thread_id,
                normalize_text(Some(generated_title)),
                generated_turn_count as i64,
                now,
            ],
        )
        .map_err(to_io_error)?;
    Ok(())
}
