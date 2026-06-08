use std::io;

pub(crate) use crate::time::unix_now_ts;

pub(crate) fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}
