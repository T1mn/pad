pub(in crate::event::attach) fn current_tmux_session() -> Option<String> {
    tmux_display_message(None, "#{session_name}")
}

pub(crate) fn current_tmux_pane_id() -> Option<String> {
    tmux_display_message(None, "#{pane_id}")
}

pub(in crate::event::attach) fn current_tmux_window_target() -> Option<String> {
    tmux_display_message(None, "#{session_name}:#{window_index}")
}

pub(in crate::event::attach) fn tmux_target_snapshot(target: &str) -> Option<String> {
    tmux_display_message(
        Some(target),
        "window=#{session_name}:#{window_index} pane=#{pane_id} active=#{pane_active} zoomed=#{window_zoomed_flag} layout=#{window_layout} visible=#{window_visible_layout}",
    )
}

fn tmux_display_message(target: Option<&str>, format: &str) -> Option<String> {
    let mut command = std::process::Command::new("tmux");
    command.args(["display-message"]);
    if let Some(target) = target {
        command.args(["-t", target]);
    }
    command.args(["-p", format]);

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
