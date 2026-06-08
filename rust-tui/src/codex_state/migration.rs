mod paths;
mod sqlite;

use super::cache::invalidate_thread_cache;
use super::query::default_db_path;
use super::util::to_io_error;
use paths::rollout_path_replacements;
use sqlite::{has_rollout_paths_to_normalize, normalize_rollout_prefix, open_migration_db};
use std::io;
use std::path::Path;

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

    let connection = open_migration_db(db_path)?;
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
