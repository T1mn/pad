mod bindings {
    pub(super) use crate::tmux_bindings::{
        current_root_binding, pad_sider_toggle_command, restore_binding_cmd,
        return_binding_command, PAD_SIDER_TOGGLE_KEYS,
    };
}
mod create;
mod launch {
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
}
mod pad_context {
    use std::process::Command;

    pub(super) struct PadContext {
        pub(super) pane: Option<String>,
        pub(super) window: Option<String>,
        pub(super) session: Option<String>,
    }

    pub(super) fn resolve_pad_context() -> PadContext {
        let pane = std::env::var("TMUX_PANE").ok();
        let window = pane
            .as_deref()
            .and_then(|pane_id| tmux_display_unchecked(pane_id, "#{session_name}:#{window_index}"));
        let session = pane
            .as_deref()
            .and_then(|pane_id| tmux_display(pane_id, "#{session_name}"));
        PadContext {
            pane,
            window,
            session,
        }
    }

    fn tmux_display(pane_id: &str, format: &str) -> Option<String> {
        tmux_display_output(pane_id, format)
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn tmux_display_unchecked(pane_id: &str, format: &str) -> Option<String> {
        tmux_display_output(pane_id, format)
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn tmux_display_output(pane_id: &str, format: &str) -> Option<std::process::Output> {
        Command::new("tmux")
            .args(["display-message", "-p", "-t", pane_id, format])
            .output()
            .ok()
    }
}
mod return_bindings;
mod shell {
    pub(super) fn shell_single_quote(value: &str) -> String {
        crate::shell_quote::single_quote(value)
    }

    pub(super) fn wrap_tmux_run_shell(script: &str) -> String {
        format!("sh -lc {}", shell_single_quote(script))
    }

    pub(super) fn shell_trace_log_cmd(trace_id: &str, stage: &str, details: &str) -> String {
        let log_path = crate::paths::log_path().to_string_lossy().to_string();
        let message = format!("[handoff trace={}] stage={} {}", trace_id, stage, details);
        format!(
            "printf '[%s] %s\\n' \"$(date '+%H:%M:%S')\" {} >> {}",
            shell_single_quote(&message),
            shell_single_quote(&log_path)
        )
    }
}
mod status;
mod target;
mod tmux {
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
}

pub use create::create_session_with_agent;

#[cfg(test)]
mod tests;
