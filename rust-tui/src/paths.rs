use std::fs;
use std::io;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

mod codex_home;
mod codex_hooks;
mod codex_wrapper;
mod hook_bridge;
mod prompts;
mod runtime_files;
mod sounds;

pub(crate) use codex_home::{
    canonical_codex_home_dir, ensure_pad_codex_home_layout, pad_codex_auth_path,
    pad_codex_config_path, pad_codex_home_dir, pad_codex_hooks_path,
};

#[allow(unused_imports)]
pub(crate) use prompts::{
    codex_index_prompt_file_path, codex_jailbreak_prompt_file_path,
    codex_selected_prompt_file_path, ensure_codex_index_prompt_file_seeded,
    ensure_codex_jailbreak_prompt_file_seeded, write_codex_selected_prompt_file,
    DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE, DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE,
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

pub fn pad_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pad")
}

pub fn config_path() -> PathBuf {
    pad_home_dir().join("config.toml")
}

pub fn relay_export_path() -> PathBuf {
    pad_home_dir().join("relay.yaml")
}

pub fn opencode_exports_dir() -> PathBuf {
    pad_home_dir().join("opencode-exports")
}

pub fn opencode_stats_dir() -> PathBuf {
    pad_home_dir().join("opencode-stats")
}

pub fn opencode_diagnostics_dir() -> PathBuf {
    pad_home_dir().join("opencode-diagnostics")
}

pub fn workspace_recipes_path() -> PathBuf {
    pad_home_dir().join("workspace-recipes.toml")
}

pub fn pad_db_path() -> PathBuf {
    pad_home_dir().join("pad.db")
}

pub fn legacy_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        })
        .join("pad")
        .join("config.toml")
}

pub fn logs_dir() -> PathBuf {
    pad_home_dir().join("logs")
}

pub fn log_path() -> PathBuf {
    logs_dir().join("pad.log")
}

pub fn telegram_bot_log_path() -> PathBuf {
    logs_dir().join("telegram-bot.log")
}

pub fn hook_events_path() -> PathBuf {
    logs_dir().join("hook-events.jsonl")
}

pub fn notifications_dir() -> PathBuf {
    pad_home_dir().join("notifications")
}

pub fn notification_inbox_path() -> PathBuf {
    notifications_dir().join("inbox.json")
}

pub fn session_continuity_log_path() -> PathBuf {
    logs_dir().join("session-continuity.jsonl")
}

pub fn scripts_dir() -> PathBuf {
    pad_home_dir().join("scripts")
}

pub fn prompts_dir() -> PathBuf {
    pad_home_dir().join("prompt")
}

pub fn sessions_dir() -> PathBuf {
    pad_home_dir().join("sessions")
}

pub fn sessions_index_path() -> PathBuf {
    sessions_dir().join("index.json")
}

pub fn session_continuity_state_path() -> PathBuf {
    sessions_dir().join("continuity.json")
}

pub fn claude_hook_bridge_path() -> PathBuf {
    scripts_dir().join("claude_hook_bridge.py")
}

pub fn codex_hook_bridge_path() -> PathBuf {
    scripts_dir().join("codex_hook_bridge.py")
}

pub fn pad_codex_wrapper_path() -> PathBuf {
    scripts_dir().join("pad-codex")
}

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
    ensure_codex_jailbreak_prompt_file_seeded()?;
    ensure_codex_index_prompt_file_seeded()?;
    ensure_pad_codex_home_layout()?;
    if !hook_events_path().exists() {
        fs::write(hook_events_path(), "")?;
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
