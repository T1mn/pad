use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod hook_bridge;

const CODEX_JAILBREAK_PROMPT_VERSION: &str = "codex-jailbreak-prompt-2026-04-26.1";
const CODEX_INDEX_PROMPT_VERSION: &str = "codex-index-prompt-2026-04-26.1";
const LEGACY_CODEX_JAILBREAK_PROMPT_HASHES: &[&str] = &["c8bf76a53a9b840d52c987ebff0310b2"];
pub const DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE: &str =
    include_str!("../assets/prompts/codex_jailbreak.md");
pub const DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE: &str =
    include_str!("../assets/prompts/codex_index.md");

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedPromptState {
    version: String,
    content_md5: String,
}

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

pub fn session_continuity_log_path() -> PathBuf {
    logs_dir().join("session-continuity.jsonl")
}

pub fn scripts_dir() -> PathBuf {
    pad_home_dir().join("scripts")
}

pub fn prompts_dir() -> PathBuf {
    pad_home_dir().join("prompt")
}

pub fn codex_jailbreak_prompt_file_path() -> PathBuf {
    prompts_dir().join("codex_jailbreak.md")
}

pub fn codex_index_prompt_file_path() -> PathBuf {
    prompts_dir().join("codex_index.md")
}

pub fn codex_selected_prompt_file_path() -> PathBuf {
    prompts_dir().join("codex_selected.md")
}

fn legacy_codex_prompt_file_path() -> PathBuf {
    prompts_dir().join("codex.md")
}

fn codex_jailbreak_prompt_state_path() -> PathBuf {
    prompts_dir().join("codex_jailbreak.version")
}

fn codex_index_prompt_state_path() -> PathBuf {
    prompts_dir().join("codex_index.version")
}

pub fn ensure_codex_jailbreak_prompt_file_seeded() -> io::Result<()> {
    fs::create_dir_all(prompts_dir())?;
    let prompt_path = codex_jailbreak_prompt_file_path();
    let state_path = codex_jailbreak_prompt_state_path();
    let existing_prompt = match fs::read_to_string(&prompt_path) {
        Ok(existing) => Some(existing),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            match fs::read_to_string(legacy_codex_prompt_file_path()) {
                Ok(existing) => Some(existing),
                Err(err) if err.kind() == io::ErrorKind::NotFound => None,
                Err(err) => return Err(err),
            }
        }
        Err(err) => return Err(err),
    };
    let current_state = ManagedPromptState {
        version: CODEX_JAILBREAK_PROMPT_VERSION.to_string(),
        content_md5: prompt_md5(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE),
    };
    let existing_state = read_managed_prompt_state(&state_path)?;

    let needs_seed = match existing_prompt.as_deref() {
        None => true,
        Some(existing) if existing.trim().is_empty() => true,
        Some(existing) => {
            should_refresh_managed_prompt(existing, existing_state.as_ref(), &current_state)
        }
    };

    if needs_seed {
        fs::write(prompt_path, DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE)?;
        write_managed_prompt_state(&state_path, &current_state)?;
    } else if !prompt_path.exists() {
        if let Some(existing) = existing_prompt {
            fs::write(prompt_path, existing)?;
        }
    }

    Ok(())
}

pub fn ensure_codex_index_prompt_file_seeded() -> io::Result<()> {
    fs::create_dir_all(prompts_dir())?;
    let prompt_path = codex_index_prompt_file_path();
    let state_path = codex_index_prompt_state_path();
    let existing_prompt = match fs::read_to_string(&prompt_path) {
        Ok(existing) => Some(existing),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    let current_state = ManagedPromptState {
        version: CODEX_INDEX_PROMPT_VERSION.to_string(),
        content_md5: prompt_md5(DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE),
    };
    let existing_state = read_managed_prompt_state(&state_path)?;

    let needs_seed = match existing_prompt.as_deref() {
        None => true,
        Some(existing) if existing.trim().is_empty() => true,
        Some(existing) => {
            should_refresh_managed_prompt(existing, existing_state.as_ref(), &current_state)
        }
    };

    if needs_seed {
        fs::write(prompt_path, DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE)?;
        write_managed_prompt_state(&state_path, &current_state)?;
    }

    Ok(())
}

pub fn write_codex_selected_prompt_file(
    include_jailbreak: bool,
    include_index: bool,
) -> io::Result<Option<PathBuf>> {
    let mut prompt_paths = Vec::new();
    if include_jailbreak {
        ensure_codex_jailbreak_prompt_file_seeded()?;
        prompt_paths.push(codex_jailbreak_prompt_file_path());
    }
    if include_index {
        ensure_codex_index_prompt_file_seeded()?;
        prompt_paths.push(codex_index_prompt_file_path());
    }

    match prompt_paths.as_slice() {
        [] => Ok(None),
        [single] => Ok(Some(single.clone())),
        paths => {
            let mut content = String::from(
                "# Generated by pad from selected Codex prompt candidates. Do not edit directly.\n\n",
            );
            for path in paths {
                content.push_str(&format!("<!-- source: {} -->\n\n", path.display()));
                content.push_str(&fs::read_to_string(path)?);
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push('\n');
            }
            let combined_path = codex_selected_prompt_file_path();
            fs::write(&combined_path, content)?;
            Ok(Some(combined_path))
        }
    }
}

pub fn sounds_dir() -> PathBuf {
    pad_home_dir().join("sounds")
}

pub fn sound_file_path(preset_id: &str) -> PathBuf {
    sounds_dir().join(format!("{preset_id}.wav"))
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

pub fn hook_socket_path() -> PathBuf {
    pad_home_dir().join("pad-hook.sock")
}

pub fn pad_status_path() -> PathBuf {
    pad_home_dir().join("pad-status.json")
}

pub fn telegram_bot_status_path() -> PathBuf {
    pad_home_dir().join("telegram-bot-status.json")
}

pub fn telegram_state_path() -> PathBuf {
    pad_home_dir().join("telegram-state.json")
}

pub fn telegram_hook_socket_path() -> PathBuf {
    pad_home_dir().join("telegram-hook.sock")
}

pub fn ensure_runtime_layout() -> io::Result<()> {
    fs::create_dir_all(pad_home_dir())?;
    fs::create_dir_all(logs_dir())?;
    fs::create_dir_all(scripts_dir())?;
    fs::create_dir_all(prompts_dir())?;
    fs::create_dir_all(sessions_dir())?;
    ensure_codex_jailbreak_prompt_file_seeded()?;
    ensure_codex_index_prompt_file_seeded()?;
    if !hook_events_path().exists() {
        fs::write(hook_events_path(), "")?;
    }
    hook_bridge::install_bridge_scripts()?;
    crate::sound::ensure_runtime_assets()?;
    hook_bridge::ensure_codex_hook_support()?;
    crate::thread_meta::ensure_db()?;
    Ok(())
}

pub fn log_runtime_layout_status() {
    hook_bridge::log_bridge_statuses();
}

fn prompt_md5(content: &str) -> String {
    format!("{:x}", md5::compute(content))
}

fn should_refresh_managed_prompt(
    existing_prompt: &str,
    existing_state: Option<&ManagedPromptState>,
    current_state: &ManagedPromptState,
) -> bool {
    let existing_md5 = prompt_md5(existing_prompt);
    match existing_state {
        Some(state) => {
            existing_md5 == state.content_md5
                && (state.version != current_state.version
                    || state.content_md5 != current_state.content_md5)
        }
        None => {
            existing_md5 == current_state.content_md5
                || LEGACY_CODEX_JAILBREAK_PROMPT_HASHES.contains(&existing_md5.as_str())
        }
    }
}

fn read_managed_prompt_state(path: &Path) -> io::Result<Option<ManagedPromptState>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };

    let mut version = None;
    let mut content_md5 = None;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("version=") {
            version = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("content_md5=") {
            content_md5 = Some(value.trim().to_string());
        }
    }

    match (version, content_md5) {
        (Some(version), Some(content_md5)) if !version.is_empty() && !content_md5.is_empty() => {
            Ok(Some(ManagedPromptState {
                version,
                content_md5,
            }))
        }
        _ => Ok(None),
    }
}

fn write_managed_prompt_state(path: &Path, state: &ManagedPromptState) -> io::Result<()> {
    fs::write(
        path,
        format!(
            "version={}\ncontent_md5={}\n",
            state.version, state.content_md5
        ),
    )
}

#[cfg(test)]
use hook_bridge::{
    claude_hook_bridge_template, codex_hook_bridge_template, CLAUDE_BRIDGE_VERSION,
    CODEX_BRIDGE_VERSION,
};

#[cfg(test)]
mod paths_tests;
