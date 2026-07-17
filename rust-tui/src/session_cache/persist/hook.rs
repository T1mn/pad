use super::super::bindings::upsert_binding;
use super::super::model::{snapshot_from_record, HookBindingContext, SessionCacheSnapshot};
use super::super::storage::{load_index, prune_index, save_index};
use super::super::turns::{merge_recent_turns, normalize_cached_codex_prompt};
use super::super::util::{clean_text, first_non_empty_str, now_ts, prefer_non_empty_str};
use super::record::upsert_session_record;
use crate::hook::HookEvent;
use crate::model::{AgentPanel, AgentType, SessionCacheState};
use crate::session_continuity::ContinuityWriteSource;
use std::io;
use std::path::Path;

pub fn persist_hook_event(
    panel: &AgentPanel,
    event: &HookEvent,
) -> io::Result<Option<SessionCacheSnapshot>> {
    let Some(agent_session_id) = event
        .session_id
        .as_deref()
        .or(panel.agent_session_id.as_deref())
    else {
        return Ok(None);
    };

    let mut index = load_index();
    let _ = prune_index(&mut index);
    let now = now_ts();
    let agent_type = panel.agent_type.as_str();
    let normalize_codex = panel.agent_type == AgentType::Codex;
    let panel_fallback_allowed = match (
        event.session_id.as_deref(),
        panel.agent_session_id.as_deref(),
    ) {
        (Some(event_id), Some(panel_id)) => event_id == panel_id,
        _ => true,
    };
    let panel_transcript = panel_fallback_allowed
        .then_some(panel.transcript_path.as_deref())
        .flatten();
    let panel_prompt = panel_fallback_allowed
        .then_some(panel.last_user_prompt.as_deref())
        .flatten();
    let panel_assistant = panel_fallback_allowed
        .then_some(panel.last_assistant_message.as_deref())
        .flatten();

    let record_idx = upsert_session_record(&mut index, agent_session_id, agent_type, now);
    index.sessions[record_idx].transcript_path = prefer_non_empty_str([
        event.transcript_path.as_deref(),
        panel_transcript,
        index.sessions[record_idx].transcript_path.as_deref(),
    ]);
    if index.sessions[record_idx].last_source != "hook" {
        index.sessions[record_idx].last_source = "hook".to_string();
    }
    index.sessions[record_idx].last_seen_at = now;
    index.sessions[record_idx].updated_at = now;

    let prompt = normalize_cached_codex_prompt(
        first_non_empty_str([event.prompt.as_deref(), panel_prompt]),
        normalize_codex,
    );
    let assistant = match event.event.as_str() {
        "user_prompt_submit" => clean_text(event.last_assistant_message.as_deref()),
        _ => clean_text(event.last_assistant_message.as_deref())
            .or_else(|| clean_text(panel_assistant)),
    };

    merge_recent_turns(
        &mut index.sessions[record_idx].recent_turns,
        prompt.as_deref(),
        assistant.as_deref(),
        normalize_cached_codex_prompt(panel_prompt, normalize_codex).as_deref(),
    );

    let (last_user_prompt, last_assistant_message) =
        if let Some(first) = index.sessions[record_idx].recent_turns.first() {
            (Some(first.question.clone()), first.answer.clone())
        } else {
            (prompt, assistant)
        };
    index.sessions[record_idx].last_user_prompt = last_user_prompt;
    index.sessions[record_idx].last_assistant_message = last_assistant_message;

    upsert_binding(
        &mut index,
        panel,
        agent_session_id,
        HookBindingContext::from_event(event),
        now,
    );
    save_index(&index)?;

    crate::session_continuity::record_cache_write(
        &panel.agent_type,
        agent_session_id,
        index.sessions[record_idx]
            .transcript_path
            .as_deref()
            .map(Path::new),
        ContinuityWriteSource::Hook,
        index.sessions[record_idx].recent_turns.len(),
    );

    Ok(Some(snapshot_from_record(
        &index.sessions[record_idx],
        SessionCacheState::Confirmed,
    )))
}
