use super::paths::{codex_auth_backup_path, codex_auth_path, codex_backup_path, codex_config_path};
use crate::atomic_file::{ensure_private, write_private};
use std::path::Path;

pub(in crate::relay) fn preserve_backup(path: &Path, content: &str) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => ensure_private(path),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "backup path is not a regular file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => write_private(path, content),
        Err(error) => Err(error),
    }
}

pub(in crate::relay) fn restore_file(path: &Path, backup_path: &Path) -> std::io::Result<()> {
    let content = match std::fs::read_to_string(backup_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    write_text_file(path, &content)
}

pub(in crate::relay) fn write_text_file(path: &Path, content: &str) -> std::io::Result<()> {
    write_private(path, content)
}

pub(in crate::relay) fn restore_codex_config() -> std::io::Result<()> {
    let path = codex_config_path();
    let backup = codex_backup_path();
    let content = match std::fs::read_to_string(&backup) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    write_private(&path, content)
}

pub(in crate::relay) fn restore_codex_auth() -> std::io::Result<()> {
    let path = codex_auth_path();
    let backup = codex_auth_backup_path();
    let content = match std::fs::read_to_string(&backup) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    write_private(&path, content)
}

pub(in crate::relay) fn log_file_error(operation: &str, path: &Path, error: &std::io::Error) {
    log_debug!("relay: {} {} failed: {}", operation, path.display(), error);
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
