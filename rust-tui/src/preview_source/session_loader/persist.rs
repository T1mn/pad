use crate::model::{PreviewSessionOrigin, PreviewTurn};
use crate::preview_source::session_target::{self, SessionTarget};
use crate::preview_source::PreviewRequest;
use std::path::Path;

pub(super) fn persist_resolved_session_if_needed(
    request: &PreviewRequest,
    target: &SessionTarget,
    transcript_path: &Path,
    turns: &[PreviewTurn],
) {
    if target.origin != PreviewSessionOrigin::Pane || !request.persist_resolved_session {
        return;
    }

    let Some(panel) = session_target::persistence_panel_from_request(request, target) else {
        return;
    };

    if let Err(err) = crate::session_cache::persist_resolved_session(&panel, transcript_path, turns)
    {
        log_debug!("session_cache: persist resolved failed: {}", err);
    }
}
