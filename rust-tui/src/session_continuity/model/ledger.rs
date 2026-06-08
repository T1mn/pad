use super::enums::{ContinuityAttemptClassification, ContinuityHealth};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(in crate::session_continuity) struct ContinuityLedger {
    pub(in crate::session_continuity) version: u32,
    pub(in crate::session_continuity) sessions: Vec<SessionContinuityRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::session_continuity) struct SessionContinuityRecord {
    pub(in crate::session_continuity) session_id: String,
    pub(in crate::session_continuity) agent_type: Option<String>,
    pub(in crate::session_continuity) transcript_path: Option<String>,
    pub(in crate::session_continuity) last_hook_event: Option<String>,
    pub(in crate::session_continuity) last_turn_id: Option<String>,
    pub(in crate::session_continuity) last_hook_event_at: Option<i64>,
    pub(in crate::session_continuity) last_prompt_submit_at: Option<i64>,
    pub(in crate::session_continuity) last_stop_at: Option<i64>,
    pub(in crate::session_continuity) last_assistant_message_at: Option<i64>,
    pub(in crate::session_continuity) last_hook_cache_persist_at: Option<i64>,
    pub(in crate::session_continuity) last_resolver_sync_at: Option<i64>,
    pub(in crate::session_continuity) last_thread_updated_at: Option<i64>,
    pub(in crate::session_continuity) last_rollout_seen_at: Option<i64>,
    pub(in crate::session_continuity) last_rollout_mtime: Option<i64>,
    pub(in crate::session_continuity) last_rollout_size: Option<u64>,
    pub(in crate::session_continuity) lag_seconds: Option<i64>,
    pub(in crate::session_continuity) stale_event_count: u32,
    pub(in crate::session_continuity) bootstrap_event_count: u32,
    pub(in crate::session_continuity) health: ContinuityHealth,
    pub(in crate::session_continuity) attempt_classification: ContinuityAttemptClassification,
    pub(in crate::session_continuity) updated_at: i64,
}

impl SessionContinuityRecord {
    pub(in crate::session_continuity) fn new(session_id: &str, now: i64) -> Self {
        Self {
            session_id: session_id.to_string(),
            agent_type: None,
            transcript_path: None,
            last_hook_event: None,
            last_turn_id: None,
            last_hook_event_at: None,
            last_prompt_submit_at: None,
            last_stop_at: None,
            last_assistant_message_at: None,
            last_hook_cache_persist_at: None,
            last_resolver_sync_at: None,
            last_thread_updated_at: None,
            last_rollout_seen_at: None,
            last_rollout_mtime: None,
            last_rollout_size: None,
            lag_seconds: None,
            stale_event_count: 0,
            bootstrap_event_count: 0,
            health: ContinuityHealth::Healthy,
            attempt_classification: ContinuityAttemptClassification::Normal,
            updated_at: now,
        }
    }
}
