use crate::model::{AgentPanel, AgentType};
use std::path::Path;

use super::super::super::display::clean_title;

pub(super) fn resolve_live_panel_upstream_title(panel: &AgentPanel) -> Option<String> {
    let session_id = panel.agent_session_id.as_deref()?;
    match panel.agent_type {
        AgentType::Codex => crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.as_deref().and_then(clean_title)),
        AgentType::Claude => crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.as_deref().and_then(clean_title)),
        AgentType::Gemini => crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.as_deref().and_then(clean_title)),
        _ => None,
    }
}

pub(super) fn resolve_live_panel_subtitle(panel: &AgentPanel) -> Option<String> {
    panel.last_user_prompt.clone().or_else(|| {
        panel
            .cached_preview_turns
            .first()
            .map(|turn| turn.question.clone())
    })
}

pub(super) fn resolve_live_panel_session_provider_name(panel: &AgentPanel) -> Option<String> {
    if let Some(path) = panel.transcript_path.as_deref() {
        return crate::sidebar::provider::resolve_session_provider_name(
            &panel.agent_type,
            Some(Path::new(path)),
        );
    }

    if panel.agent_type == AgentType::Codex {
        let session_id = panel.agent_session_id.as_deref()?;
        let thread = crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()?;
        return crate::sidebar::provider::resolve_session_provider_name(
            &panel.agent_type,
            Some(thread.rollout_path.as_path()),
        );
    }

    None
}
