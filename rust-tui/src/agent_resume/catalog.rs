use super::model::ResumeTarget;

pub fn list_resume_targets() -> Vec<ResumeTarget> {
    let mut targets: Vec<ResumeTarget> = crate::session_cache::list_cached_sessions()
        .into_iter()
        .filter(|session| !session.agent_session_id.trim().is_empty())
        .map(resume_target_from_cached_session)
        .collect();
    targets.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.agent_session_id.cmp(&right.agent_session_id))
    });
    targets
}

pub fn find_resume_target(session_id: &str) -> Option<ResumeTarget> {
    crate::session_cache::find_cached_session(session_id).map(resume_target_from_cached_session)
}

fn resume_target_from_cached_session(
    session: crate::session_cache::CachedSessionSummary,
) -> ResumeTarget {
    ResumeTarget {
        agent_session_id: session.agent_session_id,
        agent_type: session.agent_type,
        working_dir: session.working_dir.unwrap_or_else(|| ".".to_string()),
        transcript_path: session.transcript_path,
        title: session.last_user_prompt.or(session.last_assistant_message),
        updated_at: session.updated_at,
    }
}
