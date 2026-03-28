use super::bindings::find_snapshot_for_panel;
use super::model::{supports_cached_session, SessionCacheSnapshot};
use super::storage::{load_index, prune_index, save_index};
use crate::model::{AgentPanel, AgentState, AgentStateSource, PreviewTurn};

pub fn preload_panels(panels: &mut [AgentPanel]) -> std::io::Result<()> {
    let mut index = load_index();
    let changed = prune_index(&mut index);

    for panel in panels.iter_mut() {
        if !supports_cached_session(panel) {
            continue;
        }
        if panel.agent_session_id.is_some()
            || panel.transcript_path.is_some()
            || !panel.cached_preview_turns.is_empty()
        {
            continue;
        }

        if let Some(snapshot) = find_snapshot_for_panel(&index, panel) {
            apply_snapshot_to_panel(panel, &snapshot);
        }
    }

    if changed {
        save_index(&index)?;
    }

    Ok(())
}

pub(super) fn apply_snapshot_to_panel(panel: &mut AgentPanel, snapshot: &SessionCacheSnapshot) {
    panel.agent_session_id = Some(snapshot.agent_session_id.clone());
    panel.transcript_path = snapshot.transcript_path.clone();
    panel.cached_preview_turns = snapshot.recent_turns.clone();
    panel.last_user_prompt = snapshot.last_user_prompt.clone();
    panel.last_assistant_message = snapshot.last_assistant_message.clone();
    panel.session_cache_state = Some(snapshot.state);

    if latest_turn_missing_answer(&snapshot.recent_turns) {
        panel.state = AgentState::Busy;
        panel.state_source = AgentStateSource::Hook;
        panel.is_active = true;
    }
}

pub(super) fn latest_turn_missing_answer(turns: &[PreviewTurn]) -> bool {
    turns
        .first()
        .map(|turn| {
            !turn.question.trim().is_empty()
                && turn
                    .answer
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
        })
        .unwrap_or(false)
}
