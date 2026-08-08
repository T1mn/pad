use std::io;

mod feature {
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
}
mod hooks_json;
mod toml_edit;
mod version {
    use std::process::Command;

    const NEW_HOOKS_FEATURE_VERSION: (u64, u64, u64) = (0, 130, 0);

    pub(crate) fn codex_hooks_feature_key_for_version(version: Option<&str>) -> &'static str {
        match version.and_then(parse_codex_cli_version) {
            Some(version) if version >= NEW_HOOKS_FEATURE_VERSION => "hooks",
            _ => "codex_hooks",
        }
    }

    pub(super) fn detect_codex_cli_version() -> Option<String> {
        Command::new("codex")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|raw| {
                raw.split_whitespace()
                    .rev()
                    .find(|token| parse_codex_cli_version(token).is_some())
                    .map(str::to_string)
            })
    }

    pub(crate) fn parse_codex_cli_version(raw: &str) -> Option<(u64, u64, u64)> {
        let clean = raw.trim().trim_start_matches('v');
        let mut parts = clean.split(['.', '-']);
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    }
}

use feature::ensure_codex_feature_enabled;
use hooks_json::ensure_codex_hooks_json;

#[cfg(test)]
pub(super) use toml_edit::{
    remove_toml_key_in_section as test_remove_toml_key_in_section,
    set_toml_bool_in_section as test_set_toml_bool_in_section,
};
#[cfg(test)]
pub(super) use version::{
    codex_hooks_feature_key_for_version as test_codex_hooks_feature_key_for_version,
    parse_codex_cli_version as test_parse_codex_cli_version,
};

pub(super) fn ensure_codex_hook_support() -> io::Result<()> {
    ensure_codex_feature_enabled()?;
    ensure_codex_hooks_json()?;
    Ok(())
}
