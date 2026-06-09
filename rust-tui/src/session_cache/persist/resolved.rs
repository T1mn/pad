use super::super::bindings::upsert_binding;
use super::super::model::HookBindingContext;
use super::super::storage::{load_index, prune_index, save_index};
use super::super::turns::normalize_turns;
use super::super::util::now_ts;
use super::record::upsert_session_record;
use crate::model::{AgentPanel, AgentType, PreviewTurn, SessionCacheState};
use crate::session_continuity::ContinuityWriteSource;
use std::io;
use std::path::Path;

pub fn persist_resolved_session(
    panel: &AgentPanel,
    transcript_path: &Path,
    turns: &[PreviewTurn],
) -> io::Result<()> {
    let Some(agent_session_id) = panel.agent_session_id.as_ref() else {
        return Ok(());
    };

    let normalized_turns = normalize_turns(turns, panel.agent_type == AgentType::Codex);
    let transcript = transcript_path.to_string_lossy().to_string();
    if panel.session_cache_state == Some(SessionCacheState::Confirmed)
        && panel.transcript_path.as_deref() == Some(transcript.as_str())
        && panel.cached_preview_turns.as_ref() == normalized_turns.as_slice()
    {
        return Ok(());
    }

    let mut index = load_index();
    let _ = prune_index(&mut index);
    let now = now_ts();
    let agent_type = panel.agent_type.as_str();
    let record_idx = upsert_session_record(&mut index, agent_session_id, agent_type, now);
    let last_user_prompt = normalized_turns.first().map(|turn| turn.question.clone());
    let last_assistant_message = normalized_turns
        .first()
        .and_then(|turn| turn.answer.clone());
    let record = &mut index.sessions[record_idx];
    record.transcript_path = Some(transcript);
    record.recent_turns = normalized_turns;
    record.last_user_prompt = last_user_prompt;
    record.last_assistant_message = last_assistant_message;
    record.last_source = "resolver".to_string();
    record.last_seen_at = now;
    record.updated_at = now;

    upsert_binding(
        &mut index,
        panel,
        agent_session_id,
        HookBindingContext::default(),
        now,
    );
    save_index(&index)?;
    crate::session_continuity::record_cache_write(
        &panel.agent_type,
        agent_session_id,
        Some(transcript_path),
        ContinuityWriteSource::Resolver,
        index.sessions[record_idx].recent_turns.len(),
    );
    Ok(())
}
