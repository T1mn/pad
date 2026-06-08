use crate::app::App;
use crate::model::{PreviewSessionOrigin, PreviewSource, SessionCacheState};
use crate::preview_source::PreviewUpdate;

pub(super) fn sync_live_panel_from_preview_update(
    app: &mut App,
    update: &PreviewUpdate,
    previous_panel_cache_state: Option<SessionCacheState>,
) -> bool {
    let Some(panel) = update
        .live_pane_id
        .as_deref()
        .and_then(|pane_id| app.panels.iter_mut().find(|panel| panel.pane_id == pane_id))
    else {
        return false;
    };

    let should_persist_panel_session = update.session_origin != Some(PreviewSessionOrigin::App);
    if should_persist_panel_session {
        if let Some(transcript_path) = update.transcript_path.clone() {
            panel.transcript_path = Some(transcript_path);
        }
    }
    if app.preview.source == PreviewSource::Session
        && !update.turns.is_empty()
        && should_persist_panel_session
    {
        panel.cached_preview_turns = update.turns.clone();
        panel.last_user_prompt = update.turns.first().map(|turn| turn.question.clone());
        panel.last_assistant_message = update.turns.first().and_then(|turn| turn.answer.clone());
        if let Some(state) = update.session_cache_state {
            panel.session_cache_state = Some(state);
        }
    }
    if should_persist_panel_session {
        if let Some(session_id) = update.session_id.clone() {
            panel.agent_session_id = Some(session_id);
        }
        previous_panel_cache_state != panel.session_cache_state
    } else {
        false
    }
}
