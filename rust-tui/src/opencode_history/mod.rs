mod archive;
mod model;
mod query;
mod stats;
mod util;

use std::io;
use std::path::Path;

pub use model::OpenCodeThreadRef;

pub fn all_threads() -> io::Result<Vec<OpenCodeThreadRef>> {
    query::all_threads(Some(false))
}

pub fn all_archived_threads() -> io::Result<Vec<OpenCodeThreadRef>> {
    query::all_threads(Some(true))
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<OpenCodeThreadRef>> {
    query::thread_for_id(session_id)
}

pub fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<OpenCodeThreadRef>> {
    query::threads_for_cwd(cwd)
}

pub fn archive_thread(session_id: &str) -> io::Result<()> {
    archive::set_archived(session_id, true)
}

pub fn unarchive_thread(session_id: &str) -> io::Result<()> {
    archive::set_archived(session_id, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_thread_archive_returns_not_found() {
        let err = archive_thread("missing-session").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
