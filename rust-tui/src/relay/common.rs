mod files;
mod formats;
mod paths;

use crate::theme::AgentConfig;

pub(super) use files::{
    backup_codex_auth, backup_codex_config, backup_file, has_backup, has_pad_codex_auth_backup,
    has_pad_codex_backup, restore_codex_auth, restore_codex_config, restore_file, write_text_file,
};
pub(super) use formats::{
    parse_env_file, parse_json_object, parse_toml_document, read_json_value, serialize_env_file,
    serialize_json_pretty, serialize_toml_document, write_json_value,
};
pub(super) use paths::{
    claude_backup_path, claude_permission_state_path, claude_settings_path, codex_auth_path,
    codex_config_path, codex_permission_state_path, gemini_env_backup_path, gemini_env_path,
    gemini_settings_backup_path, gemini_settings_path, opencode_config_path,
    opencode_managed_state_path,
};

pub(super) fn should_restore_standard_relay_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.api_key.trim().is_empty()
}
