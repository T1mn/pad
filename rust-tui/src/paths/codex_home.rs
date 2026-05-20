use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::pad_home_dir;

const SHARED_SESSION_DIRS: &[&str] = &["sessions", "archived_sessions"];
const SHARED_STATE_FILES: &[&str] = &["state_5.sqlite", "state_5.sqlite-shm", "state_5.sqlite-wal"];

pub(super) fn pad_codex_home_dir() -> PathBuf {
    pad_home_dir().join("codex-home")
}

pub(super) fn canonical_codex_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
}

pub(super) fn pad_codex_config_path() -> PathBuf {
    pad_codex_home_dir().join("config.toml")
}

pub(super) fn pad_codex_auth_path() -> PathBuf {
    pad_codex_home_dir().join("auth.json")
}

pub(super) fn pad_codex_hooks_path() -> PathBuf {
    pad_codex_home_dir().join("hooks.json")
}

pub(super) fn ensure_pad_codex_home_layout() -> io::Result<()> {
    let pad_home = pad_codex_home_dir();
    let canonical_home = canonical_codex_home_dir();

    fs::create_dir_all(&pad_home)?;
    fs::create_dir_all(&canonical_home)?;
    seed_config_from_canonical(&pad_home, &canonical_home)?;

    for name in SHARED_SESSION_DIRS {
        ensure_shared_dir(&pad_home.join(name), &canonical_home.join(name))?;
    }
    for name in SHARED_STATE_FILES {
        ensure_shared_file_link(&pad_home.join(name), &canonical_home.join(name))?;
    }

    Ok(())
}

fn seed_config_from_canonical(pad_home: &Path, canonical_home: &Path) -> io::Result<()> {
    let target = pad_home.join("config.toml");
    if path_exists_or_symlink(&target) {
        return Ok(());
    }

    let source = canonical_home.join("config.toml");
    if source.exists() {
        fs::copy(source, target)?;
    }
    Ok(())
}

fn ensure_shared_dir(link_path: &Path, target_path: &Path) -> io::Result<()> {
    if path_exists_or_symlink(link_path) {
        return Ok(());
    }

    fs::create_dir_all(target_path)?;
    create_symlink_or_dir(target_path, link_path)
}

fn ensure_shared_file_link(link_path: &Path, target_path: &Path) -> io::Result<()> {
    if path_exists_or_symlink(link_path) {
        return Ok(());
    }
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    create_symlink_or_copy(target_path, link_path)
}

fn path_exists_or_symlink(path: &Path) -> bool {
    path.exists() || fs::symlink_metadata(path).is_ok()
}

#[cfg(unix)]
fn create_symlink_or_dir(target_path: &Path, link_path: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target_path, link_path)
}

#[cfg(not(unix))]
fn create_symlink_or_dir(_target_path: &Path, link_path: &Path) -> io::Result<()> {
    fs::create_dir_all(link_path)
}

#[cfg(unix)]
fn create_symlink_or_copy(target_path: &Path, link_path: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target_path, link_path)
}

#[cfg(not(unix))]
fn create_symlink_or_copy(target_path: &Path, link_path: &Path) -> io::Result<()> {
    if target_path.exists() {
        fs::copy(target_path, link_path)?;
    }
    Ok(())
}
