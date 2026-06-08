use crate::thread_meta::db::{ensure_schema_at, open_db, to_io_error};
use rusqlite::params;
use std::io;
use std::path::Path;

pub(in crate::thread_meta) fn set_thread_deleted_at(
    db_path: &Path,
    agent_type: &str,
    thread_id: &str,
    deleted: bool,
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
                deleted,
                deleted_at,
                note,
                pinned,
                updated_at
             ) VALUES (?1, ?2, NULL, NULL, NULL, NULL, ?3, ?4, NULL, 0, ?4)
             ON CONFLICT(agent_type, thread_id) DO UPDATE SET
                deleted = excluded.deleted,
                deleted_at = CASE WHEN excluded.deleted = 1 THEN excluded.deleted_at ELSE NULL END,
                updated_at = excluded.updated_at",
            params![
                agent_type,
                thread_id,
                if deleted { 1_i64 } else { 0_i64 },
                now,
            ],
        )
        .map_err(to_io_error)?;
    Ok(())
}
