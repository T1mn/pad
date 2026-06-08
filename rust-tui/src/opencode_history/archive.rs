use super::query::db_path_for_session;
use super::util::{open_write, to_io_error};
use std::io;

pub(crate) fn set_archived(session_id: &str, archived: bool) -> io::Result<()> {
    let Some(db_path) = db_path_for_session(session_id)? else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("OpenCode session {session_id} was not found"),
        ));
    };
    let connection = open_write(&db_path)?;
    let value = if archived { Some(now_millis()) } else { None };
    let changed = connection
        .execute(
            "UPDATE session SET time_archived = ?2, time_updated = MAX(time_updated, ?3) WHERE id = ?1",
            rusqlite::params![session_id, value, now_millis()],
        )
        .map_err(to_io_error)?;
    if changed == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("OpenCode session {session_id} was not found"),
        ));
    }
    Ok(())
}

fn now_millis() -> i64 {
    crate::time::unix_now_millis() as i64
}
