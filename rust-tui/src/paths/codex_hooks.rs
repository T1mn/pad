use std::io;

mod feature;
mod hooks_json;
mod toml_edit;
mod version;

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
