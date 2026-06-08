use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn file_mtime_secs(path: &Path) -> io::Result<i64> {
    fs::metadata(path)?
        .modified()
        .ok()
        .and_then(crate::time::system_time_unix_secs)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "failed to read file mtime"))
}

pub(crate) fn now_ts() -> i64 {
    crate::time::unix_now_ts()
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}
