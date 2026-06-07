use std::process::Command;

pub(super) fn current_tmux_client_snapshot() -> Option<String> {
    Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "client=#{client_tty} session=#{session_name} window=#{window_index} pane=#{pane_id}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}
