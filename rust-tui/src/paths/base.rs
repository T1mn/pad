use std::path::PathBuf;

pub fn pad_home_dir() -> PathBuf {
    resolve_pad_home_dir(
        std::env::var_os("PAD_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
        dirs::home_dir(),
    )
}

fn resolve_pad_home_dir(
    override_dir: Option<PathBuf>,
    environment_home: Option<PathBuf>,
    platform_home: Option<PathBuf>,
) -> PathBuf {
    override_dir
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            environment_home
                .or(platform_home)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".pad")
        })
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

pub fn terminal_workspace_path() -> PathBuf {
    pad_home_dir().join("terminal-workspace.json")
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

#[cfg(test)]
mod tests {
    use super::{resolve_pad_home_dir, terminal_workspace_path};
    use std::path::PathBuf;

    #[test]
    fn explicit_pad_home_is_used_without_rewriting_process_home() {
        assert_eq!(
            resolve_pad_home_dir(
                Some(PathBuf::from("/tmp/pad-isolated")),
                Some(PathBuf::from("/users/example")),
                None,
            ),
            PathBuf::from("/tmp/pad-isolated")
        );
        assert_eq!(
            resolve_pad_home_dir(None, Some(PathBuf::from("/users/example")), None),
            PathBuf::from("/users/example/.pad")
        );
    }

    #[test]
    fn terminal_workspace_lives_under_pad_home() {
        let path = terminal_workspace_path();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("terminal-workspace.json")
        );
    }
}
