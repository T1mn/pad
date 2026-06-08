use super::super::util::to_io_error;
use rusqlite::{Connection, OpenFlags};
use std::io;
use std::path::Path;

pub(super) fn open_migration_db(db_path: &Path) -> io::Result<Connection> {
    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;
    connection
        .pragma_update(None, "busy_timeout", 5000_i64)
        .map_err(to_io_error)?;
    Ok(connection)
}

pub(super) fn has_rollout_paths_to_normalize(
    connection: &Connection,
    replacements: &[(String, String)],
) -> io::Result<bool> {
    for (from_prefix, _) in replacements {
        let from_with_sep = format!("{from_prefix}/");
        let found: i64 = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM threads
                    WHERE rollout_path = ?1
                       OR rollout_path LIKE ?2
                    LIMIT 1
                )",
                (from_prefix, format!("{from_with_sep}%")),
                |row| row.get(0),
            )
            .map_err(to_io_error)?;
        if found != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn normalize_rollout_prefix(
    connection: &Connection,
    from_prefix: &str,
    to_prefix: &str,
) -> io::Result<usize> {
    let from_with_sep = format!("{from_prefix}/");
    connection
        .execute(
            "UPDATE threads
             SET rollout_path = ?1 || substr(rollout_path, ?2)
             WHERE rollout_path = ?3
                OR rollout_path LIKE ?4",
            (
                to_prefix,
                from_with_sep.len() as i64,
                from_prefix,
                format!("{from_with_sep}%"),
            ),
        )
        .map_err(to_io_error)
}
