use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(crate) fn file_mtime_secs(path: &Path) -> io::Result<i64> {
    fs::metadata(path)?
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
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
