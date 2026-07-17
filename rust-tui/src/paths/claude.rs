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
