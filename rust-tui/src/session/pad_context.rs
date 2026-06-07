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
