use super::paths::{codex_auth_backup_path, codex_auth_path, codex_backup_path, codex_config_path};
use std::path::Path;

pub(in crate::relay) fn has_backup(path: &Path) -> bool {
    path.exists()
}

pub(in crate::relay) fn backup_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

pub(in crate::relay) fn restore_file(path: &Path, backup_path: &Path) {
    let Ok(content) = std::fs::read_to_string(backup_path) else {
        return;
    };
    write_text_file(path, &content);
}

pub(in crate::relay) fn write_text_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, content);
}

pub(in crate::relay) fn has_pad_codex_backup() -> bool {
    codex_backup_path().exists()
}

pub(in crate::relay) fn has_pad_codex_auth_backup() -> bool {
    codex_auth_backup_path().exists()
}

pub(in crate::relay) fn backup_codex_config(content: &str) -> std::io::Result<()> {
    let backup = codex_backup_path();
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(backup, content)
}

pub(in crate::relay) fn backup_codex_auth(content: &str) -> std::io::Result<()> {
    let backup = codex_auth_backup_path();
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(backup, content)
}

pub(in crate::relay) fn restore_codex_config() {
    let path = codex_config_path();
    let backup = codex_backup_path();
    let Ok(content) = std::fs::read_to_string(&backup) else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, content);
}

pub(in crate::relay) fn restore_codex_auth() {
    let path = codex_auth_path();
    let backup = codex_auth_backup_path();
    let Ok(content) = std::fs::read_to_string(&backup) else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, content);
}
