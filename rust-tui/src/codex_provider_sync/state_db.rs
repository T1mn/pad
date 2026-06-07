use rusqlite::{Connection, OpenFlags};
use std::io;
use std::path::Path;

pub(super) const STATE_DB_BASENAME: &str = "state_5.sqlite";

pub(super) fn update_sqlite_provider(
    sqlite_path: &Path,
    target_provider: &str,
) -> io::Result<usize> {
    if !sqlite_path.exists() {
        return Ok(0);
    }

    let connection = Connection::open_with_flags(
        sqlite_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;
    connection
        .pragma_update(None, "busy_timeout", 5000_i64)
        .map_err(to_io_error)?;
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(to_io_error)?;

    let result = connection.execute(
        "UPDATE threads
         SET model_provider = ?1
         WHERE COALESCE(model_provider, '') <> ?1",
        [target_provider],
    );

    match result {
        Ok(updated) => {
            connection.execute_batch("COMMIT").map_err(to_io_error)?;
            Ok(updated)
        }
        Err(err) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(to_io_error(err))
        }
    }
}

fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}
