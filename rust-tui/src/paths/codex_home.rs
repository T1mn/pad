use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::pad_home_dir;

pub(super) const PAD_CODEX_PROFILE: &str = "pad";

/// Legacy PAD Codex home, kept for PAD-private auth and old rollout migration.
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
    canonical_codex_home_dir().join(format!("{PAD_CODEX_PROFILE}.config.toml"))
}

/// PAD relay auth is injected as process env so Codex app auth stays untouched.
pub(super) fn pad_codex_auth_path() -> PathBuf {
    pad_codex_home_dir().join("auth.json")
}

/// Hooks live in the canonical home; the bridge no-ops unless PAD sets its guard env.
pub(super) fn pad_codex_hooks_path() -> PathBuf {
    canonical_codex_home_dir().join("hooks.json")
}

pub(super) fn ensure_pad_codex_home_layout() -> io::Result<()> {
    let canonical_home = canonical_codex_home_dir();

    fs::create_dir_all(pad_codex_home_dir())?;
    fs::create_dir_all(&canonical_home)?;
    seed_pad_profile_from_canonical(&canonical_home)?;
    if let Err(err) = crate::codex_state::normalize_pad_codex_home_rollout_paths() {
        crate::log_debug!("codex_home: rollout path normalization skipped: {}", err);
    }

    Ok(())
}

fn seed_pad_profile_from_canonical(canonical_home: &Path) -> io::Result<()> {
    let target = pad_codex_config_path();
    if path_exists_or_symlink(&target) {
        return Ok(());
    }

    let source = canonical_home.join("config.toml");
    if source.exists() {
        fs::copy(source, target)?;
    }
    Ok(())
}

fn path_exists_or_symlink(path: &Path) -> bool {
    path.exists() || fs::symlink_metadata(path).is_ok()
}
