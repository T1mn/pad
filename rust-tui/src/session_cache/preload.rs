use super::bindings::find_snapshot_for_panel;
use super::model::{supports_cached_session, SessionCacheSnapshot};
use super::storage::{load_index, prune_index, save_index};
use crate::model::{
    AgentPanel, AgentState, AgentStateSource, AgentType, PreviewTurn, SharedPreviewTurns,
};

pub fn preload_panels(panels: &mut [AgentPanel]) -> std::io::Result<()> {
    if !panels.iter().any(panel_needs_preload) {
        return Ok(());
    }

    let mut index = load_index();
    let changed = prune_index(&mut index);

    for panel in panels.iter_mut() {
        if !panel_needs_preload(panel) {
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

pub(super) fn panel_needs_preload(panel: &AgentPanel) -> bool {
    supports_cached_session(panel)
        && panel.agent_session_id.is_none()
        && panel.transcript_path.is_none()
        && panel.cached_preview_turns.is_empty()
}

pub(super) fn apply_snapshot_to_panel(panel: &mut AgentPanel, snapshot: &SessionCacheSnapshot) {
    let recent_turns = normalize_snapshot_turns(&snapshot.recent_turns, &panel.agent_type);
    let last_user_prompt =
        normalize_snapshot_prompt(snapshot.last_user_prompt.as_deref(), &panel.agent_type);
    let is_busy = latest_turn_missing_answer(recent_turns.as_ref());
    panel.agent_session_id = Some(snapshot.agent_session_id.clone());
    panel.transcript_path = snapshot.transcript_path.clone();
    panel.cached_preview_turns = recent_turns;
    panel.last_user_prompt = last_user_prompt;
    panel.last_assistant_message = snapshot.last_assistant_message.clone();
    panel.session_cache_state = Some(snapshot.state);

    if is_busy {
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

fn normalize_snapshot_turns(
    turns: &SharedPreviewTurns,
    agent_type: &AgentType,
) -> SharedPreviewTurns {
    if *agent_type != AgentType::Codex {
        return turns.clone();
    }

    turns
        .iter()
        .cloned()
        .filter_map(|mut turn| {
            turn.question =
                crate::preview_source::codex::normalize_codex_user_text(&turn.question, None);
            if turn.question.is_empty() {
                None
            } else {
                Some(turn)
            }
        })
        .collect::<Vec<_>>()
        .into()
}

fn normalize_snapshot_prompt(value: Option<&str>, agent_type: &AgentType) -> Option<String> {
    let text = value?.trim();
    if text.is_empty() {
        return None;
    }

    if *agent_type == AgentType::Codex {
        let normalized = crate::preview_source::codex::normalize_codex_user_text(text, None);
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    } else {
        Some(text.to_string())
    }
}
