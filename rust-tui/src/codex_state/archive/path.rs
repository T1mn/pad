use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn target_rollout_path(
    source_path: &Path,
    codex_home: &Path,
    thread_id: &str,
    archive: bool,
) -> io::Result<PathBuf> {
    let file_name = rollout_file_name(source_path, thread_id)?;
    if archive {
        ensure_path_in_dir(source_path, &codex_home.join("sessions"), "sessions")?;
        return Ok(codex_home.join("archived_sessions").join(file_name));
    }

    ensure_path_in_dir(
        source_path,
        &codex_home.join("archived_sessions"),
        "archived directory",
    )?;
    let (year, month, day) = rollout_date_parts(file_name)?;
    Ok(codex_home
        .join("sessions")
        .join(year)
        .join(month)
        .join(day)
        .join(file_name))
}

fn rollout_file_name<'a>(source_path: &'a Path, thread_id: &str) -> io::Result<&'a str> {
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
    Ok(file_name)
}

fn ensure_path_in_dir(path: &Path, dir: &Path, label: &str) -> io::Result<()> {
    let normalized_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let normalized_dir = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());

    if !normalized_path.starts_with(&normalized_dir) {
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
