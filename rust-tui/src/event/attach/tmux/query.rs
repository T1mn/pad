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

pub(in crate::event::attach) fn writable_client_for_pane(pane_id: &str) -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args([
            "list-clients",
            "-F",
            "#{client_name}\t#{pane_id}\t#{client_flags}",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_writable_client(&String::from_utf8_lossy(&output.stdout), pane_id)
}

fn parse_writable_client(output: &str, pane_id: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut fields = line.splitn(3, '\t');
        let name = fields.next()?;
        let active_pane = fields.next()?;
        let flags = fields.next()?;
        let read_only = flags.split(',').any(|flag| flag == "read-only");
        (active_pane == pane_id && !read_only && !name.is_empty()).then(|| name.to_string())
    })
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

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
