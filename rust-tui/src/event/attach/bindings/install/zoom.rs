use crate::app::App;

pub(super) struct ZoomDecision {
    pub(super) already_zoomed: bool,
    pub(super) pane_count: usize,
    pub(super) should_zoom: bool,
    pub(super) restore_zoom_cmd: String,
}

impl ZoomDecision {
    pub(super) fn for_target(app: &App, target_pane_id: &str) -> Self {
        let zoom_info = std::process::Command::new("tmux")
            .args([
                "display-message",
                "-t",
                target_pane_id,
                "-p",
                "#{window_zoomed_flag} #{window_panes}",
            ])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_default();

        let mut parts = zoom_info.split_whitespace();
        let already_zoomed = parts.next().unwrap_or("0") == "1";
        let pane_count = parts.next().unwrap_or("1").parse().unwrap_or(1);
        let want_zoom = app.config.desired_agent_style.zoom == "auto";
        let should_zoom = want_zoom && pane_count > 1 && !already_zoomed;
        let restore_zoom_cmd = if should_zoom {
            // Do NOT zoom here — zoom happens after select-pane so user sees it instantly.
            format!("tmux resize-pane -Z -t '{}'", target_pane_id)
        } else {
            String::new()
        };

        Self {
            already_zoomed,
            pane_count,
            should_zoom,
            restore_zoom_cmd,
        }
    }
}
