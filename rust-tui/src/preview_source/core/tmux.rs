use super::model::PreviewRequest;

const TMUX_CAPTURE_LINES: usize = 50;

pub(super) fn load_tmux_preview(request: &PreviewRequest) -> String {
    let Some(pane_id) = request.live_pane_id.as_deref() else {
        return String::from("No live pane available");
    };

    match crate::pty::capture_pane(pane_id, TMUX_CAPTURE_LINES) {
        Ok(content) => content,
        Err(_) => String::from("Failed to capture pane"),
    }
}
