use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::pad_home_dir;

pub(super) const PAD_CODEX_PROFILE: &str = "pad";

/// PAD-private Codex home. Official Codex App continues to use `~/.codex`.
pub(crate) fn pad_codex_home_dir() -> PathBuf {
    pad_home_dir().join("codex-home")
}

pub(crate) fn canonical_codex_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
}

pub(crate) fn pad_codex_config_path() -> PathBuf {
    pad_codex_home_dir().join(format!("{PAD_CODEX_PROFILE}.config.toml"))
}

/// PAD relay auth lives outside `~/.codex` so Codex App auth stays untouched.
pub(crate) fn pad_codex_auth_path() -> PathBuf {
    pad_codex_home_dir().join("auth.json")
}

/// Hooks live in PAD's private Codex home; official Codex App hooks stay untouched.
pub(crate) fn pad_codex_hooks_path() -> PathBuf {
    pad_codex_home_dir().join("hooks.json")
}

pub(crate) fn ensure_pad_codex_home_layout() -> io::Result<()> {
    fs::create_dir_all(pad_codex_home_dir())?;
    unlink_legacy_shared_state_symlinks()?;
    seed_pad_profile_from_canonical()?;

    Ok(())
}

fn unlink_legacy_shared_state_symlinks() -> io::Result<()> {
    for name in [
        "sessions",
        "archived_sessions",
        "state_5.sqlite",
        "state_5.sqlite-shm",
        "state_5.sqlite-wal",
    ] {
        let path = pad_codex_home_dir().join(name);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.file_type().is_symlink() {
            continue;
        }
        remove_symlink(&path)?;
        crate::log_debug!(
            "codex_home: removed legacy shared Codex state symlink {}",
            path.display()
        );
    }
    Ok(())
}

fn remove_symlink(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(file_err) => match fs::remove_dir(path) {
            Ok(()) => Ok(()),
            Err(_) => Err(file_err),
        },
    }
}

fn seed_pad_profile_from_canonical() -> io::Result<()> {
    let target = pad_codex_config_path();
    if path_exists_or_symlink(&target) {
        return Ok(());
    }

    let source = canonical_codex_home_dir().join("config.toml");
    if source.exists() {
        fs::copy(source, target)?;
    }
    Ok(())
}

fn path_exists_or_symlink(path: &Path) -> bool {
    path.exists() || fs::symlink_metadata(path).is_ok()
}
