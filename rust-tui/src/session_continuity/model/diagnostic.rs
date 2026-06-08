use super::enums::{ContinuityAttemptClassification, ContinuityHealth};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(in crate::session_continuity) struct ContinuityDiagnosticEvent {
    pub(in crate::session_continuity) ts: i64,
    pub(in crate::session_continuity) kind: &'static str,
    pub(in crate::session_continuity) session_id: String,
    pub(in crate::session_continuity) agent_type: Option<String>,
    pub(in crate::session_continuity) event: Option<String>,
    pub(in crate::session_continuity) turn_id: Option<String>,
    pub(in crate::session_continuity) transcript_path: Option<String>,
    pub(in crate::session_continuity) source: Option<&'static str>,
    pub(in crate::session_continuity) health: ContinuityHealth,
    pub(in crate::session_continuity) attempt_classification: ContinuityAttemptClassification,
    pub(in crate::session_continuity) lag_seconds: Option<i64>,
    pub(in crate::session_continuity) stale_event_count: u32,
    pub(in crate::session_continuity) rollout_mtime: Option<i64>,
    pub(in crate::session_continuity) rollout_size: Option<u64>,
    pub(in crate::session_continuity) thread_updated_at: Option<i64>,
    pub(in crate::session_continuity) cached_turns: Option<usize>,
    pub(in crate::session_continuity) transcript_turns: Option<usize>,
    pub(in crate::session_continuity) prefer_cache: Option<bool>,
    pub(in crate::session_continuity) reason: Option<&'static str>,
}
