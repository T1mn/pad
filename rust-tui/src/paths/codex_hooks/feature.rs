use super::toml_edit::{remove_toml_key_in_section, set_toml_bool_in_section};
use super::version::{codex_hooks_feature_key_for_version, detect_codex_cli_version};
use crate::paths::pad_codex_config_path;
use std::fs;
use std::io;

pub(super) fn ensure_codex_feature_enabled() -> io::Result<()> {
    let path = pad_codex_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = fs::read_to_string(&path).unwrap_or_default();
    let key = codex_hooks_feature_key_for_version(detect_codex_cli_version().as_deref());
    let mut updated = set_toml_bool_in_section(&existing, "features", key, true);
    if key == "hooks" {
        updated = remove_toml_key_in_section(&updated, "features", "codex_hooks");
    }

    if updated != existing {
        fs::write(path, updated)?;
    }

    Ok(())
}
