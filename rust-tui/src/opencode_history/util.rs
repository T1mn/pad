use rusqlite::OpenFlags;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn default_db_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = std::env::var_os("OPENCODE_DB") {
        paths.push(PathBuf::from(path));
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        push_unique(
            &mut paths,
            PathBuf::from(data_home)
                .join("opencode")
                .join("opencode.db"),
        );
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        push_unique(
            &mut paths,
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("opencode.db"),
        );
        push_unique(
            &mut paths,
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("opencode-local.db"),
        );
    }
    paths
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

pub(crate) fn open_readonly(path: &Path) -> io::Result<rusqlite::Connection> {
    rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)
}

pub(crate) fn open_write(path: &Path) -> io::Result<rusqlite::Connection> {
    rusqlite::Connection::open(path).map_err(to_io_error)
}

pub(crate) fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err.to_string())
}
