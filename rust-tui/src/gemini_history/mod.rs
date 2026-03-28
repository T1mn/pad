mod model;
mod scan;
mod storage;
mod tests;
mod util;

use std::io;
use std::path::Path;

pub use model::GeminiThreadRef;

pub fn all_threads() -> io::Result<Vec<GeminiThreadRef>> {
    let root = util::default_gemini_tmp_dir()?;
    let db_path = util::default_index_db_path()?;
    all_threads_at(&root, &db_path)
}

pub fn all_archived_threads() -> io::Result<Vec<GeminiThreadRef>> {
    let root = util::default_gemini_tmp_dir()?;
    let db_path = util::default_index_db_path()?;
    all_archived_threads_at(&root, &db_path)
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<GeminiThreadRef>> {
    let root = util::default_gemini_tmp_dir()?;
    let db_path = util::default_index_db_path()?;
    thread_for_id_at(&root, &db_path, session_id)
}

pub fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<GeminiThreadRef>> {
    let root = util::default_gemini_tmp_dir()?;
    let db_path = util::default_index_db_path()?;
    threads_for_cwd_at(&root, &db_path, cwd)
}

pub fn archive_thread(session_id: &str) -> io::Result<()> {
    let root = util::default_gemini_tmp_dir()?;
    let db_path = util::default_index_db_path()?;
    archive_thread_at(&root, &db_path, session_id, true)
}

pub fn unarchive_thread(session_id: &str) -> io::Result<()> {
    let root = util::default_gemini_tmp_dir()?;
    let db_path = util::default_index_db_path()?;
    archive_thread_at(&root, &db_path, session_id, false)
}

pub(crate) fn all_threads_at(root: &Path, db_path: &Path) -> io::Result<Vec<GeminiThreadRef>> {
    sync_index_at(root, db_path)?;
    storage::query_threads(db_path, Some(false))
}

pub(crate) fn all_archived_threads_at(
    root: &Path,
    db_path: &Path,
) -> io::Result<Vec<GeminiThreadRef>> {
    sync_index_at(root, db_path)?;
    storage::query_threads(db_path, Some(true))
}

pub(crate) fn thread_for_id_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
) -> io::Result<Option<GeminiThreadRef>> {
    sync_index_at(root, db_path)?;
    storage::query_thread_for_id(db_path, session_id)
}

pub(crate) fn threads_for_cwd_at(
    root: &Path,
    db_path: &Path,
    cwd: &Path,
) -> io::Result<Vec<GeminiThreadRef>> {
    sync_index_at(root, db_path)?;
    storage::query_threads_for_cwd(db_path, cwd, Some(false))
}

pub(crate) fn archive_thread_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
    archive: bool,
) -> io::Result<()> {
    sync_index_at(root, db_path)?;
    storage::set_threads_archived(db_path, session_id, archive)
}

pub(crate) fn sync_index_at(root: &Path, db_path: &Path) -> io::Result<()> {
    let records = scan::collect_records(root)?;
    storage::replace_records(db_path, &records)
}
