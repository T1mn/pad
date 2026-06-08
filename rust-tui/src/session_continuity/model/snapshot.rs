use super::enums::{ContinuityAttemptClassification, ContinuityHealth};
use super::ledger::SessionContinuityRecord;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuitySnapshot {
    pub session_id: String,
    pub agent_type: Option<String>,
    pub transcript_path: Option<String>,
    pub last_hook_event: Option<String>,
    pub last_turn_id: Option<String>,
    pub last_hook_event_at: Option<i64>,
    pub last_prompt_submit_at: Option<i64>,
    pub last_stop_at: Option<i64>,
    pub last_assistant_message_at: Option<i64>,
    pub last_hook_cache_persist_at: Option<i64>,
    pub last_resolver_sync_at: Option<i64>,
    pub last_thread_updated_at: Option<i64>,
    pub last_rollout_seen_at: Option<i64>,
    pub last_rollout_mtime: Option<i64>,
    pub last_rollout_size: Option<u64>,
    pub lag_seconds: Option<i64>,
    pub stale_event_count: u32,
    pub bootstrap_event_count: u32,
    pub health: ContinuityHealth,
    pub attempt_classification: ContinuityAttemptClassification,
    pub updated_at: i64,
}

impl From<SessionContinuityRecord> for ContinuitySnapshot {
    fn from(record: SessionContinuityRecord) -> Self {
        Self {
            session_id: record.session_id,
            agent_type: record.agent_type,
            transcript_path: record.transcript_path,
            last_hook_event: record.last_hook_event,
            last_turn_id: record.last_turn_id,
            last_hook_event_at: record.last_hook_event_at,
            last_prompt_submit_at: record.last_prompt_submit_at,
            last_stop_at: record.last_stop_at,
            last_assistant_message_at: record.last_assistant_message_at,
            last_hook_cache_persist_at: record.last_hook_cache_persist_at,
            last_resolver_sync_at: record.last_resolver_sync_at,
            last_thread_updated_at: record.last_thread_updated_at,
            last_rollout_seen_at: record.last_rollout_seen_at,
            last_rollout_mtime: record.last_rollout_mtime,
            last_rollout_size: record.last_rollout_size,
            lag_seconds: record.lag_seconds,
            stale_event_count: record.stale_event_count,
            bootstrap_event_count: record.bootstrap_event_count,
            health: record.health,
            attempt_classification: record.attempt_classification,
            updated_at: record.updated_at,
        }
    }
}
