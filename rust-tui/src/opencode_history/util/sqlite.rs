use rusqlite::OpenFlags;
use std::io;
use std::path::Path;

pub(crate) fn open_readonly(path: &Path) -> io::Result<rusqlite::Connection> {
    rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)
}

pub(crate) fn open_write(path: &Path) -> io::Result<rusqlite::Connection> {
    let connection = rusqlite::Connection::open(path).map_err(to_io_error)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(to_io_error)?;
    Ok(connection)
}

pub(crate) fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err.to_string())
}
