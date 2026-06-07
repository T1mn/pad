pub(super) fn build_codex_restart_command(
    agent_cmd: &str,
    cwd: &str,
    session_id: Option<&str>,
) -> String {
    let agent_cmd = agent_cmd.trim();
    let agent_cmd = if agent_cmd.is_empty() {
        "codex"
    } else {
        agent_cmd
    };
    let session_part = session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| shell_single_quote(id.trim()))
        .unwrap_or_else(|| "--last".to_string());

    format!(
        "exec {} -C {} resume {}",
        crate::codex_runtime::with_pad_codex_runtime(agent_cmd),
        shell_single_quote(cwd),
        session_part
    )
}

fn shell_single_quote(value: &str) -> String {
    crate::codex_runtime::shell_single_quote(value)
}
