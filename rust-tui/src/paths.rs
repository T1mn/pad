use std::fs;
use std::io;

mod base;
mod claude {
    use std::path::PathBuf;

    pub fn claude_config_dir() -> PathBuf {
        std::env::var_os("CLAUDE_CONFIG_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".claude"))
    }

    pub fn claude_projects_dir() -> PathBuf {
        claude_config_dir().join("projects")
    }

    pub fn claude_settings_path() -> PathBuf {
        claude_config_dir().join("settings.json")
    }

    fn home_dir() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}
mod codex_home;
mod codex_hooks;
mod codex_wrapper;
mod hook_bridge;
mod prompts;
mod runtime_files {
    use std::path::PathBuf;

    pub fn hook_socket_path() -> PathBuf {
        super::pad_home_dir().join("pad-hook.sock")
    }

    pub fn api_socket_path() -> PathBuf {
        super::pad_home_dir().join("pad-api.sock")
    }

    pub fn pad_status_path() -> PathBuf {
        super::pad_home_dir().join("pad-status.json")
    }

    pub fn telegram_bot_status_path() -> PathBuf {
        super::pad_home_dir().join("telegram-bot-status.json")
    }

    pub fn telegram_state_path() -> PathBuf {
        super::pad_home_dir().join("telegram-state.json")
    }

    pub fn telegram_hook_socket_path() -> PathBuf {
        super::pad_home_dir().join("telegram-hook.sock")
    }
}
mod sounds {
    use std::path::PathBuf;

    pub fn sounds_dir() -> PathBuf {
        super::pad_home_dir().join("sounds")
    }

    pub fn sound_file_path(preset_id: &str) -> PathBuf {
        sounds_dir().join(format!("{preset_id}.wav"))
    }
}

#[cfg(test)]
pub(crate) use claude::claude_config_dir;
pub(crate) use claude::{claude_projects_dir, claude_settings_path};
pub(crate) use codex_home::{
    canonical_codex_home_dir, ensure_pad_codex_home_layout, pad_codex_auth_path,
    pad_codex_config_path, pad_codex_home_dir, pad_codex_hooks_path,
};

pub(crate) use prompts::write_codex_selected_prompt_file;

#[cfg(test)]
pub(crate) use prompts::{
    codex_index_prompt_file_path, codex_jailbreak_prompt_file_path,
    codex_selected_prompt_file_path, DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE,
    DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE,
};
pub(crate) use runtime_files::{
    api_socket_path, hook_socket_path, pad_status_path, telegram_bot_status_path,
    telegram_hook_socket_path, telegram_state_path,
};
pub(crate) use sounds::{sound_file_path, sounds_dir};

#[cfg(test)]
use prompts::{
    codex_jailbreak_prompt_state_path, legacy_codex_prompt_file_path, prompt_md5,
    read_managed_prompt_state, write_managed_prompt_state, ManagedPromptState,
    CODEX_JAILBREAK_PROMPT_VERSION,
};

pub use base::{
    claude_hook_bridge_path, codex_hook_bridge_path, config_path, hook_events_path,
    legacy_config_path, log_path, logs_dir, notification_inbox_path, notifications_dir,
    opencode_diagnostics_dir, opencode_exports_dir, opencode_stats_dir, pad_codex_wrapper_path,
    pad_db_path, pad_home_dir, prompts_dir, relay_export_path, scripts_dir,
    session_continuity_log_path, session_continuity_state_path, sessions_dir, sessions_index_path,
    telegram_bot_log_path, terminal_workspace_path,
};

pub fn ensure_pad_codex_wrapper() -> io::Result<()> {
    fs::create_dir_all(scripts_dir())?;
    codex_wrapper::install_pad_codex_wrapper()
}

pub fn ensure_runtime_layout() -> io::Result<()> {
    fs::create_dir_all(pad_home_dir())?;
    fs::create_dir_all(logs_dir())?;
    fs::create_dir_all(notifications_dir())?;
    fs::create_dir_all(scripts_dir())?;
    fs::create_dir_all(prompts_dir())?;
    fs::create_dir_all(sessions_dir())?;
    prompts::ensure_codex_jailbreak_prompt_file_seeded()?;
    prompts::ensure_codex_index_prompt_file_seeded()?;
    ensure_pad_codex_home_layout()?;
    let hook_events = hook_events_path();
    if hook_events.exists() {
        crate::atomic_file::ensure_private(&hook_events)?;
    } else {
        crate::atomic_file::write_private(&hook_events, "")?;
    }
    hook_bridge::install_bridge_scripts()?;
    ensure_pad_codex_wrapper()?;
    crate::sound::ensure_runtime_assets()?;
    codex_hooks::ensure_codex_hook_support()?;
    crate::thread_meta::ensure_db()?;
    Ok(())
}

pub fn log_runtime_layout_status() {
    hook_bridge::log_bridge_statuses();
}

#[cfg(test)]
use codex_hooks::{
    test_codex_hooks_feature_key_for_version as codex_hooks_feature_key_for_version,
    test_parse_codex_cli_version as parse_codex_cli_version,
    test_remove_toml_key_in_section as remove_toml_key_in_section,
    test_set_toml_bool_in_section as set_toml_bool_in_section,
};
#[cfg(test)]
use hook_bridge::{
    claude_hook_bridge_template, codex_hook_bridge_template, CLAUDE_BRIDGE_VERSION,
    CODEX_BRIDGE_VERSION,
};

#[cfg(test)]
mod paths_tests;
