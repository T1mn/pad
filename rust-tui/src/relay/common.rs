mod files;
mod formats;
mod paths {
    use std::path::{Path, PathBuf};

    pub(in crate::relay) fn opencode_config_path() -> PathBuf {
        if let Some(path) = std::env::var_os("OPENCODE_CONFIG") {
            return PathBuf::from(path);
        }

        if let Some(dir) = std::env::var_os("OPENCODE_CONFIG_DIR") {
            let dir = PathBuf::from(dir);
            return existing_opencode_config_in(&dir).unwrap_or_else(|| dir.join("opencode.jsonc"));
        }

        let dir = home_dir().join(".config").join("opencode");
        existing_opencode_config_in(&dir).unwrap_or_else(|| dir.join("opencode.jsonc"))
    }

    fn existing_opencode_config_in(dir: &Path) -> Option<PathBuf> {
        let jsonc = dir.join("opencode.jsonc");
        if jsonc.exists() {
            return Some(jsonc);
        }
        let json = dir.join("opencode.json");
        json.exists().then_some(json)
    }

    pub(in crate::relay) fn opencode_managed_state_path() -> PathBuf {
        crate::paths::pad_home_dir().join("opencode-relay-state.json")
    }

    pub(in crate::relay) fn home_dir() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub(in crate::relay) fn codex_permission_state_path() -> PathBuf {
        crate::paths::pad_home_dir().join("codex-permissions-state.json")
    }

    pub(in crate::relay) fn claude_permission_state_path() -> PathBuf {
        crate::paths::pad_home_dir().join("claude-permissions-state.json")
    }

    pub(in crate::relay) fn claude_settings_path() -> PathBuf {
        crate::paths::claude_settings_path()
    }

    pub(in crate::relay) fn claude_backup_path() -> PathBuf {
        crate::paths::pad_home_dir().join("claude-settings.pre-pad.json")
    }

    pub(in crate::relay) fn gemini_settings_path() -> PathBuf {
        home_dir().join(".gemini").join("settings.json")
    }

    pub(in crate::relay) fn gemini_env_path() -> PathBuf {
        home_dir().join(".gemini").join(".env")
    }

    pub(in crate::relay) fn gemini_settings_backup_path() -> PathBuf {
        crate::paths::pad_home_dir().join("gemini-settings.pre-pad.json")
    }

    pub(in crate::relay) fn gemini_env_backup_path() -> PathBuf {
        crate::paths::pad_home_dir().join("gemini-env.pre-pad")
    }

    pub(in crate::relay) fn codex_config_path() -> PathBuf {
        crate::paths::pad_codex_config_path()
    }

    pub(in crate::relay) fn codex_backup_path() -> PathBuf {
        crate::paths::pad_home_dir().join("codex-config.pre-pad.toml")
    }

    pub(in crate::relay) fn codex_auth_path() -> PathBuf {
        crate::paths::pad_codex_auth_path()
    }

    pub(in crate::relay) fn codex_auth_backup_path() -> PathBuf {
        crate::paths::pad_home_dir().join("codex-auth.pre-pad.json")
    }
}

use crate::theme::AgentConfig;

pub(super) use files::{
    log_file_error, preserve_backup, restore_codex_auth, restore_codex_config, restore_file,
    write_text_file,
};
pub(super) use formats::{
    parse_env_file, parse_json_object, parse_json_object_strict, parse_toml_document,
    read_json_object_for_update, read_json_value, serialize_env_file, serialize_json_pretty,
    serialize_toml_document, write_json_value,
};
pub(super) use paths::{
    claude_backup_path, claude_permission_state_path, claude_settings_path, codex_auth_backup_path,
    codex_auth_path, codex_backup_path, codex_config_path, codex_permission_state_path,
    gemini_env_backup_path, gemini_env_path, gemini_settings_backup_path, gemini_settings_path,
    opencode_config_path, opencode_managed_state_path,
};

pub(super) fn should_restore_standard_relay_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.api_key.trim().is_empty()
}
