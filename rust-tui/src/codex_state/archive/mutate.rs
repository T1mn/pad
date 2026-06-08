use super::super::cache::invalidate_thread_cache;
use super::super::query::default_db_path;
use super::super::util::to_io_error;
use super::db::{read_thread_for_update, update_archived_thread, update_unarchived_thread};
use super::path::target_rollout_path;
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn mutate_thread_archive_state(thread_id: &str, archive: bool) -> io::Result<()> {
    let db_path = default_db_path()?;
    let codex_home = codex_home_dir();
    mutate_thread_archive_state_at(&db_path, &codex_home, thread_id, archive)
}

pub(crate) fn mutate_thread_archive_state_at(
    db_path: &Path,
    codex_home: &Path,
    thread_id: &str,
    archive: bool,
) -> io::Result<()> {
    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    let thread = read_thread_for_update(&connection, thread_id)?;
    validate_archive_transition(thread_id, thread.archived, archive)?;

    let source_path = PathBuf::from(&thread.rollout_path);
    if !source_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("rollout file not found: {}", source_path.display()),
        ));
    }

    let target_path = target_rollout_path(&source_path, codex_home, thread_id, archive)?;
    create_target_parent(&target_path)?;
    if target_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("target rollout already exists: {}", target_path.display()),
        ));
    }

    fs::rename(&source_path, &target_path)?;
    update_archive_row(
        &connection,
        db_path,
        thread_id,
        archive,
        &source_path,
        &target_path,
    )
}

fn validate_archive_transition(thread_id: &str, archived: bool, archive: bool) -> io::Result<()> {
    if archive && archived {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("thread {} is already archived", thread_id),
        ));
    }
    if !archive && !archived {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("thread {} is not archived", thread_id),
        ));
    }
    Ok(())
}

fn create_target_parent(target_path: &Path) -> io::Result<()> {
    let target_parent = target_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "target rollout path missing parent",
        )
    })?;
    fs::create_dir_all(target_parent)
}

fn update_archive_row(
    connection: &Connection,
    db_path: &Path,
    thread_id: &str,
    archive: bool,
    source_path: &Path,
    target_path: &Path,
) -> io::Result<()> {
    let update_result = if archive {
        update_archived_thread(connection, thread_id, target_path)
    } else {
        update_unarchived_thread(connection, thread_id, target_path)
    };

    match update_result.map_err(to_io_error) {
        Ok(1) => {
            invalidate_thread_cache(db_path);
            Ok(())
        }
        Ok(_) => rollback_rollout_move(thread_id, source_path, target_path),
        Err(err) => {
            let _ = fs::rename(target_path, source_path);
            Err(err)
        }
    }
}

fn rollback_rollout_move(
    thread_id: &str,
    source_path: &Path,
    target_path: &Path,
) -> io::Result<()> {
    let _ = fs::rename(target_path, source_path);
    Err(io::Error::other(format!(
        "failed to update thread {} archive state",
        thread_id
    )))
}

fn codex_home_dir() -> PathBuf {
    crate::paths::canonical_codex_home_dir()
}
