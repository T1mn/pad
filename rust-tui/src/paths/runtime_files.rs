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
