use crate::hook::HookEvent;
use crate::model::{AgentPanel, AgentType};

#[derive(Clone, Debug)]
pub(super) struct PendingCodexTitleSummary {
    pub(super) session_id: String,
    pub(super) turns: Vec<crate::model::PreviewTurn>,
    pub(super) turn_count: usize,
}

pub(super) fn codex_title_summary_request_for_panel(
    codex_config: &crate::theme::CodexConfig,
    panel: &AgentPanel,
    event: &HookEvent,
    persisted_snapshot: Option<&crate::session_cache::SessionCacheSnapshot>,
) -> Option<PendingCodexTitleSummary> {
    if !crate::title_summary::is_enabled(codex_config)
        || event.event != "stop"
        || panel.agent_type != AgentType::Codex
    {
        return None;
    }

    let session_id = panel
        .agent_session_id
        .clone()
        .or_else(|| persisted_snapshot.map(|snapshot| snapshot.agent_session_id.clone()))
        .or_else(|| event.session_id.clone())?;

    let turns = persisted_snapshot
        .map(|snapshot| snapshot.recent_turns.to_vec())
        .unwrap_or_else(|| panel.cached_preview_turns.to_vec());
    let turn_count = turns
        .iter()
        .filter(|turn| !turn.question.trim().is_empty())
        .count();

    Some(PendingCodexTitleSummary {
        session_id,
        turns,
        turn_count,
    })
}
