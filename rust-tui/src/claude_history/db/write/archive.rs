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
