use super::super::model::{CachedSessionRecord, SessionCacheIndex};

pub(in crate::session_cache) fn upsert_session_record(
    index: &mut SessionCacheIndex,
    agent_session_id: &str,
    agent_type: &str,
    now: i64,
) -> usize {
    if let Some(existing_idx) = index
        .sessions
        .iter()
        .position(|record| record.agent_session_id == agent_session_id)
    {
        index.sessions[existing_idx].agent_type = agent_type.to_string();
        index.sessions[existing_idx].last_seen_at = now;
        return existing_idx;
    }

    index.sessions.push(CachedSessionRecord {
        agent_session_id: agent_session_id.to_string(),
        agent_type: agent_type.to_string(),
        transcript_path: None,
        recent_turns: Vec::new(),
        last_user_prompt: None,
        last_assistant_message: None,
        last_seen_at: now,
        updated_at: now,
        last_source: "hook".to_string(),
    });
    index.sessions.len().saturating_sub(1)
}
