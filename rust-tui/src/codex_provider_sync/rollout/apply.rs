use super::line::split_first_line;
use super::RolloutChange;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub(in crate::codex_provider_sync) fn apply_rollout_changes(
    changes: &[RolloutChange],
) -> io::Result<usize> {
    let mut updated = 0usize;
    for change in changes {
        apply_rollout_change(change)?;
        updated += 1;
    }
    Ok(updated)
}

fn apply_rollout_change(change: &RolloutChange) -> io::Result<()> {
    let current = fs::read_to_string(&change.path)?;
    let (first_line, separator, rest) = split_first_line(&current);

    if first_line != change.original_first_line || separator != change.original_separator {
        return Err(io::Error::other(format!(
            "rollout file changed during provider sync: {}",
            change.path.display()
        )));
    }

    let updated_content = format!(
        "{}{}{}",
        change.updated_first_line, change.original_separator, rest
    );
    let permissions = fs::metadata(&change.path)?.permissions();
    write_file_atomically(&change.path, updated_content.as_bytes(), permissions)
}

fn write_file_atomically(
    path: &Path,
    content: &[u8],
    permissions: fs::Permissions,
) -> io::Result<()> {
    let (temp_path, mut temp_file) = create_temp_file(path, &permissions)?;
    let write_result = write_content(&mut temp_file, content, &permissions);
    drop(temp_file);

    if let Err(err) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }
    if let Err(err) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }
    Ok(())
}

fn create_temp_file(path: &Path, permissions: &fs::Permissions) -> io::Result<(PathBuf, File)> {
    loop {
        let temp_path = temp_path(path)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            options.mode(permissions.mode());
        }

        match options.open(&temp_path) {
            Ok(file) => return Ok((temp_path, file)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                let _ = fs::remove_file(&temp_path);
                return Err(err);
            }
        }
    }
}

fn temp_path(path: &Path) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "rollout has no parent"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "rollout has no file name"))?;
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".pad-sync-{}-{id}", std::process::id()));
    Ok(parent.join(temp_name))
}

fn write_content(file: &mut File, content: &[u8], permissions: &fs::Permissions) -> io::Result<()> {
    file.set_permissions(permissions.clone())?;
    file.write_all(content)?;
    file.flush()?;
    file.set_permissions(permissions.clone())
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod tests;
