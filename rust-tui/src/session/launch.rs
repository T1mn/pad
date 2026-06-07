use std::process::Command;

pub(super) fn should_launch_after_attach(agent_cmd: &str) -> bool {
    matches!(agent_cmd.trim(), "gemini" | "gemini-cli")
}

pub(super) fn launch_agent_after_attach(target_pane: &str, agent_cmd: &str) {
    let escaped_agent = super::shell::shell_single_quote(agent_cmd);
    let escaped_pane = super::shell::shell_single_quote(target_pane);
    let script = format!(
        "sleep 0.2; tmux send-keys -t {pane} C-c; tmux send-keys -t {pane} 'clear' Enter; tmux send-keys -t {pane} {agent} Enter",
        pane = escaped_pane,
        agent = escaped_agent
    );
    let result = Command::new("tmux")
        .args(["run-shell", "-b", &script])
        .output();
    log_debug!(
        "session: delayed launch target_pane={} cmd={} result={:?}",
        target_pane,
        agent_cmd,
        result.map(|o| o.status)
    );
}
