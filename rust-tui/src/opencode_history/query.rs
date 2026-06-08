mod db;
mod messages;
mod model_parse;
mod thread;

use super::model::OpenCodeThreadRef;
use super::util::default_db_paths;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) use db::query_threads_at;

pub(crate) fn all_threads(archived: Option<bool>) -> io::Result<Vec<OpenCodeThreadRef>> {
    let mut threads = Vec::new();
    for db_path in default_db_paths().into_iter().filter(|path| path.exists()) {
        threads.extend(query_threads_at(&db_path, archived)?);
    }
    threads.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    Ok(threads)
}

pub(crate) fn thread_for_id(session_id: &str) -> io::Result<Option<OpenCodeThreadRef>> {
    for db_path in default_db_paths().into_iter().filter(|path| path.exists()) {
        if let Some(thread) = db::query_thread_for_id_at(&db_path, session_id)? {
            return Ok(Some(thread));
        }
    }
    Ok(None)
}

pub(crate) fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<OpenCodeThreadRef>> {
    let cwd = normalize_path(cwd).to_string_lossy().to_string();
    Ok(all_threads(Some(false))?
        .into_iter()
        .filter(|thread| normalize_path(&thread.cwd).to_string_lossy() == cwd)
        .collect())
}

pub(crate) fn db_path_for_session(session_id: &str) -> io::Result<Option<PathBuf>> {
    Ok(thread_for_id(session_id)?.map(|thread| thread.db_path))
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod query_tests;
