use super::cache::invalidate_thread_cache;
use super::query::default_db_path;
use super::util::to_io_error;
use rusqlite::{Connection, OpenFlags};
use std::io;
use std::path::{Path, PathBuf};

const SHARED_ROLLOUT_DIRS: &[&str] = &["sessions", "archived_sessions"];

pub fn normalize_pad_codex_home_rollout_paths() -> io::Result<usize> {
    normalize_pad_codex_home_rollout_paths_at(
        &default_db_path()?,
        &crate::paths::pad_codex_home_dir(),
        &crate::paths::canonical_codex_home_dir(),
    )
}

pub(crate) fn normalize_pad_codex_home_rollout_paths_at(
    db_path: &Path,
    pad_codex_home: &Path,
    canonical_codex_home: &Path,
) -> io::Result<usize> {
    if !db_path.exists() {
        return Ok(0);
    }

    let replacements = rollout_path_replacements(pad_codex_home, canonical_codex_home);
    if replacements.is_empty() {
        return Ok(0);
    }

    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;
    connection
        .pragma_update(None, "busy_timeout", 5000_i64)
        .map_err(to_io_error)?;
    if !has_rollout_paths_to_normalize(&connection, &replacements)? {
        return Ok(0);
    }
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(to_io_error)?;

    let result = (|| {
        let mut updated = 0usize;
        for (from_prefix, to_prefix) in &replacements {
            updated += normalize_rollout_prefix(&connection, from_prefix, to_prefix)?;
        }
        Ok(updated)
    })();

    match result {
        Ok(updated) => {
            connection.execute_batch("COMMIT").map_err(to_io_error)?;
            if updated > 0 {
                invalidate_thread_cache(db_path);
            }
            Ok(updated)
        }
        Err(err) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

fn rollout_path_replacements(
    pad_codex_home: &Path,
    canonical_codex_home: &Path,
) -> Vec<(String, String)> {
    let mut replacements = Vec::new();
    for dir in SHARED_ROLLOUT_DIRS {
        let from = path_string(pad_codex_home.join(dir));
        let to = path_string(canonical_codex_home.join(dir));
        if from != to {
            replacements.push((from, to));
        }
    }
    replacements
}

fn has_rollout_paths_to_normalize(
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

fn normalize_rollout_prefix(
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

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}
