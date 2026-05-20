use super::cache::invalidate_thread_cache;
use super::model::ThreadRow;
use super::query::default_db_path;
use super::util::{to_io_error, unix_now_ts};
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn archive_thread(thread_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(thread_id, true)
}

pub fn unarchive_thread(thread_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(thread_id, false)
}

fn mutate_thread_archive_state(thread_id: &str, archive: bool) -> io::Result<()> {
    let db_path = default_db_path()?;
    let codex_home = codex_home_dir()?;
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
    if archive && thread.archived {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("thread {} is already archived", thread_id),
        ));
    }
    if !archive && !thread.archived {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("thread {} is not archived", thread_id),
        ));
    }

    let source_path = PathBuf::from(&thread.rollout_path);
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "rollout path missing file name")
        })?;
    if !file_name.contains(thread_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "rollout path `{}` does not match thread id {}",
                source_path.display(),
                thread_id
            ),
        ));
    }

    let target_path = if archive {
        ensure_path_in_dir(&source_path, &codex_home.join("sessions"), "sessions")?;
        codex_home.join("archived_sessions").join(file_name)
    } else {
        ensure_path_in_dir(
            &source_path,
            &codex_home.join("archived_sessions"),
            "archived directory",
        )?;
        let (year, month, day) = rollout_date_parts(file_name)?;
        codex_home
            .join("sessions")
            .join(year)
            .join(month)
            .join(day)
            .join(file_name)
    };

    let target_parent = target_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "target rollout path missing parent",
        )
    })?;
    fs::create_dir_all(target_parent)?;
    if target_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("target rollout already exists: {}", target_path.display()),
        ));
    }
    if !source_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("rollout file not found: {}", source_path.display()),
        ));
    }

    fs::rename(&source_path, &target_path)?;

    let update_result = if archive {
        let archived_at = unix_now_ts();
        connection.execute(
            "UPDATE threads
             SET archived = 1, archived_at = ?1, rollout_path = ?2
             WHERE id = ?3 AND archived = 0",
            (
                archived_at,
                target_path.to_string_lossy().to_string(),
                thread_id.to_string(),
            ),
        )
    } else {
        let updated_at = unix_now_ts();
        connection.execute(
            "UPDATE threads
             SET archived = 0, archived_at = NULL, rollout_path = ?1, updated_at = ?2
             WHERE id = ?3 AND archived = 1",
            (
                target_path.to_string_lossy().to_string(),
                updated_at,
                thread_id.to_string(),
            ),
        )
    };

    match update_result.map_err(to_io_error) {
        Ok(1) => {
            invalidate_thread_cache(db_path);
            Ok(())
        }
        Ok(_) => {
            let _ = fs::rename(&target_path, &source_path);
            Err(io::Error::other(format!(
                "failed to update thread {} archive state",
                thread_id
            )))
        }
        Err(err) => {
            let _ = fs::rename(&target_path, &source_path);
            Err(err)
        }
    }
}

fn read_thread_for_update(connection: &Connection, thread_id: &str) -> io::Result<ThreadRow> {
    connection
        .query_row(
            "SELECT rollout_path, archived FROM threads WHERE id = ?1",
            [thread_id],
            |row| {
                Ok(ThreadRow {
                    rollout_path: row.get::<_, String>(0)?,
                    archived: row.get::<_, i64>(1)? != 0,
                })
            },
        )
        .map_err(to_io_error)
}

fn ensure_path_in_dir(path: &Path, dir: &Path, label: &str) -> io::Result<()> {
    if !path.starts_with(dir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("rollout path `{}` must be in {}", path.display(), label),
        ));
    }
    Ok(())
}

fn rollout_date_parts(file_name: &str) -> io::Result<(&str, &str, &str)> {
    let stem = file_name.strip_prefix("rollout-").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "rollout path missing filename timestamp",
        )
    })?;
    if stem.len() < 10 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "rollout path missing filename timestamp",
        ));
    }

    let year = &stem[0..4];
    let month = &stem[5..7];
    let day = &stem[8..10];
    if stem.as_bytes().get(4) != Some(&b'-') || stem.as_bytes().get(7) != Some(&b'-') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "rollout path missing filename timestamp",
        ));
    }
    Ok((year, month, day))
}

fn codex_home_dir() -> io::Result<PathBuf> {
    Ok(crate::paths::canonical_codex_home_dir())
}
